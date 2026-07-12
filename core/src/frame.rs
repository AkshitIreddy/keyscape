use crate::color::Col;

/// Full addressable frame: keyboard LEDs 0..=166, aux 167..=177 (lid logo
/// 168, front wrap-around bar 169/170/172/173), and the 33-segment rear
/// light strip 177..=209 under the lid logo.
pub const LED_COUNT: usize = 210;
pub const FRAME_BYTES: usize = LED_COUNT * 3;

#[derive(Clone)]
pub struct Frame {
    pub px: [Col; LED_COUNT],
}

impl Frame {
    pub fn new() -> Frame {
        Frame { px: [Col::BLACK; LED_COUNT] }
    }

    pub fn clear(&mut self, c: Col) {
        self.px = [c; LED_COUNT];
    }

    /// Multiply every pixel — the standard trail/decay op.
    pub fn fade(&mut self, k: f32) {
        for p in self.px.iter_mut() {
            *p = p.scale(k);
        }
    }

    pub fn set(&mut self, led: usize, c: Col) {
        if led < LED_COUNT {
            self.px[led] = c;
        }
    }

    pub fn add(&mut self, led: usize, c: Col) {
        if led < LED_COUNT {
            self.px[led] = self.px[led].add(c);
        }
    }

    pub fn max(&mut self, led: usize, c: Col) {
        if led < LED_COUNT {
            self.px[led] = self.px[led].max(c);
        }
    }

    /// Quantize to wire bytes with output gain and gamma.
    pub fn to_bytes(&self, gain: f32, gamma: f32, out: &mut [u8; FRAME_BYTES]) {
        for (i, p) in self.px.iter().enumerate() {
            let c = p.clamp01();
            out[i * 3] = ((c.r * gain).clamp(0.0, 1.0).powf(gamma) * 255.0) as u8;
            out[i * 3 + 1] = ((c.g * gain).clamp(0.0, 1.0).powf(gamma) * 255.0) as u8;
            out[i * 3 + 2] = ((c.b * gain).clamp(0.0, 1.0).powf(gamma) * 255.0) as u8;
        }
    }
}

/// Bitset over LED indices, used for key masks.
#[derive(Clone, Copy, PartialEq)]
pub struct LedMask([u64; 4]);

impl LedMask {
    pub fn none() -> LedMask {
        LedMask([0; 4])
    }

    pub fn all() -> LedMask {
        let mut m = LedMask::none();
        for i in 0..LED_COUNT {
            m.set(i);
        }
        m
    }

    pub fn set(&mut self, led: usize) {
        if led < LED_COUNT {
            self.0[led / 64] |= 1 << (led % 64);
        }
    }

    pub fn clear(&mut self, led: usize) {
        if led < LED_COUNT {
            self.0[led / 64] &= !(1 << (led % 64));
        }
    }

    pub fn get(&self, led: usize) -> bool {
        led < LED_COUNT && self.0[led / 64] & (1 << (led % 64)) != 0
    }

    /// Zero out everything outside the mask.
    pub fn apply(&self, frame: &mut Frame) {
        for i in 0..LED_COUNT {
            if !self.get(i) {
                frame.px[i] = Col::BLACK;
            }
        }
    }
}
