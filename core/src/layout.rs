//! Physical keyboard layout, loaded from the JSON extracted out of ASUS's own
//! per-key DeviceContent CSV (see tools/parse-layout.mjs).

use crate::frame::{LedMask, LED_COUNT};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct RawKey {
    led: u16,
    name: String,
    row: u8,
    col: u8,
    scan: Option<u16>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Deserialize)]
struct RawAux {
    led: u16,
    name: String,
}

#[derive(Deserialize)]
struct RawLayout {
    keys: Vec<RawKey>,
    aux: Vec<RawAux>,
}

#[derive(Clone, Debug)]
pub struct Key {
    pub led: usize,
    pub name: String,
    pub row: u8,
    pub col: u8,
    pub scan: Option<u16>,
    /// Key rect, normalized to the keyboard bounding box (y: 0 top .. 1 bottom).
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Key center in isotropic space: cx in 0..ASPECT, cy in 0..1, so that
    /// distances mean the same thing horizontally and vertically.
    pub cx: f32,
    pub cy: f32,
}

pub struct Layout {
    pub keys: Vec<Key>,
    pub aux: Vec<(usize, String)>,
    /// Rear light strip LEDs (chassis rear, under the lid logo), ordered
    /// left-to-right in lid space.
    pub rear: Vec<usize>,
    /// Width of isotropic space (height is 1.0).
    pub aspect: f32,
    /// key index by LED index (None for gaps / aux).
    pub key_of_led: Vec<Option<usize>>,
    /// key index by PS/2 scan code (with 0xE0-extended codes as 0x80|code).
    pub key_of_scan: HashMap<u16, usize>,
    /// Neighbor key indices (within ~1.35 key pitch), for graph effects.
    pub neighbors: Vec<Vec<usize>>,
}

/// Lid logo is 167 per OpenRGB/g-helper; 168 is mirrored as a safety net.
pub const LOGO_LEDS: [usize; 2] = [167, 168];
/// Front light bar, physical left → right.
pub const LIGHTBAR_LEDS: [usize; 6] = [174, 173, 172, 171, 170, 169];

impl Layout {
    pub fn load() -> Layout {
        let raw: RawLayout =
            serde_json::from_str(include_str!("../assets/layout_us.json"))
                .expect("embedded layout JSON is valid");

        // Real keyboard bounds are 2318x935 px => aspect ~2.48.
        let aspect = 2318.0 / 935.0_f32;
        let keys: Vec<Key> = raw
            .keys
            .iter()
            .map(|k| Key {
                led: k.led as usize,
                name: k.name.clone(),
                row: k.row,
                col: k.col,
                scan: k.scan,
                x: k.x,
                y: k.y,
                w: k.w,
                h: k.h,
                cx: (k.x + k.w * 0.5) * aspect,
                cy: k.y + k.h * 0.5,
            })
            .collect();

        let mut key_of_led = vec![None; LED_COUNT];
        let mut key_of_scan = HashMap::new();
        for (i, k) in keys.iter().enumerate() {
            key_of_led[k.led] = Some(i);
            if let Some(s) = k.scan {
                key_of_scan.insert(s, i);
            }
        }

        // Average pitch = median horizontal center distance between adjacent
        // letters; ~0.16 in iso space. Neighbors = centers within 1.35 pitch.
        let pitch = 0.155_f32;
        let mut neighbors = vec![Vec::new(); keys.len()];
        for i in 0..keys.len() {
            for j in 0..keys.len() {
                if i == j {
                    continue;
                }
                let dx = keys[i].cx - keys[j].cx;
                let dy = keys[i].cy - keys[j].cy;
                if (dx * dx + dy * dy).sqrt() < pitch * 1.35 {
                    neighbors[i].push(j);
                }
            }
        }

        let mut rear: Vec<usize> = raw
            .aux
            .iter()
            .filter(|a| a.name.starts_with("Rear"))
            .map(|a| a.led as usize)
            .collect();
        rear.sort();

        Layout {
            keys,
            aux: raw.aux.iter().map(|a| (a.led as usize, a.name.clone())).collect(),
            rear,
            aspect,
            key_of_led,
            key_of_scan,
            neighbors,
        }
    }

    pub fn key_by_name(&self, name: &str) -> Option<&Key> {
        self.keys.iter().find(|k| k.name == name)
    }

    /// Resolve a named mask to a LED bitset. Unknown names resolve to "all".
    pub fn mask(&self, name: &str) -> LedMask {
        let mut m = LedMask::none();
        let mut add = |pred: &dyn Fn(&Key) -> bool| {
            for k in self.keys.iter().filter(|k| pred(k)) {
                m.set(k.led);
            }
        };
        match name {
            "letters" => add(&|k| k.name.len() == 1 && k.name.chars().next().unwrap().is_ascii_alphabetic()),
            "wasd" => add(&|k| matches!(k.name.as_str(), "W" | "A" | "S" | "D")),
            "arrows" => add(&|k| matches!(k.name.as_str(), "Up" | "Down" | "Left" | "Right")),
            "function" => add(&|k| k.name.starts_with('F') && k.name.len() <= 3 && k.name[1..].chars().all(|c| c.is_ascii_digit())),
            "numbers" => add(&|k| k.row == 2 && k.name.len() == 1 && k.name.chars().next().unwrap().is_ascii_digit()),
            "modifiers" => add(&|k| {
                matches!(
                    k.name.as_str(),
                    "LShift" | "RShift" | "LCtrl" | "RCtrl" | "LAlt" | "RAlt" | "Win" | "Fn" | "Caps" | "Tab" | "Enter" | "Backspace" | "Esc" | "Space"
                )
            }),
            "media" => add(&|k| {
                k.row == 0 || matches!(k.name.as_str(), "Play" | "Stop" | "Prev" | "Next")
            }),
            "main" => add(&|k| k.row >= 1 && !matches!(k.name.as_str(), "Play" | "Stop" | "Prev" | "Next")),
            "edge" => {
                // Perimeter of the main block: top F-row, bottom row, and the
                // leftmost/rightmost key of every row.
                add(&|k| k.row == 1 || k.row == 6);
                for row in 2..=5u8 {
                    let mut in_row: Vec<&Key> = self.keys.iter().filter(|k| k.row == row).collect();
                    in_row.sort_by(|a, b| a.cx.partial_cmp(&b.cx).unwrap());
                    if let Some(k) = in_row.first() {
                        m.set(k.led);
                    }
                    if let Some(k) = in_row.last() {
                        m.set(k.led);
                    }
                }
            }
            _ => {
                m = LedMask::all();
            }
        }
        m
    }

    pub fn mask_names() -> &'static [&'static str] {
        &["all", "main", "letters", "wasd", "arrows", "function", "numbers", "modifiers", "media", "edge"]
    }
}
