//! The render engine: one thread that owns the device, the active effect and
//! the settings. Event-driven — it ticks at the configured cap (default
//! 30 fps) while the scene is changing, throttles itself to 4 fps once the
//! output has been static for a couple of seconds, and only touches the HID
//! bus when bytes actually changed (plus a 2 s keepalive full resend while
//! ASUS's LightingService is alive and fighting for the device).

use crate::effects::{self, AudioFeatures, Effect, RenderCtx, Tap};
use crate::frame::{Frame, LedMask, FRAME_BYTES};
use crate::hid::Keyboard;
use crate::layout::{Layout, LIGHTBAR_LEDS, LOGO_LEDS};
use crate::math::Rng;
use crate::palette::Palette;
use crate::params::{self, Params};
use crate::settings::Settings;
use serde_json::{json, Value};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub enum Cmd {
    SetEffect(String),
    SetParams { id: String, params: Params },
    /// Deep-merge a JSON patch into `Settings` (see `Settings` for shape).
    PatchSettings(Value),
    /// Advance the playlist by hand (also works with playlist disabled).
    NextEffect,
    Input { taps: Vec<Tap>, held: LedMask },
    Audio(AudioFeatures),
    /// Guard reports whether LightingService is currently running.
    SetKeepalive(bool),
    SubscribePreview(Sender<Vec<u8>>),
    QueryStatus(Sender<Value>),
    Shutdown,
}

/// Lifecycle callbacks the engine fires so capture threads only run while
/// they're actually needed (a capture thread left running after an effect
/// swap burns CPU forever — learned the hard way).
pub struct Hooks {
    pub set_input_capture: Box<dyn Fn(bool) + Send>,
    pub set_audio_capture: Box<dyn Fn(bool) + Send>,
    pub set_guard_manage: Box<dyn Fn(bool) + Send>,
}

impl Default for Hooks {
    fn default() -> Self {
        Hooks {
            set_input_capture: Box::new(|_| {}),
            set_audio_capture: Box::new(|_| {}),
            set_guard_manage: Box::new(|_| {}),
        }
    }
}

struct Common {
    speed: f32,
    intensity: f32,
    palette: Palette,
    mask: LedMask,
}

pub struct Engine {
    layout: Arc<Layout>,
    settings: Settings,
    hooks: Hooks,
    kb: Option<Keyboard>,
    kb_retry_at: Instant,
    effect: Box<dyn Effect>,
    effect_id: String,
    common: Common,
    eff_params: Params,
    t: f32,
    fade: f32,
    pal_shift: f32,
    rng: Rng,
    taps: Vec<Tap>,
    held: LedMask,
    audio: AudioFeatures,
    keepalive: bool,
    last_send: Instant,
    /// Rear strip is a built-in-effect-only zone: last color pushed and when.
    rear_sent: (u8, u8, u8),
    rear_sent_at: Instant,
    last_tick: Instant,
    unchanged: u32,
    last_bytes: [u8; FRAME_BYTES],
    subs: Vec<Sender<Vec<u8>>>,
    playlist_next: Instant,
    settings_dirty: bool,
    settings_changed_at: Instant,
    started: Instant,
}

fn parse_common(layout: &Layout, params: &Params, default_palette: &str) -> Common {
    let palette = params
        .get("palette")
        .and_then(crate::palette::from_value)
        .or_else(|| crate::palette::by_name(default_palette))
        .unwrap_or_else(|| Palette::new(vec![]));
    Common {
        speed: params::get_f32(params, "speed", 1.0),
        intensity: params::get_f32(params, "intensity", 1.0),
        palette,
        mask: layout.mask(params::get_str(params, "mask", "all")),
    }
}

fn merge_json(dst: &mut Value, patch: &Value) {
    if let (Some(d), Some(p)) = (dst.as_object_mut(), patch.as_object()) {
        for (k, v) in p {
            match (d.get_mut(k), v.is_object()) {
                (Some(slot), true) if slot.is_object() => merge_json(slot, v),
                _ => {
                    d.insert(k.clone(), v.clone());
                }
            }
        }
    }
}

