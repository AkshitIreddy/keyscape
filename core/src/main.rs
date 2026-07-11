mod color;
mod frame;
mod hid;
mod layout;
mod math;
mod palette;
mod params;

use color::Col;
use frame::{Frame, FRAME_BYTES};

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
            // Smoke test: flood the whole board (incl. logo + light bar) with
            // one color. `--solid RRGGBB`
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
        _ => {
            let l = layout::Layout::load();
            println!(
                "keyscape-core scaffold: {} keys, {} aux LEDs, aspect {:.2} (engine not wired yet)",
                l.keys.len(),
                l.aux.len(),
                l.aspect
            );
        }
    }
}
