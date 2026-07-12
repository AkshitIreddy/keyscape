// Keyscape UI shell. All lighting lives in keyscape-core (a separate,
// always-on process) — this window is just a WebView over the core's
// WebSocket API. Closing it never touches the lighting.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const CORE_PORT: u16 = 53971;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

fn core_running() -> bool {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, CORE_PORT));
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

fn core_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // installed layout and dev layout (shared workspace target dir)
            v.push(dir.join("keyscape-core.exe"));
            v.push(dir.join("../release/keyscape-core.exe"));
            v.push(dir.join("../debug/keyscape-core.exe"));
        }
    }
    v
}

/// Start the core if it isn't already listening; wait briefly for the port.
/// (Login autostart is owned by the core itself, driven by its settings.)
fn ensure_core() -> bool {
    for cand in core_candidates() {
        if cand.exists() {
            if !core_running() {
                let _ = Command::new(&cand)
                    .arg("run")
                    .creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
                    .spawn();
            }
            break;
        }
    }
    for _ in 0..12 {
        if core_running() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    core_running()
}

#[tauri::command]
fn start_core() -> bool {
    ensure_core()
}

fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            std::thread::spawn(|| {
                ensure_core();
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![start_core])
        .run(tauri::generate_context!())
        .expect("failed to run Keyscape");
}