impl Engine {
    pub fn new(layout: Arc<Layout>, settings: Settings, hooks: Hooks) -> Engine {
        let now = Instant::now();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        let mut e = Engine {
            layout,
            settings,
            hooks,
            kb: None,
            kb_retry_at: now,
            effect: Box::new(NullEffect),
            effect_id: String::new(),
            common: Common { speed: 1.0, intensity: 1.0, palette: Palette::new(vec![]), mask: LedMask::all() },
            eff_params: Params::new(),
            t: 0.0,
            fade: 1.0,
            pal_shift: 0.0,
            rng: Rng::new(seed),
            taps: Vec::new(),
            held: LedMask::none(),
            audio: AudioFeatures::default(),
            keepalive: false,
            last_send: now,
            rear_sent: (0, 0, 0),
            rear_sent_at: now - Duration::from_secs(60),
            last_tick: now,
            unchanged: 0,
            last_bytes: [0; FRAME_BYTES],
            subs: Vec::new(),
            playlist_next: now + Duration::from_secs(3600),
            settings_dirty: false,
            settings_changed_at: now,
            started: now,
        };
        let id = e.settings.active_effect.clone();
        e.set_effect(&id);
        e.reset_playlist_timer();
        e
    }

    fn try_open_kb(&mut self) {
        if self.kb.is_some() || Instant::now() < self.kb_retry_at {
            return;
        }
        match Keyboard::open() {
            Ok(kb) => {
                let _ = kb.set_brightness(self.settings.brightness.clamp(1, 3));
                let _ = kb.set_zone_power_all();
                self.kb = Some(kb);
                self.unchanged = 0;
            }
            Err(_) => {
                self.kb_retry_at = Instant::now() + Duration::from_secs(5);
            }
        }
    }

    fn set_effect(&mut self, id: &str) {
        let info = match effects::by_id(id) {
            Some(i) => i,
            None => return,
        };
        self.effect = if effects::script::is_scripted(id) {
            effects::script::make(id, &self.layout, self.rng.next_u64())
        } else {
            (info.make)(&self.layout, self.rng.next_u64())
        };
        self.effect_id = id.to_string();
        let stored = self.settings.effect_params.get(id).cloned().unwrap_or_default();
        self.eff_params = params::with_defaults(&stored, &info.specs());
        self.common = parse_common(&self.layout, &self.eff_params, info.default_palette);
        self.t = 0.0;
        self.fade = 0.0;
        self.unchanged = 0;
        // sentinel: never produced by the quantizer -> rear repaints ~3 s
        // after the new effect settles
        self.rear_sent = (1, 1, 1);
        if self.settings.active_effect != id {
            self.settings.active_effect = id.to_string();
            self.mark_dirty();
        }
        (self.hooks.set_input_capture)(info.needs_input && self.settings.input_reactive);
    }

    fn set_params(&mut self, id: &str, p: Params) {
        self.settings.effect_params.insert(id.to_string(), p.clone());
        self.mark_dirty();
        if id == self.effect_id {
            if let Some(info) = effects::by_id(id) {
                self.eff_params = params::with_defaults(&p, &info.specs());
                self.common = parse_common(&self.layout, &self.eff_params, info.default_palette);
                self.unchanged = 0;
            }
        }
    }

