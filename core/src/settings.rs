//! Persisted daemon configuration: %APPDATA%\Keyscape\config.json.
//! The engine is the single writer; saves are debounced and atomic
//! (write temp + rename) so a crash can't truncate the config.

use crate::params::Params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

fn default_true() -> bool {
    true
}

fn default_transition() -> f32 {
    0.4
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct RearCfg {
    /// "follow" (mirror the scene), "static" (fixed color), or "off".
    pub mode: String,
    /// Hex color for static mode.
    pub color: String,
}

impl Default for RearCfg {
    fn default() -> Self {
        // Off by default: the rear strip is a built-in-effect-only
        // zone that cannot hold a color while the keyboard streams per-key
        // data, so "follow"/"static" only manage a brief flash and don't
        // persist. Opt in knowingly.
        RearCfg { mode: "off".into(), color: "#7C5CFF".into() }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PlaylistCfg {
    pub enabled: bool,
    /// true = shuffle, false = play the list in order.
    pub shuffle: bool,
    pub interval_sec: f32,
    /// Effect ids to cycle through; empty = every registered effect.
    pub effects: Vec<String>,
}

impl Default for PlaylistCfg {
    fn default() -> Self {
        PlaylistCfg { enabled: false, shuffle: true, interval_sec: 120.0, effects: vec![] }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AudioCfg {
    /// Music-reactive master switch. OFF by default — loopback capture only
    /// ever starts after the user explicitly enables it.
    pub enabled: bool,
    pub gain: f32,
    /// Which aspects of the active effect the music modulates.
    pub mod_brightness: bool,
    pub mod_speed: bool,
    pub mod_palette: bool,
    /// Modulation depth 0..1.
    pub amount: f32,
}

impl Default for AudioCfg {
    fn default() -> Self {
        AudioCfg {
            enabled: false,
            gain: 1.0,
            mod_brightness: true,
            mod_speed: true,
            mod_palette: false,
            amount: 0.7,
        }
    }
}

/// One global keyboard shortcut. `vk` is a Win32 virtual-key code; `vk == 0`
/// means the action is unbound (kept in the map so a deep-merge patch can
/// clear a binding without needing key deletion).
#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(default)]
pub struct HotkeyBinding {
    pub vk: u32,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GuardCfg {
    /// Try to stop ASUS LightingService while the daemon runs.
    pub manage_lighting_service: bool,
    /// Start it again when the daemon exits.
    pub restore_on_exit: bool,
    /// Bookkeeping across crashes: we stopped it and owe a restore.
    pub stopped_by_us: bool,
}

impl Default for GuardCfg {
    fn default() -> Self {
        GuardCfg { manage_lighting_service: true, restore_on_exit: true, stopped_by_us: false }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub active_effect: String,
    pub effect_params: HashMap<String, Params>,
    /// Hardware brightness 1-3 (0 would make per-key colors invisible).
    pub brightness: u8,
    /// Software master gain 0..1.
    pub master: f32,
    pub gamma: f32,
    pub fps: f32,
    pub paused: bool,
    /// Derive logo/light-bar colors from the keyboard scene.
    #[serde(default = "default_true")]
    pub aux_glow: bool,
    /// Start the lighting core at login (HKCU Run entry, enforced at start).
    #[serde(default = "default_true")]
    pub autostart: bool,
    /// Effect crossfade duration in seconds.
    #[serde(default = "default_transition")]
    pub transition: f32,
    /// Rear lid strip behavior (built-in-only zone).
    pub rear: RearCfg,
    /// Allow the keyboard tap stream for typing-reactive effects.
    #[serde(default = "default_true")]
    pub input_reactive: bool,
    pub playlist: PlaylistCfg,
    pub audio: AudioCfg,
    pub guard: GuardCfg,
    /// Global keyboard shortcuts: action id -> key binding.
    #[serde(default)]
    pub hotkeys: HashMap<String, HotkeyBinding>,
    /// Opaque UI preferences (theme, sounds, motion); the daemon just stores it.
    pub ui: serde_json::Value,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            active_effect: "nebula_drift".into(),
            effect_params: HashMap::new(),
            brightness: 3,
            master: 1.0,
            gamma: 1.8,
            fps: 30.0,
            paused: false,
            aux_glow: true,
            autostart: true,
            transition: 0.4,
            rear: RearCfg::default(),
            input_reactive: true,
            playlist: PlaylistCfg::default(),
            audio: AudioCfg::default(),
            guard: GuardCfg::default(),
            hotkeys: HashMap::new(),
            ui: serde_json::Value::Null,
        }
    }
}

/// Enforce the login-autostart registry entry for the core. Called at daemon
/// start and whenever the setting flips.
pub fn apply_autostart(on: bool) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    if on {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new("reg")
                .args(["add", key, "/v", "Keyscape", "/t", "REG_SZ", "/d",
                       &format!("\"{}\" run", exe.display()), "/f"])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
        }
    } else {
        let _ = std::process::Command::new("reg")
            .args(["delete", key, "/v", "Keyscape", "/f"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
}

pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Keyscape")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

impl Settings {
    pub fn load() -> Settings {
        match std::fs::read_to_string(config_path()) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self) {
        let dir = config_dir();
        let _ = std::fs::create_dir_all(&dir);
        let tmp = dir.join("config.json.tmp");
        if let Ok(s) = serde_json::to_string_pretty(self) {
            if std::fs::write(&tmp, s).is_ok() {
                let _ = std::fs::rename(&tmp, config_path());
            }
        }
    }
}
