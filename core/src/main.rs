mod audio;
mod color;
mod effects;
mod engine;
mod frame;
mod guard;
mod hid;
mod input;
mod ipc;
mod layout;
mod math;
mod palette;
mod params;
mod settings;
mod tray;

use color::Col;
use frame::{Frame, FRAME_BYTES};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock};

/// Shared with the console ctrl handler for clean shutdown.
struct Cleanup {
    tx: Sender<engine::Cmd>,
    guard: Arc<Mutex<guard::Guard>>,
}

static CLEANUP: OnceLock<Cleanup> = OnceLock::new();

unsafe extern "system" fn ctrl_handler(_ctrl_type: u32) -> windows_sys::Win32::Foundation::BOOL {
    if let Some(c) = CLEANUP.get() {
        let _ = c.tx.send(engine::Cmd::Shutdown);
        // give the engine a beat to blank the board, then restore ASUS
        std::thread::sleep(std::time::Duration::from_millis(400));
        if let Ok(mut g) = c.guard.lock() {
            g.shutdown();
        }
    }
    std::process::exit(0);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--identify") => {
            if let Err(e) = hid::identify() {
                eprintln!("identify failed: {e}");
                std::process::exit(1);
            }
        }
        Some("--solid") => {
            let hexstr = args.get(1).map(String::as_str).unwrap_or("00E5A0");
            let hex = u32::from_str_radix(hexstr.trim_start_matches('#'), 16).unwrap_or(0x00E5A0);
            match hid::Keyboard::open() {
                Ok(mut kb) => {
                    kb.set_brightness(3).expect("set brightness");
                    let mut f = Frame::new();
                    f.clear(Col::hex(hex));
                    let mut bytes = [0u8; FRAME_BYTES];
                    f.to_bytes(1.0, 1.8, &mut bytes);
                    let sent = kb.send_frame(&bytes).expect("send frame");
                    println!("sent {sent} blocks of #{hexstr}");
                }
                Err(e) => {
                    eprintln!("open failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--version") => println!("keyscape-core {}", env!("CARGO_PKG_VERSION")),
        Some("--zone-test") => zone_test(),
        Some("--list") => {
            effects::register_python(effects::python::scan());
            for e in effects::registry() {
                println!("{:24} {:10} {}", e.id, e.category, e.name);
            }
            println!("{} effects", effects::registry().len());
        }
        _ => run(args),
    }
}

/// Interactive probe of the aux LED page: lights one aux index at a time so
/// you can watch which physical zone (logo / front bar / rear strip) each
/// index drives on this exact unit. Run with the core stopped.
fn zone_test() {
    let core_addr =
        std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, ipc::PORT));
    let core_was_running = std::net::TcpStream::connect_timeout(
        &core_addr,
        std::time::Duration::from_millis(300),
    )
    .is_ok();
    if core_was_running {
        println!("Stopping the running lighting core for the test...");
        if let Ok((mut ws, _)) =
            tungstenite::connect(format!("ws://127.0.0.1:{}", ipc::PORT))
        {
            let _ = ws.send(tungstenite::Message::Text(r#"{"op":"quit"}"#.into()));
        }
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(300));
            if std::net::TcpStream::connect_timeout(
                &core_addr,
                std::time::Duration::from_millis(200),
            )
            .is_err()
            {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let kb = match hid::Keyboard::open() {
        Ok(kb) => kb,
        Err(e) => {
            eprintln!("open failed: {e}");
            std::process::exit(1);
        }
    };
    let _ = kb.set_brightness(3);
    let _ = kb.set_zone_power_all();

    /// aux-page packet with exactly one of the 11 positional slots
    /// (indices 167..=177) lit white
    fn aux_single(slot: usize) -> [u8; 64] {
        let mut buf = [0u8; 64];
        buf[0] = 0x5D;
        buf[1] = 0xBC;
        buf[3] = 0x01;
        buf[4] = 0x04;
        buf[9 + slot * 3] = 255;
        buf[10 + slot * 3] = 255;
        buf[11 + slot * 3] = 255;
        buf
    }
    let stage = |name: &str, secs: u64| {
        println!("{name}");
        std::thread::sleep(std::time::Duration::from_secs(secs));
    };

    println!("Aux LED sweep — each step lights ONE aux index bright white for 4 s.");
    println!("Watch the LID LOGO, the FRONT under-edge bar, and the REAR strip, and note");
    println!("which index lights which zone.\n");
    stage("Baseline 4s: keyboard untouched, aux zones should be dark...", 4);

    for slot in 0..11usize {
        let idx = 167 + slot;
        let _ = kb.send_raw(&aux_single(slot));
        stage(&format!("INDEX {idx} is WHITE now — logo / front bar / rear / nothing?"), 4);
    }
    // clear the aux page
    let mut clear = [0u8; 64];
    clear[0] = 0x5D;
    clear[1] = 0xBC;
    clear[3] = 0x01;
    clear[4] = 0x04;
    let _ = kb.send_raw(&clear);

    // built-in static mode fallback: proves whether the rear strip listens
    // to firmware effects even if no direct index drives it
    let mut b3 = [0u8; 64];
    b3[0] = 0x5D;
    b3[1] = 0xB3;
    b3[2] = 0x00; // zone: whole device
    b3[3] = 0x00; // mode: static
    b3[4] = 255;
    b3[5] = 0;
    b3[6] = 255;
    let _ = kb.send_raw(&b3);
    let mut b5 = [0u8; 64];
    b5[0] = 0x5D;
    b5[1] = 0xB5;
    let _ = kb.send_raw(&b5);
    let mut b4 = [0u8; 64];
    b4[0] = 0x5D;
    b4[1] = 0xB4;
    let _ = kb.send_raw(&b4);
    stage("\nFINAL: MAGENTA via built-in static mode (whole board changes) — did the REAR strip turn magenta?", 6);

    println!("\nDone. Report which index lit each zone (e.g. \"logo=167, front bar=169-174,");
    println!("rear=176+177\") and whether the rear went magenta at the end.");

    if core_was_running {
        if let Ok(exe) = std::env::current_exe() {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new(exe)
                .arg("run")
                .creation_flags(0x0800_0000 | 0x0000_0200)
                .spawn();
            println!("Lighting core restarted.");
        }
    }
}

fn run(args: Vec<String>) {
    let layout = Arc::new(layout::Layout::load());
    // discover user Python effects before anything queries the registry
    effects::register_python(effects::python::scan());
    let mut settings = settings::Settings::load();
    // `run <effect_id>` overrides the persisted effect (handy for testing).
    if let Some(id) = args.get(1) {
        if effects::by_id(id).is_some() {
            settings.active_effect = id.clone();
        }
    }
    let audio_on = settings.audio.enabled;

    let (tx, rx) = mpsc::channel();

    let input_cap = Arc::new(Mutex::new(input::InputCapture::new(tx.clone(), layout.clone())));
    let audio_cap = Arc::new(Mutex::new(audio::AudioCapture::new(tx.clone())));
    let svc_guard = Arc::new(Mutex::new(guard::Guard::start(
        tx.clone(),
        settings.guard.manage_lighting_service,
        settings.guard.restore_on_exit,
    )));

    let hooks = engine::Hooks {
        set_input_capture: {
            let ic = input_cap.clone();
            Box::new(move |on| {
                if let Ok(mut ic) = ic.lock() {
                    ic.set_capture(on);
                }
            })
        },
        set_audio_capture: {
            let ac = audio_cap.clone();
            Box::new(move |on| {
                if let Ok(mut ac) = ac.lock() {
                    ac.set_capture(on);
                }
            })
        },
        set_guard_manage: {
            let g = svc_guard.clone();
            Box::new(move |on| {
                if let Ok(g) = g.lock() {
                    g.set_manage(on);
                }
            })
        },
    };

    let eng = engine::Engine::new(layout.clone(), settings, hooks);
    let engine_thread = std::thread::Builder::new()
        .name("engine".into())
        .spawn(move || eng.run(rx))
        .expect("spawn engine");

    // Music mode resumes only if the user had explicitly enabled it.
    if audio_on {
        audio_cap.lock().unwrap().set_capture(true);
    }

    let _ = CLEANUP.set(Cleanup { tx: tx.clone(), guard: svc_guard.clone() });
    unsafe {
        windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(ctrl_handler), 1);
    }

    // Tray icon: the daemon's only visible surface when the UI is closed.
    {
        let tray_tx = tx.clone();
        let quit_tx = tx.clone();
        let quit_guard = svc_guard.clone();
        std::thread::Builder::new()
            .name("tray".into())
            .spawn(move || {
                tray::run(
                    tray_tx,
                    Box::new(move || {
                        let _ = quit_tx.send(engine::Cmd::Shutdown);
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        if let Ok(mut g) = quit_guard.lock() {
                            g.shutdown();
                        }
                        std::process::exit(0);
                    }),
                );
            })
            .expect("spawn tray");
    }

    println!(
        "keyscape-core running: {} effects, LightingService {}",
        effects::registry().len(),
        if guard::is_running() { "RUNNING (keepalive mode until stopped)" } else { "not running" }
    );

    // The IPC accept loop owns the main thread from here.
    ipc::serve(tx.clone(), layout);
    let _ = engine_thread.join();
}