    fn patch_settings(&mut self, patch: Value) {
        let before_brightness = self.settings.brightness;
        let before_audio = self.settings.audio.enabled;
        let before_input = self.settings.input_reactive;
        let before_manage = self.settings.guard.manage_lighting_service;
        let before_autostart = self.settings.autostart;
        let before_rear = self.settings.rear.clone();
        let mut v = serde_json::to_value(&self.settings).unwrap_or(Value::Null);
        merge_json(&mut v, &patch);
        if let Ok(s) = serde_json::from_value::<Settings>(v) {
            self.settings = s;
        }
        self.mark_dirty();
        self.unchanged = 0;
        if self.settings.brightness != before_brightness {
            if let Some(kb) = &self.kb {
                let _ = kb.set_brightness(self.settings.brightness.clamp(1, 3));
            }
        }
        if self.settings.audio.enabled != before_audio {
            (self.hooks.set_audio_capture)(self.settings.audio.enabled);
            if !self.settings.audio.enabled {
                self.audio = AudioFeatures::default();
            }
        }
        if self.settings.input_reactive != before_input {
            let needs = effects::by_id(&self.effect_id).map(|i| i.needs_input).unwrap_or(false);
            (self.hooks.set_input_capture)(needs && self.settings.input_reactive);
        }
        if self.settings.guard.manage_lighting_service != before_manage {
            (self.hooks.set_guard_manage)(self.settings.guard.manage_lighting_service);
        }
        if self.settings.autostart != before_autostart {
            crate::settings::apply_autostart(self.settings.autostart);
        }
        if self.settings.rear != before_rear {
            // repaint the rear strip promptly on mode/color changes
            self.rear_sent = (1, 1, 1);
            self.rear_sent_at = Instant::now() - Duration::from_secs(120);
        }
        self.reset_playlist_timer();
    }

    fn mark_dirty(&mut self) {
        self.settings_dirty = true;
        self.settings_changed_at = Instant::now();
    }

    fn reset_playlist_timer(&mut self) {
        self.playlist_next = if self.settings.playlist.enabled {
            Instant::now() + Duration::from_secs_f32(self.settings.playlist.interval_sec.max(10.0))
        } else {
            Instant::now() + Duration::from_secs(3600)
        };
    }

