use crate::color::Col;
use serde_json::Value;

/// Gradient palette sampled by t in 0..1 (wrapping), so effects can scroll
/// through it forever without seams.
#[derive(Clone)]
pub struct Palette {
    stops: Vec<(f32, Col)>,
}

impl Palette {
    pub fn new(stops: Vec<(f32, Col)>) -> Palette {
        let mut stops = stops;
        stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        if stops.is_empty() {
            stops.push((0.0, Col::WHITE));
        }
        Palette { stops }
    }

    /// Sample with wrap-around between the last and first stop.
    pub fn sample(&self, t: f32) -> Col {
        let t = (t.fract() + 1.0).fract();
        let n = self.stops.len();
        if n == 1 {
            return self.stops[0].1;
        }
        for i in 0..n {
            let (t0, c0) = self.stops[i];
            let (t1, c1) = if i + 1 < n {
                self.stops[i + 1]
            } else {
                (self.stops[0].0 + 1.0, self.stops[0].1)
            };
            if t >= t0 && t < t1 {
                let span = (t1 - t0).max(1e-5);
                return Col::lerp(c0, c1, (t - t0) / span);
            }
        }
        // t below the first stop: blend from the (wrapped) last stop.
        let (tl, cl) = *self.stops.last().unwrap();
        let (t0, c0) = self.stops[0];
        let span = (t0 + 1.0 - tl).max(1e-5);
        Col::lerp(cl, c0, (t + 1.0 - tl) / span)
    }

    /// Stop list, for serializing palettes out over IPC.
    pub fn stops(&self) -> &[(f32, Col)] {
        &self.stops
    }

    /// A copy with every stop phase-shifted (wrapping) — used by the music
    /// layer to drift hues without effects knowing.
    pub fn shifted(&self, dt: f32) -> Palette {
        Palette::new(
            self.stops
                .iter()
                .map(|(t, c)| (((t + dt).fract() + 1.0).fract(), *c))
                .collect(),
        )
    }

    /// Convenience: sample without wrap (clamps at the ends).
    pub fn sample_clamped(&self, t: f32) -> Col {
        let t = t.clamp(0.0, 1.0);
        let n = self.stops.len();
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        if t >= self.stops[n - 1].0 {
            return self.stops[n - 1].1;
        }
        self.sample(t)
    }
}

macro_rules! pal {
    ($(($t:expr, $c:expr)),+ $(,)?) => {
        Palette::new(vec![$(($t, Col::hex($c))),+])
    };
}

/// (id, display name, palette). Order = display order in the UI.
pub fn builtins() -> Vec<(&'static str, &'static str, Palette)> {
    vec![
        ("aurora", "Aurora", pal![(0.0, 0x021A12), (0.25, 0x0FE0A0), (0.5, 0x2E86FF), (0.75, 0x7A2EFF), (0.9, 0x041020)]),
        ("synthwave", "Synthwave", pal![(0.0, 0x12002E), (0.3, 0xFF2E97), (0.6, 0x00E5FF), (0.85, 0x5A00B4)]),
        ("ember", "Ember", pal![(0.0, 0x0A0000), (0.3, 0x7A1000), (0.6, 0xFF5A00), (0.85, 0xFFC846)]),
        ("glacier", "Glacier", pal![(0.0, 0x02102E), (0.35, 0x1560C8), (0.7, 0x7ADCFF), (0.92, 0xEAF8FF)]),
        ("sakura", "Sakura", pal![(0.0, 0x2E0A1E), (0.3, 0xC83C78), (0.65, 0xFFB4D2), (0.9, 0xFFF0F5)]),
        ("oceanic", "Oceanic", pal![(0.0, 0x001428), (0.35, 0x006496), (0.7, 0x00C8B4), (0.9, 0xB4FFE6)]),
        ("sunset", "Sunset", pal![(0.0, 0x28104E), (0.35, 0xC83264), (0.65, 0xFF8C3C), (0.9, 0xFFD28C)]),
        ("matrix", "Matrix", pal![(0.0, 0x000A00), (0.4, 0x00641E), (0.75, 0x00E650), (0.95, 0xB4FFC8)]),
        ("toxic", "Toxic", pal![(0.0, 0x0A1400), (0.4, 0x3C6E00), (0.75, 0xA0E600), (0.95, 0xE6FF64)]),
        ("royal", "Royal", pal![(0.0, 0x14003C), (0.4, 0x4B0096), (0.7, 0xB43CFF), (0.92, 0xFFD700)]),
        ("candy", "Candy", pal![(0.0, 0xFF6EC7), (0.25, 0xFFD36E), (0.5, 0x6EFFB4), (0.75, 0x6EC7FF)]),
        ("inferno", "Inferno", pal![(0.0, 0x000004), (0.3, 0x56106E), (0.6, 0xE1642E), (0.9, 0xFCFEA4)]),
        ("viridian", "Viridian", pal![(0.0, 0x440154), (0.35, 0x31688E), (0.7, 0x35B779), (0.95, 0xFDE725)]),
        ("ultraviolet", "Ultraviolet", pal![(0.0, 0x0A0028),
            (0.4, 0x3C00A0), (0.7, 0x8C46FF), (0.92, 0xC8B4FF)]),
        ("copper", "Copper", pal![(0.0, 0x140A05), (0.4, 0x7A4623), (0.75, 0xDC9650), (0.95, 0xFFE6C8)]),
        ("miami", "Miami", pal![(0.0, 0x0A2E3C), (0.35, 0x00C8C8), (0.65, 0xFF64A0), (0.9, 0xFFDC96)]),
        ("forest", "Forest", pal![(0.0, 0x0A1E0A), (0.35, 0x1E6432), (0.7, 0x64B450), (0.92, 0xDCF0B4)]),
        ("bloodmoon", "Blood Moon", pal![(0.0, 0x140000), (0.4, 0x640A0A), (0.75, 0xC81E1E), (0.95, 0xFF9664)]),
        ("hologram", "Hologram", pal![(0.0, 0x003C50), (0.3, 0x00DCFF), (0.6, 0xB4FFFF), (0.8, 0x5064FF)]),
        ("peppermint", "Peppermint", pal![(0.0, 0x003228), (0.35, 0x00B48C), (0.7, 0xFFFFFF), (0.9, 0xFF6478)]),
        ("midnight", "Midnight", pal![(0.0, 0x00020A), (0.4, 0x0A1E50), (0.75, 0x3C64C8), (0.95, 0x96BEFF)]),
        ("mono", "Moonlight Mono", pal![(0.0, 0x000000), (0.45, 0x8296AA), (0.9, 0xF0F6FF)]),
    ]
}

pub fn by_name(name: &str) -> Option<Palette> {
    builtins().into_iter().find(|(id, _, _)| *id == name).map(|(_, _, p)| p)
}

/// Parse a palette param: either a builtin id string or an inline stop list
/// [{"t":0.0,"c":"#RRGGBB"}, ...] for fully custom palettes.
pub fn from_value(v: &Value) -> Option<Palette> {
    match v {
        Value::String(s) => by_name(s),
        Value::Array(arr) => {
            let mut stops = Vec::new();
            for e in arr {
                let t = e.get("t")?.as_f64()? as f32;
                let c = e.get("c")?.as_str()?;
                let hex = u32::from_str_radix(c.trim_start_matches('#'), 16).ok()?;
                stops.push((t, Col::hex(hex)));
            }
            if stops.is_empty() {
                None
            } else {
                Some(Palette::new(stops))
            }
        }
        _ => None,
    }
}
