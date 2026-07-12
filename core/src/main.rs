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
            for e in effects::registry() {
                println!("{:24} {:10} {}", e.id, e.category, e.name);
            }
            println!("{} effects", effects::registry().len());
        }
        _ => run(args),
    }
}

/// Interactive protocol probe for the rear light strip: each stage paints the
/// candidate encoding in a distinct color — whichever color shows up on the
/// physical strip identifies the real addressing. Run with the core stopped.
fn zone_test() {
    if std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, ipc::PORT)),
        std::time::Duration::from_millis(300),
    )
    .is_ok()
    {
        eprintln!("The lighting core is running and would overwrite the test.");
        eprintln!("Quit it first (tray icon -> Quit lighting core), then rerun.");
        std::process::exit(1);
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

    fn direct(bank: u8, start: u8, count: u8, rgb: (u8, u8, u8)) -> [u8; 64] {
        let mut buf = [0u8; 64];
        buf[0] = 0x5D;
        buf[1] = 0xBC;
        buf[3] = 0x01;
        buf[4] = bank;
        buf[5] = 0x01;
        buf[6] = start;
        buf[7] = count;
        for i in 0..count as usize {
            buf[9 + i * 3] = rgb.0;
            buf[10 + i * 3] = rgb.1;
            buf[11 + i * 3] = rgb.2;
        }
        buf
    }
    let stage = |name: &str, secs: u64| {
        println!("{name}");
        std::thread::sleep(std::time::Duration::from_secs(secs));
    };

    println!("Watch the REAR light strip (back edge, under the lid logo) and note every color it shows.\n");
    stage("Stage 0: baseline for 5s — strip should be dark/unchanged...", 5);

    for buf in [direct(0x04, 177, 16, (255, 0, 0)), direct(0x04, 193, 16, (255, 0, 0))] {
        let _ = kb.send_raw(&buf);
    }
    stage("Stage 1: RED sent (bank 4, global index 177+). Watching 7s...", 7);

    for buf in [
        direct(0x05, 0, 16, (0, 255, 0)),
        direct(0x05, 16, 16, (0, 255, 0)),
        direct(0x05, 32, 4, (0, 255, 0)),
    ] {
        let _ = kb.send_raw(&buf);
    }
    stage("Stage 2: GREEN sent (bank 5, zero-based). Watching 7s...", 7);

    for buf in [
        direct(0x06, 0, 16, (0, 80, 255)),
        direct(0x06, 16, 16, (0, 80, 255)),
        direct(0x06, 32, 4, (0, 80, 255)),
    ] {
        let _ = kb.send_raw(&buf);
    }
    stage("Stage 3: BLUE sent (bank 6, zero-based). Watching 7s...", 7);

    // built-in static mode fallback: zone byte 0 = all zones
    let mut b3 = [0u8; 64];
    b3[0] = 0x5D;
    b3[1] = 0xB3;
    b3[2] = 0x00; // zone
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
    stage("Stage 4: MAGENTA sent via built-in static mode (whole board may change). Watching 7s...", 7);

    println!("\nDone. Note the color sequence the REAR strip showed (e.g. \"dark, dark, green, magenta\"),");
    println!("and whether the FRONT under-edge light bar lit during any stage.");
    println!("Restart lighting from the Start Menu (Keyscape) when finished.");
}

fn run(args: Vec<String>) {
    let layout = Arc::new(layout::Layout::load());
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
