mod color;
mod effects;
mod engine;
mod frame;
mod hid;
mod layout;
mod math;
mod palette;
mod params;
mod settings;

use color::Col;
use frame::{Frame, FRAME_BYTES};
use std::sync::mpsc;
use std::sync::Arc;

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

    let (tx, rx) = mpsc::channel();
    let eng = engine::Engine::new(layout.clone(), settings, engine::Hooks::default());
    let engine_thread = std::thread::Builder::new()
        .name("engine".into())
        .spawn(move || eng.run(rx))
        .expect("spawn engine");

    println!(
        "keyscape-core running ({} effects registered). Ctrl+C to stop.",
        effects::registry().len()
    );
    // IPC server takes over this thread in a later commit; for now just wait.
    let _ = tx; // keep the channel alive
    let _ = engine_thread.join();
}
