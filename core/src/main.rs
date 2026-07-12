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
        Some("--list") => {
            for e in effects::registry() {
                println!("{:24} {:10} {}", e.id, e.category, e.name);
            }
            println!("{} effects", effects::registry().len());
        }
        _ => run(args),
    }
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
