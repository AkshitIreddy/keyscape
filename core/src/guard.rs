//! ASUS LightingService conflict guard.
//!
//! Armoury Crate's LightingService writes to the same HID device (shared
//! handles, last-writer-wins). Strategy, in order of preference:
//! 1. If we can (elevated), stop the service while Keyscape runs and start
//!    it again on exit — bookkept in guard.json so a crash still restores.
//! 2. Otherwise fall back to keepalive mode: tell the engine to fully resend
//!    the frame every 2 s so our colors always win within a couple seconds.
//!
//! The UI additionally offers a one-click elevated "fix permanently"
//! (stop + disable) via `elevate_disable` / `elevate_enable`.

use crate::engine::Cmd;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

const SERVICE: &str = "LightingService";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn sc(args: &[&str]) -> Option<String> {
    Command::new("sc.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr))
}

pub fn is_running() -> bool {
    sc(&["query", SERVICE]).map(|out| out.contains("RUNNING")).unwrap_or(false)
}

/// True if the stop went through (needs elevation).
fn try_stop() -> bool {
    sc(&["stop", SERVICE])
        .map(|out| out.contains("STOP_PENDING") || out.contains("STOPPED"))
        .unwrap_or(false)
}

fn try_start() -> bool {
    sc(&["start", SERVICE])
        .map(|out| out.contains("START_PENDING") || out.contains("RUNNING"))
        .unwrap_or(false)
}

fn state_path() -> std::path::PathBuf {
    crate::settings::config_dir().join("guard.json")
}

fn load_stopped_by_us() -> bool {
    std::fs::read_to_string(state_path())
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("stopped_by_us").and_then(|b| b.as_bool()))
        .unwrap_or(false)
}

fn save_stopped_by_us(v: bool) {
    let _ = std::fs::create_dir_all(crate::settings::config_dir());
    let _ = std::fs::write(state_path(), format!("{{\"stopped_by_us\":{v}}}"));
}

/// One-click permanent fix, run elevated via UAC prompt: stop + disable.
pub fn elevate_disable() {
    let _ = Command::new("powershell")
        .args([
            "-WindowStyle", "Hidden", "-Command",
            "Start-Process sc.exe -ArgumentList 'stop','LightingService' -Verb RunAs -Wait; \
             Start-Process sc.exe -ArgumentList 'config','LightingService','start=','disabled' -Verb RunAs",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

/// Undo the permanent fix: re-enable + start.
pub fn elevate_enable() {
    let _ = Command::new("powershell")
        .args([
            "-WindowStyle", "Hidden", "-Command",
            "Start-Process sc.exe -ArgumentList 'config','LightingService','start=','auto' -Verb RunAs -Wait; \
             Start-Process sc.exe -ArgumentList 'start','LightingService' -Verb RunAs",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

pub struct Guard {
    manage: Arc<AtomicBool>,
    restore_on_exit: bool,
    stop_flag: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Guard {
    pub fn start(tx: Sender<Cmd>, manage: bool, restore_on_exit: bool) -> Guard {
        let manage_flag = Arc::new(AtomicBool::new(manage));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let m = manage_flag.clone();
        let s = stop_flag.clone();

        let handle = std::thread::Builder::new()
            .name("guard".into())
            .spawn(move || {
                let mut tried_stop_at: Option<std::time::Instant> = None;
                while !s.load(Ordering::Relaxed) {
                    let running = is_running();
                    let managing = m.load(Ordering::Relaxed);
                    if running && managing {
                        // Re-attempt at most once a minute (it always fails
                        // without elevation; no point hammering).
                        let due = tried_stop_at
                            .map(|t| t.elapsed() > Duration::from_secs(60))
                            .unwrap_or(true);
                        if due {
                            tried_stop_at = Some(std::time::Instant::now());
                            if try_stop() {
                                save_stopped_by_us(true);
                            }
                        }
                    }
                    let _ = tx.send(Cmd::SetKeepalive(running && is_running()));
                    // Re-check every 10 s in 200 ms slices so shutdown is fast.
                    for _ in 0..50 {
                        if s.load(Ordering::Relaxed) {
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }
                }
            })
            .expect("spawn guard");

        Guard { manage: manage_flag, restore_on_exit, stop_flag, handle: Some(handle) }
    }

    pub fn set_manage(&self, on: bool) {
        self.manage.store(on, Ordering::Relaxed);
    }

    /// Stop the watcher and restore the service if we owe it one.
    pub fn shutdown(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        if self.restore_on_exit && load_stopped_by_us() {
            if try_start() {
                save_stopped_by_us(false);
            }
        }
    }
}
