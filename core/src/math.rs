//! Deterministic rng + value noise used across effects. No external deps —
//! effects only need "good enough" noise, cheaply.

#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform in [a, b).
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }
}

fn hash2(ix: i32, iy: i32, seed: u32) -> f32 {
    let mut h = (ix as u32).wrapping_mul(0x85EBCA6B)
        ^ (iy as u32).wrapping_mul(0xC2B2AE35)
        ^ seed.wrapping_mul(0x27D4EB2F);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A2D39);
    h ^= h >> 15;
    (h & 0xFFFFFF) as f32 / 16777216.0
}

/// Smooth 2D value noise in [0, 1].
pub fn noise2(x: f32, y: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    let a = hash2(ix, iy, seed);
    let b = hash2(ix + 1, iy, seed);
    let c = hash2(ix, iy + 1, seed);
    let d = hash2(ix + 1, iy + 1, seed);
    a + (b - a) * ux + (c - a) * uy + (a - b - c + d) * ux * uy
}

/// 3-octave fractal value noise in [0, 1] (third coordinate for time).
pub fn fbm3(x: f32, y: f32, t: f32, seed: u32) -> f32 {
    // Slide octaves through time by sampling noise2 at time-offset positions;
    // cheaper than true 3D noise and visually equivalent at keyboard scale.
    let mut acc = 0.0;
    let mut amp = 0.5;
    let mut fx = x;
    let mut fy = y;
    let mut ft = t;
    let mut norm = 0.0;
    for o in 0..3 {
        acc += amp * noise2(fx + ft, fy - ft * 0.7, seed.wrapping_add(o * 131));
        norm += amp;
        amp *= 0.5;
        fx *= 2.03;
        fy *= 2.03;
        ft *= 1.7;
    }
    acc / norm
}
