//! Effect framework. Every effect renders a full keyboard frame from a
//! `RenderCtx`; the engine owns timing, masking, intensity, transitions,
//! aux-LED derivation and HID delivery.
//!
//! Contract for effect authors:
//! - Read *extra* params live from `ctx.params` every frame (no caching), so
//!   UI changes apply instantly without instance rebuilds.
//! - `speed` is already applied: `ctx.t`/`ctx.dt` advance faster or slower.
//!   `intensity`, `mask` and transitions are applied by the engine afterward.
//! - Index per-key state by key index (0..layout.keys.len()), not LED index.
//! - Persistent internal state is fine; it is dropped when the user switches
//!   effects (capture threads etc. must stop in `Drop`).

use crate::frame::{Frame, LedMask};
use crate::layout::Layout;
use crate::math::Rng;
use crate::palette::Palette;
use crate::params::{ParamSpec, Params};

pub mod ambient;
pub mod cosmic;
pub mod digital;
pub mod kinetic;
pub mod organic;
pub mod physics;
pub mod script;
pub mod typing;

/// Smoothed system-audio features (all 0..1-ish, already attack/release
/// filtered off the render path). `active` is false when music mode is off.
#[derive(Clone, Copy, Default)]
pub struct AudioFeatures {
    pub active: bool,
    pub level: f32,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    /// Beat impulse: jumps to ~1 on onset, decays in ~150 ms.
    pub beat: f32,
    /// Spectral centroid 0..1 (dark → bright timbre).
    pub centroid: f32,
}

/// One key-down event, in engine time.
#[derive(Clone, Copy)]
pub struct Tap {
    pub key: usize,
    pub led: usize,
    pub cx: f32,
    pub cy: f32,
    pub t: f32,
}

pub struct RenderCtx<'a> {
    /// Effect-local time in seconds, speed-scaled (and music-scaled).
    pub t: f32,
    pub dt: f32,
    pub layout: &'a Layout,
    pub palette: &'a Palette,
    pub params: &'a Params,
    pub audio: AudioFeatures,
    /// Key-down events since the previous frame.
    pub taps: &'a [Tap],
    /// Keys currently held (by LED index).
    pub held: &'a LedMask,
    pub rng: &'a mut Rng,
}

pub trait Effect: Send {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame);
    /// Effects that write the logo/light bar themselves return true; otherwise
    /// the engine derives aux colors from the keyboard frame.
    fn writes_aux(&self) -> bool {
        false
    }
}

pub struct EffectInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub blurb: &'static str,
    /// Wants the keyboard tap stream (engine only runs the input hook while
    /// the active effect needs it).
    pub needs_input: bool,
    pub default_palette: &'static str,
    pub extras: fn() -> Vec<ParamSpec>,
    pub make: fn(&Layout, u64) -> Box<dyn Effect>,
}

impl EffectInfo {
    /// Full param specs: common (speed/intensity/palette/mask) + extras.
    /// Scripted effects declare extras in their manifest (side-table lookup,
    /// since fn pointers can't capture per-script data).
    pub fn specs(&self) -> Vec<ParamSpec> {
        let mut v = crate::params::common_specs(self.default_palette);
        v.extend(script::extras_for(self.id));
        v.extend((self.extras)());
        v
    }
}

pub fn no_extras() -> Vec<ParamSpec> {
    vec![]
}

/// Shared "direction" extra for effects where it makes sense.
pub fn dir_param(default: &'static str) -> ParamSpec {
    ParamSpec::select("direction", "Direction", vec!["right", "left", "down", "up"], default)
}

/// Unit vector for the "direction" param in iso space.
pub fn dir_vec(p: &Params) -> (f32, f32) {
    match crate::params::get_str(p, "direction", "right") {
        "left" => (-1.0, 0.0),
        "down" => (0.0, 1.0),
        "up" => (0.0, -1.0),
        _ => (1.0, 0.0),
    }
}

fn builtins() -> &'static [EffectInfo] {
    use std::sync::OnceLock;
    static REG: OnceLock<Vec<EffectInfo>> = OnceLock::new();
    REG.get_or_init(|| {
        let mut v = Vec::new();
        v.extend(organic::effects());
        v.extend(physics::effects());
        v.extend(cosmic::effects());
        v.extend(digital::effects());
        v.extend(typing::effects());
        v.extend(ambient::effects());
        v.extend(kinetic::effects());
        v
    })
}

static SCRIPT_REG: std::sync::OnceLock<&'static [EffectInfo]> = std::sync::OnceLock::new();

/// Register user script effects (once, at startup, before the engine runs).
pub fn register_scripts(v: Vec<EffectInfo>) {
    let _ = SCRIPT_REG.set(Box::leak(v.into_boxed_slice()));
}

/// The full effect registry: built-ins in category order, then user script
/// effects.
pub fn registry() -> Vec<&'static EffectInfo> {
    let mut out: Vec<&'static EffectInfo> = builtins().iter().collect();
    if let Some(js) = SCRIPT_REG.get() {
        out.extend(js.iter());
    }
    out
}

pub fn by_id(id: &str) -> Option<&'static EffectInfo> {
    if let Some(e) = builtins().iter().find(|e| e.id == id) {
        return Some(e);
    }
    SCRIPT_REG.get().and_then(|js| js.iter().find(|e| e.id == id))
}