    fn playlist_ids(&self) -> Vec<&'static str> {
        let all: Vec<&'static str> = effects::registry().iter().map(|e| e.id).collect();
        if self.settings.playlist.effects.is_empty() {
            all
        } else {
            all.into_iter()
                .filter(|id| self.settings.playlist.effects.iter().any(|e| e == id))
                .collect()
        }
    }

    fn advance_playlist(&mut self) {
        let ids = self.playlist_ids();
        if ids.is_empty() {
            return;
        }
        let next = if self.settings.playlist.shuffle {
            let mut pick = ids[self.rng.below(ids.len())];
            if ids.len() > 1 {
                while pick == self.effect_id {
                    pick = ids[self.rng.below(ids.len())];
                }
            }
            pick
        } else {
            let cur = ids.iter().position(|id| *id == self.effect_id).unwrap_or(0);
            ids[(cur + 1) % ids.len()]
        };
        self.set_effect(next);
        self.reset_playlist_timer();
    }

    fn audio_speed_factor(&self) -> f32 {
        if !self.audio.active || !self.settings.audio.mod_speed {
            return 1.0;
        }
        let a = self.settings.audio.amount;
        1.0 + a * (0.35 * self.audio.level + 0.9 * self.audio.bass + 0.5 * self.audio.beat)
    }

    fn audio_gain_factor(&self) -> f32 {
        if !self.audio.active || !self.settings.audio.mod_brightness {
            return 1.0;
        }
        let a = self.settings.audio.amount;
        (1.0 - 0.45 * a + a * (0.55 * self.audio.level + 0.65 * self.audio.beat)).clamp(0.25, 1.6)
    }

    fn tick(&mut self, frame: &mut Frame, bytes: &mut [u8; FRAME_BYTES]) {
        let now = Instant::now();
        let gap = (now - self.last_tick).as_secs_f32();
        let dt_raw = gap.min(0.1);
        self.last_tick = now;

        // A large wall-clock gap means the machine slept/hibernated. The
        // firmware clears brightness/zone-power on resume, so re-assert once
        // and force a rear repaint — but do NOT poll this on a timer (that
        // was resetting the rear strip's built-in color every couple of
        // seconds, which is what made it flash and die).
        if gap > 2.5 {
            if let Some(kb) = &mut self.kb {
                let _ = kb.reassert_state(self.settings.brightness.clamp(1, 3));
            }
            self.rear_sent = (1, 1, 1);
            self.rear_sent_at = now - Duration::from_secs(120);
            self.unchanged = 0;
        }

        if self.settings.playlist.enabled && now >= self.playlist_next {
            self.advance_playlist();
        }

        self.try_open_kb();

        if self.settings.paused {
            frame.clear(crate::color::Col::BLACK);
        } else {
            let dt = dt_raw * self.audio_speed_factor();
            self.t += dt;

            // Music palette drift: centroid pushes the palette phase around.
            if self.audio.active && self.settings.audio.mod_palette {
                self.pal_shift += dt_raw * self.settings.audio.amount * (self.audio.centroid - 0.35) * 0.5;
            }
            let palette = if self.pal_shift.abs() > 1e-3 {
                self.common.palette.shifted(self.pal_shift)
            } else {
                self.common.palette.clone()
            };

            frame.clear(crate::color::Col::BLACK);
            let mut ctx = RenderCtx {
                t: self.t,
                dt,
                layout: &self.layout,
                palette: &palette,
                params: &self.eff_params,
                audio: self.audio,
                taps: &self.taps,
                held: &self.held,
                rng: &mut self.rng,
            };
            self.effect.render(&mut ctx, frame);
            self.taps.clear();

            self.fade =
                (self.fade + dt_raw / self.settings.transition.clamp(0.05, 3.0)).min(1.0);
            self.common.mask.apply(frame);

            if self.settings.aux_glow && !self.effect.writes_aux() {
                self.derive_aux(frame);
            }
        }

        let gain = self.settings.master.clamp(0.0, 1.0)
            * self.common.intensity
            * self.fade
            * self.audio_gain_factor();
        frame.to_bytes(gain, self.settings.gamma.max(0.2), bytes);

        if *bytes != self.last_bytes {
            self.unchanged = 0;
            if let Some(kb) = &mut self.kb {
                match kb.send_frame(bytes) {
                    Ok(_) => self.last_send = now,
                    Err(_) => {
                        self.kb = None;
                        self.kb_retry_at = now + Duration::from_secs(5);
                    }
                }
            }
            self.last_bytes = *bytes;
            // Push to preview subscribers (raw wire bytes; UI un-gammas).
            self.subs.retain(|s| s.send(bytes.to_vec()).is_ok());
        } else {
            self.unchanged = self.unchanged.saturating_add(1);
            if self.keepalive && (now - self.last_send).as_secs_f32() > 2.0 {
                if let Some(kb) = &mut self.kb {
                    if kb.resend_all().is_err() {
                        self.kb = None;
                        self.kb_retry_at = now + Duration::from_secs(5);
                    }
                }
                self.last_send = now;
            }
        }

        // Rear lid strip: a built-in-effect-only zone (ignores direct data),
        // so repaint it with a rare built-in static flash. The sequence
        // includes a flash save (B5), so the color is quantized to 4 levels
        // per channel and repainted at most once a minute (sooner right
        // after an effect switch) — flash wear stays negligible and the
        // one-frame board blink stays rare.
        let rear_due = (now - self.rear_sent_at).as_secs_f32()
            > if self.rear_sent == (1, 1, 1) { 3.0 } else { 60.0 };
        if self.fade >= 1.0 && rear_due {
            let q = |v: u8| (v >> 6) << 6;
            let rear = match self.settings.rear.mode.as_str() {
                "off" => (0, 0, 0),
                "static" => {
                    let hex = u32::from_str_radix(
                        self.settings.rear.color.trim_start_matches('#'),
                        16,
                    )
                    .unwrap_or(0x7C5CFF);
                    (((hex >> 16) & 0xFF) as u8, ((hex >> 8) & 0xFF) as u8, (hex & 0xFF) as u8)
                }
                _ => (
                    q(bytes[176 * 3] / 2 + bytes[177 * 3] / 2),
                    q(bytes[176 * 3 + 1] / 2 + bytes[177 * 3 + 1] / 2),
                    q(bytes[176 * 3 + 2] / 2 + bytes[177 * 3 + 2] / 2),
                ),
            };
            if rear != self.rear_sent {
                if let Some(kb) = &mut self.kb {
                    if kb.set_rear_via_builtin(rear.0, rear.1, rear.2).is_err() {
                        self.kb = None;
                        self.kb_retry_at = now + Duration::from_secs(5);
                    }
                    self.rear_sent = rear;
                    self.rear_sent_at = now;
                    self.unchanged = 0;
                }
            }
        }

        if self.settings_dirty && (now - self.settings_changed_at).as_secs_f32() > 1.5 {
            self.settings.save();
            self.settings_dirty = false;
        }
    }

    /// Logo mirrors the scene's average; light bar segments mirror the bottom
    /// row left→right so the glow feels like it leaks out of the keyboard.
    /// Colors are boosted hue-preservingly to a visibility floor — dark
    /// ambient scenes average to near-black, which after gamma quantizes to
    /// wire bytes of ~5 and reads as "logo is off".
    fn derive_aux(&self, frame: &mut Frame) {
        fn boost(c: crate::color::Col, target: f32, max_gain: f32) -> crate::color::Col {
            let m = c.r.max(c.g).max(c.b);
            if m < 1e-3 {
                return c; // true black stays black
            }
            c.scale((target / m).clamp(1.0, max_gain)).clamp01()
        }

        let mut avg = crate::color::Col::BLACK;
        let mut n = 0.0;
        for k in &self.layout.keys {
            avg = avg.add(frame.px[k.led]);
            n += 1.0;
        }
        if n > 0.0 {
            avg = avg.scale(1.0 / n);
        }
        // A palette-driven glow floor: the logo (and, dimmer, the bars) should
        // never be black even when the scene idles dark. Two offset samples so
        // palettes with dark stops can't zero it out.
        let glow = {
            let a = self.common.palette.sample(self.t * 0.02);
            let b = self.common.palette.sample(self.t * 0.02 + 0.33);
            boost(a.max(b), 0.9, 30.0)
        };

        // Aux LEDs shine through diffusers and read dimmer than keycaps, so
        // they get pushed to full range.
        let logo = boost(avg, 1.0, 16.0).max(glow);
        for led in LOGO_LEDS {
            frame.set(led, logo);
        }

        let bottom: Vec<&crate::layout::Key> =
            self.layout.keys.iter().filter(|k| k.row == 6).collect();
        for (i, led) in LIGHTBAR_LEDS.iter().enumerate() {
            let fr = i as f32 / (LIGHTBAR_LEDS.len() - 1) as f32;
            let mut best = avg;
            let mut best_d = f32::MAX;
            for k in &bottom {
                let d = (k.cx / self.layout.aspect - fr).abs();
                if d < best_d {
                    best_d = d;
                    best = frame.px[k.led];
                }
            }
            frame.set(*led, boost(best.max(avg.scale(0.6)), 0.9, 14.0).max(glow.scale(0.55)));
        }

        // Rear light strip (chassis rear, under the lid logo): mirror the top
        // keyboard rows, x-flipped because it's viewed from behind.
        if !self.layout.rear.is_empty() {
            let top: Vec<&crate::layout::Key> =
                self.layout.keys.iter().filter(|k| k.row <= 2).collect();
            let n_rear = self.layout.rear.len();
            for (i, &led) in self.layout.rear.iter().enumerate() {
                let fr = 1.0 - i as f32 / (n_rear - 1) as f32;
                let mut best = avg;
                let mut best_d = f32::MAX;
                for k in &top {
                    let d = (k.cx / self.layout.aspect - fr).abs();
                    if d < best_d {
                        best_d = d;
                        best = frame.px[k.led];
                    }
                }
                frame.set(led, boost(best.max(avg.scale(0.5)), 0.9, 14.0).max(glow.scale(0.5)));
            }
        }
    }

    fn status(&self) -> Value {
        let info = effects::by_id(&self.effect_id);
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "effect": self.effect_id,
            "effect_name": info.map(|i| i.name).unwrap_or(""),
            "category": info.map(|i| i.category).unwrap_or(""),
            "params": self.eff_params,
            "hid_connected": self.kb.is_some(),
            "keepalive": self.keepalive,
            "uptime_sec": (Instant::now() - self.started).as_secs(),
            "audio": {
                "active": self.audio.active,
                "level": self.audio.level,
                "bass": self.audio.bass,
                "mid": self.audio.mid,
                "treble": self.audio.treble,
                "beat": self.audio.beat,
            },
            "settings": serde_json::to_value(&self.settings).unwrap_or(Value::Null),
        })
    }

    pub fn run(mut self, rx: Receiver<Cmd>) {
        let mut frame = Frame::new();
        let mut bytes = [0u8; FRAME_BYTES];
        loop {
            // Static scene → 4 fps; active scene → configured cap.
            let fps = self.settings.fps.clamp(5.0, 60.0);
            let idle = self.unchanged > (fps as u32) * 2;
            let interval = if idle { Duration::from_millis(250) } else { Duration::from_secs_f32(1.0 / fps) };
            let deadline = self.last_tick + interval;
            let timeout = deadline.saturating_duration_since(Instant::now());

            match rx.recv_timeout(timeout) {
                Ok(cmd) => match cmd {
                    Cmd::SetEffect(id) => self.set_effect(&id),
                    Cmd::SetParams { id, params } => self.set_params(&id, params),
                    Cmd::PatchSettings(p) => self.patch_settings(p),
                    Cmd::NextEffect => self.advance_playlist(),
                    Cmd::Input { mut taps, held } => {
                        if !taps.is_empty() || held != self.held {
                            self.unchanged = 0;
                        }
                        // stamp taps with effect-local time
                        for tap in &mut taps {
                            tap.t = self.t;
                        }
                        self.taps.extend(taps);
                        self.held = held;
                    }
                    Cmd::Audio(mut a) => {
                        // input sensitivity from settings
                        let g = self.settings.audio.gain.clamp(0.1, 4.0);
                        a.level = (a.level * g).min(1.2);
                        a.bass = (a.bass * g).min(1.2);
                        a.mid = (a.mid * g).min(1.2);
                        a.treble = (a.treble * g).min(1.2);
                        self.audio = a;
                        if a.active {
                            self.unchanged = 0;
                        }
                    }
                    Cmd::SetKeepalive(on) => self.keepalive = on,
                    Cmd::SubscribePreview(s) => {
                        self.unchanged = 0;
                        self.subs.push(s);
                    }
                    Cmd::QueryStatus(s) => {
                        let _ = s.send(self.status());
                    }
                    Cmd::Shutdown => break,
                },
                Err(RecvTimeoutError::Timeout) => {
                    self.tick(&mut frame, &mut bytes);
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        if self.settings_dirty {
            self.settings.save();
        }
        // Leave the board dark on exit; the service guard restores ASUS's
        // lighting service, which will repaint on its own.
        if let Some(kb) = &mut self.kb {
            let black = [0u8; FRAME_BYTES];
            let _ = kb.send_frame(&black);
        }
    }
}

/// Placeholder until the first real effect is set.
struct NullEffect;

impl Effect for NullEffect {
    fn render(&mut self, _ctx: &mut RenderCtx, _out: &mut Frame) {}
}
