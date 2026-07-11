//! Color math in f32 (0..1 per channel). Frames render in this space and are
//! quantized with gamma only at the HID boundary.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Col {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Col {
    pub const BLACK: Col = Col { r: 0.0, g: 0.0, b: 0.0 };
    pub const WHITE: Col = Col { r: 1.0, g: 1.0, b: 1.0 };

    pub fn rgb(r: f32, g: f32, b: f32) -> Col {
        Col { r, g, b }
    }

    pub fn hex(v: u32) -> Col {
        Col {
            r: ((v >> 16) & 0xFF) as f32 / 255.0,
            g: ((v >> 8) & 0xFF) as f32 / 255.0,
            b: (v & 0xFF) as f32 / 255.0,
        }
    }

    /// h, s, v all in 0..1; h wraps.
    pub fn hsv(h: f32, s: f32, v: f32) -> Col {
        let h = (h.fract() + 1.0).fract() * 6.0;
        let i = h.floor();
        let f = h - i;
        let p = v * (1.0 - s);
        let q = v * (1.0 - s * f);
        let t = v * (1.0 - s * (1.0 - f));
        let (r, g, b) = match i as i32 {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        Col { r, g, b }
    }

    pub fn lerp(a: Col, b: Col, t: f32) -> Col {
        let t = t.clamp(0.0, 1.0);
        Col {
            r: a.r + (b.r - a.r) * t,
            g: a.g + (b.g - a.g) * t,
            b: a.b + (b.b - a.b) * t,
        }
    }

    pub fn scale(self, k: f32) -> Col {
        Col { r: self.r * k, g: self.g * k, b: self.b * k }
    }

    pub fn add(self, o: Col) -> Col {
        Col { r: self.r + o.r, g: self.g + o.g, b: self.b + o.b }
    }

    /// Per-channel max — layering lights without blowing out.
    pub fn max(self, o: Col) -> Col {
        Col { r: self.r.max(o.r), g: self.g.max(o.g), b: self.b.max(o.b) }
    }

    pub fn clamp01(self) -> Col {
        Col {
            r: self.r.clamp(0.0, 1.0),
            g: self.g.clamp(0.0, 1.0),
            b: self.b.clamp(0.0, 1.0),
        }
    }

    pub fn luma(self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
