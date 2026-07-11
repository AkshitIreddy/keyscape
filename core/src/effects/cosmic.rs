//! Cosmic — deep-space scenes: meteor showers, pulsars, supernovae, black
//! holes and the solar wind.

use super::*;
use crate::color::{smoothstep, Col};
use crate::math::{fbm3, noise2, Rng};
use crate::params::{get_f32, ParamSpec};
use std::f32::consts::{PI, TAU};

pub fn effects() -> Vec<EffectInfo> {
    vec![
        EffectInfo {
            id: "meteor_storm",
            name: "Meteor Storm",
            category: "Cosmic",
            blurb: "Meteors streak in on shallow angles and shatter into sparks on impact.",
            needs_input: false,
            default_palette: "ember",
            extras: || {
                vec![
                    ParamSpec::slider("rate", "Meteor rate", 0.2, 3.0, 0.1, 1.0),
                    dir_param("down"),
                ]
            },
            make: |l, seed| {
                Box::new(MeteorStorm {
                    seed: seed as u32,
                    rng: Rng::new(seed),
                    heat: vec![0.0; l.keys.len()],
                    meteors: Vec::new(),
                    sparks: Vec::new(),
                })
            },
        },
        EffectInfo {
            id: "pulsar",
            name: "Pulsar",
            category: "Cosmic",
            blurb: "A wandering neutron star sweeps a lighthouse beam over a faint starfield.",
            needs_input: false,
            default_palette: "glacier",
            extras: || {
                vec![
                    ParamSpec::slider("rotation", "Rotation rate", 0.05, 1.2, 0.05, 0.3),
                    ParamSpec::slider("beam_width", "Beam width", 8.0, 60.0, 1.0, 24.0),
                ]
            },
            make: |_, seed| {
                Box::new(Pulsar { seed: seed as u32, angle: 0.0, half_phase: 0.0, ring_r: -1.0 })
            },
        },
        EffectInfo {
            id: "supernova_cycle",
            name: "Supernova Cycle",
            category: "Cosmic",
            blurb: "Stars swell, destabilize, collapse and detonate — leaving twinkling remnants.",
            needs_input: false,
            default_palette: "inferno",
            extras: || {
                vec![
                    ParamSpec::slider("cycle", "Cycle length", 6.0, 30.0, 1.0, 14.0),
                    ParamSpec::slider("remnant", "Remnant life", 2.0, 15.0, 0.5, 6.0),
                ]
            },
            make: |l, seed| {
                let mut rng = Rng::new(seed);
                let sx = rng.range(l.aspect * 0.2, l.aspect * 0.8);
                let sy = rng.range(0.25, 0.75);
                Box::new(Supernova {
                    seed: seed as u32,
                    rng,
                    phase: 0,
                    pt: 0.0,
                    sx,
                    sy,
                    ring_r: 0.0,
                    remnant: vec![0.0; l.keys.len()],
                })
            },
        },
        EffectInfo {
            id: "constellation",
            name: "Constellation",
            category: "Cosmic",
            blurb: "Twinkling stars joined by faint lines; a new figure is drawn every so often.",
            needs_input: false,
            default_palette: "midnight",
            extras: || {
                vec![
                    ParamSpec::slider("stars", "Star count", 5.0, 12.0, 1.0, 8.0),
                    ParamSpec::slider("interval", "Redraw interval", 8.0, 60.0, 1.0, 20.0),
                ]
            },
            make: |_, seed| {
                Box::new(Constellation {
                    seed: seed as u32,
                    rng: Rng::new(seed),
                    cur: ConstSet { stars: Vec::new(), lines: Vec::new() },
                    old: ConstSet { stars: Vec::new(), lines: Vec::new() },
                    fade: 1.0,
                    age: 0.0,
                    flare_i: usize::MAX,
                    flare_t: 99.0,
                })
            },
        },
        EffectInfo {
            id: "black_hole",
            name: "Black Hole",
            category: "Cosmic",
            blurb: "An event horizon swallows light while its accretion ring orbits and jets flare.",
            needs_input: false,
            default_palette: "ultraviolet",
            extras: || {
                vec![
                    ParamSpec::slider("horizon", "Horizon size", 0.1, 0.45, 0.01, 0.22),
                    ParamSpec::slider("ring", "Ring brightness", 0.2, 2.0, 0.05, 1.0),
                ]
            },
            make: |_, seed| {
                Box::new(BlackHole { seed: seed as u32, rng: Rng::new(seed), jet_t: 10.0 })
            },
        },
        EffectInfo {
            id: "solar_wind",
            name: "Solar Wind",
            category: "Cosmic",
            blurb: "Charged particle ribbons curl across the board; flare fronts surge through.",
            needs_input: false,
            default_palette: "aurora",
            extras: || {
                vec![
                    ParamSpec::slider("density", "Stream density", 6.0, 50.0, 1.0, 22.0),
                    ParamSpec::slider("flare_rate", "Flares per minute", 0.5, 8.0, 0.5, 2.0),
                    dir_param("right"),
                ]
            },
            make: |l, seed| {
                Box::new(SolarWind {
                    seed: seed as u32,
                    rng: Rng::new(seed),
                    parts: Vec::new(),
                    field: vec![0.0; l.keys.len()],
                    fhue: vec![0.0; l.keys.len()],
                    flare_u: 1.0,
                })
            },
        },
    ]
}

/// Gaussian falloff from a squared distance, radius `r`.
fn puff(d2: f32, r: f32) -> f32 {
    (-d2 / (2.0 * r * r)).exp()
}

/// Wrap an angle to [-PI, PI].
fn wrap_pi(x: f32) -> f32 {
    (x + PI).rem_euclid(TAU) - PI
}

/// Distance from point (px, py) to segment (ax, ay)-(bx, by).
fn seg_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let abx = bx - ax;
    let aby = by - ay;
    let l2 = abx * abx + aby * aby;
    let t = if l2 <= 1e-9 { 0.0 } else { (((px - ax) * abx + (py - ay) * aby) / l2).clamp(0.0, 1.0) };
    let qx = ax + abx * t;
    let qy = ay + aby * t;
    ((px - qx) * (px - qx) + (py - qy) * (py - qy)).sqrt()
}

// ---------------------------------------------------------------- Meteor Storm

struct Meteor {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

struct Spark {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max: f32,
}

struct MeteorStorm {
    seed: u32,
    rng: Rng,
    /// Per-key cooling trail left behind meteor heads.
    heat: Vec<f32>,
    meteors: Vec<Meteor>,
    sparks: Vec<Spark>,
}

impl MeteorStorm {
    fn shatter(&mut self, x: f32, y: f32, vx: f32, vy: f32) {
        let n = 3 + self.rng.below(3); // 3..=5 sparks
        let vert = vy.abs() > vx.abs();
        for _ in 0..n {
            let s = self.rng.range(0.45, 1.4) * if self.rng.chance(0.5) { 1.0 } else { -1.0 };
            let back = -self.rng.range(0.15, 0.55);
            let (svx, svy) = if vert { (s, back * vy.signum()) } else { (back * vx.signum(), s) };
            let life = self.rng.range(0.22, 0.5);
            self.sparks.push(Spark { x, y, vx: svx, vy: svy, life, max: life });
        }
    }
}

impl Effect for MeteorStorm {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let rate = get_f32(ctx.params, "rate", 1.0);
        let (dx, dy) = dir_vec(ctx.params);
        let a = ctx.layout.aspect;

        // Trails cool off.
        let decay = (-ctx.dt * 2.4).exp();
        for h in self.heat.iter_mut() {
            *h *= decay;
        }

        // Spawn rate waxes and wanes in slow waves.
        let wave = 0.30 + 0.70 * (0.5 + 0.5 * (ctx.t * 0.16 + self.seed as f32 * 0.13).sin());
        if self.meteors.len() < 14 && self.rng.chance(ctx.dt * rate * 2.2 * wave) {
            let spd = self.rng.range(0.95, 1.55);
            let slant = self.rng.range(0.35, 0.75) * if self.rng.chance(0.5) { 1.0 } else { -1.0 };
            let (x, y, vx, vy) = if dy.abs() > 0.5 {
                // steep fall with a sideways drift
                let vx = slant * spd;
                let x = self.rng.range(0.05, a - 0.05) - slant * 0.6;
                (x, if dy > 0.0 { -0.10 } else { 1.10 }, vx, dy * spd)
            } else {
                // shallow horizontal streak with a slight dip
                let vy = slant * spd * 0.55;
                let y = self.rng.range(0.08, 0.92) - slant * 0.3;
                (if dx > 0.0 { -0.10 } else { a + 0.10 }, y, dx * spd, vy)
            };
            self.meteors.push(Meteor { x, y, vx, vy });
        }

        // Move meteors, deposit heat, shatter at the far edge.
        let mut i = 0;
        while i < self.meteors.len() {
            let (mx, my, mvx, mvy) = {
                let m = &mut self.meteors[i];
                m.x += m.vx * ctx.dt;
                m.y += m.vy * ctx.dt;
                (m.x, m.y, m.vx, m.vy)
            };
            for (ki, k) in ctx.layout.keys.iter().enumerate() {
                let d2 = (k.cx - mx) * (k.cx - mx) + (k.cy - my) * (k.cy - my);
                if d2 < 0.09 {
                    let g = puff(d2, 0.085);
                    if g > self.heat[ki] {
                        self.heat[ki] = g;
                    }
                }
            }
            let vert = mvy.abs() > mvx.abs();
            let hit_far = if vert {
                (mvy > 0.0 && my > 0.98) || (mvy < 0.0 && my < 0.02)
            } else {
                (mvx > 0.0 && mx > a - 0.02) || (mvx < 0.0 && mx < 0.02)
            };
            if hit_far {
                let (sx, sy) = (mx.clamp(0.05, a - 0.05), my.clamp(0.04, 0.96));
                self.shatter(sx, sy, mvx, mvy);
                self.meteors.swap_remove(i);
            } else if mx < -0.25 || mx > a + 0.25 || my < -0.25 || my > 1.25 {
                self.meteors.swap_remove(i);
            } else {
                i += 1;
            }
        }

        // Sparks scatter and die quickly.
        let mut i = 0;
        while i < self.sparks.len() {
            let s = &mut self.sparks[i];
            s.x += s.vx * ctx.dt;
            s.y += s.vy * ctx.dt;
            s.vx *= 1.0 - 1.6 * ctx.dt;
            s.life -= ctx.dt;
            if s.life <= 0.0 {
                self.sparks.swap_remove(i);
            } else {
                i += 1;
            }
        }

        // Cooling trails: hot palette end fades toward the dark end.
        for (ki, k) in ctx.layout.keys.iter().enumerate() {
            let h = self.heat[ki];
            if h > 0.004 {
                out.set(k.led, ctx.palette.sample_clamped(0.12 + 0.75 * h).scale(h));
            }
        }

        // Hot heads on top.
        for m in &self.meteors {
            for k in &ctx.layout.keys {
                let d2 = (k.cx - m.x) * (k.cx - m.x) + (k.cy - m.y) * (k.cy - m.y);
                if d2 < 0.07 {
                    let g = puff(d2, 0.075);
                    out.add(k.led, ctx.palette.sample_clamped(0.95).scale(g).add(Col::WHITE.scale(0.3 * g * g)));
                }
            }
        }

        // Impact sparks.
        for s in &self.sparks {
            let e = s.life / s.max;
            for k in &ctx.layout.keys {
                let d2 = (k.cx - s.x) * (k.cx - s.x) + (k.cy - s.y) * (k.cy - s.y);
                if d2 < 0.04 {
                    let g = puff(d2, 0.055) * e;
                    out.add(k.led, ctx.palette.sample_clamped(0.85).scale(g * 1.1));
                }
            }
        }
    }
}

// ---------------------------------------------------------------- Pulsar

struct Pulsar {
    seed: u32,
    angle: f32,
    /// Accumulated half-revolutions; each wrap launches a radial pulse ring.
    half_phase: f32,
    /// Active pulse ring radius, negative when idle.
    ring_r: f32,
}

impl Effect for Pulsar {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let rot = get_f32(ctx.params, "rotation", 0.3);
        let bw = get_f32(ctx.params, "beam_width", 24.0).to_radians();
        let a = ctx.layout.aspect;
        let t = ctx.t;

        // The star wanders slowly near the board center.
        let sx = a * 0.5 + (noise2(t * 0.05, 7.3, self.seed) - 0.5) * 1.0;
        let sy = 0.5 + (noise2(t * 0.045, 21.7, self.seed) - 0.5) * 0.5;

        self.angle += ctx.dt * rot * TAU;
        self.half_phase += ctx.dt * rot * 2.0;
        if self.half_phase >= 1.0 {
            self.half_phase -= self.half_phase.floor();
            self.ring_r = 0.0;
        }
        if self.ring_r >= 0.0 {
            self.ring_r += ctx.dt * 1.5;
            if self.ring_r > 3.0 {
                self.ring_r = -1.0;
            }
        }

        let pulse = 0.85 + 0.15 * (t * 3.1).sin();
        for (ki, k) in ctx.layout.keys.iter().enumerate() {
            // sparse background starfield
            let h = noise2(ki as f32 * 7.13, 4.7, self.seed);
            let mut c = Col::BLACK;
            if h > 0.72 {
                let h2 = noise2(ki as f32 * 7.13, 55.5, self.seed);
                let tw = 0.5 + 0.5 * (t * (0.8 + 3.0 * h2) + h * 40.0).sin();
                c = ctx.palette.sample(0.15 + 0.5 * h2).scale(0.04 + 0.09 * tw);
            }

            let dxk = k.cx - sx;
            let dyk = k.cy - sy;
            let d2 = dxk * dxk + dyk * dyk;
            let dist = d2.sqrt();
            let ang = dyk.atan2(dxk);
            let att = (1.0 - 0.30 * (dist / 2.7)).max(0.2);

            // main beam and the fainter counter-beam
            let b1 = smoothstep(bw, bw * 0.25, wrap_pi(ang - self.angle).abs());
            let b2 = smoothstep(bw, bw * 0.25, wrap_pi(ang - self.angle - PI).abs()) * 0.45;
            let beam = (b1 + b2).min(1.0) * att * pulse;
            if beam > 0.01 {
                c = c.max(ctx.palette.sample_clamped(0.5 + 0.45 * beam).scale(beam));
            }

            // radial pulse ring between sweeps
            if self.ring_r >= 0.0 {
                let band = puff((dist - self.ring_r) * (dist - self.ring_r), 0.08);
                let fadeout = (1.0 - self.ring_r / 2.8).max(0.0);
                if band > 0.02 {
                    c = c.max(ctx.palette.sample_clamped(0.55).scale(band * fadeout * 0.85));
                }
            }

            // the star itself
            let core = puff(d2, 0.09);
            if core > 0.02 {
                c = c.max(ctx.palette.sample_clamped(0.92).scale(core));
                c = c.add(Col::WHITE.scale(core * core * 0.4));
            }

            out.set(k.led, c);
        }
    }
}

// ---------------------------------------------------------------- Supernova Cycle

struct Supernova {
    seed: u32,
    rng: Rng,
    /// 0 swell, 1 flicker, 2 collapse, 3 shockwave, 4 quiet
    phase: u8,
    pt: f32,
    sx: f32,
    sy: f32,
    ring_r: f32,
    /// Per-key turbulent remnant glow deposited by the shockwave.
    remnant: Vec<f32>,
}

impl Effect for Supernova {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let cycle = get_f32(ctx.params, "cycle", 14.0).max(4.0);
        let rem_life = get_f32(ctx.params, "remnant", 6.0).max(1.0);
        let a = ctx.layout.aspect;
        let t = ctx.t;

        let decay = (-ctx.dt * 2.2 / rem_life).exp();
        for r in self.remnant.iter_mut() {
            *r *= decay;
        }

        let swell = cycle * 0.32;
        let flick = cycle * 0.22;
        let collapse = (cycle * 0.05).clamp(0.3, 0.9);
        let quiet = cycle * 0.08;

        self.pt += ctx.dt;
        match self.phase {
            0 if self.pt > swell => {
                self.phase = 1;
                self.pt = 0.0;
            }
            1 if self.pt > flick => {
                self.phase = 2;
                self.pt = 0.0;
            }
            2 if self.pt > collapse => {
                self.phase = 3;
                self.pt = 0.0;
                self.ring_r = 0.0;
            }
            3 => {
                self.ring_r += ctx.dt * 2.1;
                if self.ring_r > 3.2 {
                    self.phase = 4;
                    self.pt = 0.0;
                }
            }
            4 if self.pt > quiet => {
                self.phase = 0;
                self.pt = 0.0;
                self.sx = self.rng.range(a * 0.14, a * 0.86);
                self.sy = self.rng.range(0.22, 0.78);
            }
            _ => {}
        }

        // Star envelope this frame: (radius, brightness, white-core amount).
        let (sr, sb, sw) = match self.phase {
            0 => {
                let u = (self.pt / swell).min(1.0);
                let g = u * u * (3.0 - 2.0 * u);
                (0.05 + 0.30 * g, 0.25 + 0.60 * g, 0.05 + 0.15 * g)
            }
            1 => {
                let n = noise2(t * 6.5, 3.3, self.seed);
                let dip = smoothstep(0.25, 0.65, noise2(t * 2.2, 9.1, self.seed));
                ((0.34 + 0.035 * (t * 9.0).sin()), (0.35 + 0.75 * n) * (0.55 + 0.45 * dip), 0.1 + 0.3 * n)
            }
            2 => {
                let u = (self.pt / collapse).min(1.0);
                let s = (1.0 - u) * (1.0 - u);
                // shell shrinks and dims while the core heats white
                (0.04 + 0.30 * s, 0.35 + 0.45 * s, 0.3 + 0.7 * u)
            }
            _ => (0.0, 0.0, 0.0),
        };

        let fadeout = (1.0 - self.ring_r / 3.2).max(0.0);
        let flash = if self.phase == 3 { (-self.pt * 2.6).exp() } else { 0.0 };
        for (ki, k) in ctx.layout.keys.iter().enumerate() {
            let dx = k.cx - self.sx;
            let dy = k.cy - self.sy;
            let d2 = dx * dx + dy * dy;

            // remnant bed: turbulent hue, twinkling brightness
            let v = self.remnant[ki];
            let mut c = Col::BLACK;
            if v > 0.004 {
                let tw = 0.55 + 0.45 * noise2(ki as f32 * 5.3, t * 2.8, self.seed ^ 0x2F);
                let hue = 0.18 + 0.30 * noise2(ki as f32 * 1.7, t * 0.22, self.seed ^ 0x91);
                c = ctx.palette.sample_clamped(hue + 0.30 * v).scale(v * tw);
            }

            if sr > 0.0 {
                let g = puff(d2, sr * 0.55);
                if g > 0.01 {
                    let star = ctx
                        .palette
                        .sample_clamped(0.40 + 0.45 * g)
                        .scale(g * sb)
                        .add(Col::WHITE.scale(g * g * g * sw * sb));
                    c = c.max(star);
                }
            }

            if self.phase == 3 {
                let d = d2.sqrt();
                if flash > 0.02 {
                    let g = puff(d2, 0.26);
                    c = c.max(
                        ctx.palette.sample_clamped(0.85).scale(g * flash * 1.2).add(Col::WHITE.scale(g * flash * 0.5)),
                    );
                }
                let band = puff((d - self.ring_r) * (d - self.ring_r), 0.085);
                if band > 0.02 {
                    let ri = band * fadeout;
                    c = c.max(
                        ctx.palette.sample_clamped(0.60 + 0.30 * band).scale(ri * 1.25).add(Col::WHITE.scale(ri * ri * 0.35)),
                    );
                    let dep = ri * 0.85;
                    if dep > self.remnant[ki] {
                        self.remnant[ki] = dep;
                    }
                }
            }

            out.set(k.led, c);
        }
    }
}

// ---------------------------------------------------------------- Constellation

struct CStar {
    key: usize,
    ph: f32,
    fr: f32,
}

struct ConstSet {
    stars: Vec<CStar>,
    /// Per-key intensity of the thin connecting lines.
    lines: Vec<f32>,
}

struct Constellation {
    seed: u32,
    rng: Rng,
    cur: ConstSet,
    old: ConstSet,
    /// Crossfade: 0 = old figure, 1 = current figure fully in.
    fade: f32,
    age: f32,
    flare_i: usize,
    flare_t: f32,
}

impl Constellation {
    fn generate(&mut self, layout: &Layout, count: usize) -> ConstSet {
        let n_keys = layout.keys.len();
        let count = count.clamp(3, 14).min(n_keys);

        // Pick well-separated star keys (relaxing if the board fills up).
        let mut picked: Vec<usize> = Vec::new();
        let mut tries = 0;
        while picked.len() < count && tries < 500 {
            tries += 1;
            let c = self.rng.below(n_keys);
            if picked.contains(&c) {
                continue;
            }
            let kc = &layout.keys[c];
            let ok = picked.iter().all(|&p| {
                let k = &layout.keys[p];
                (k.cx - kc.cx) * (k.cx - kc.cx) + (k.cy - kc.cy) * (k.cy - kc.cy) > 0.38 * 0.38
            });
            if ok {
                picked.push(c);
            }
        }
        while picked.len() < count {
            let c = self.rng.below(n_keys);
            if !picked.contains(&c) {
                picked.push(c);
            }
        }

        // Connect the stars with a minimum spanning tree (Prim).
        let m = picked.len();
        let mut in_tree = vec![false; m];
        in_tree[0] = true;
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for _ in 1..m {
            let mut best = (0usize, 0usize, f32::MAX);
            for i in 0..m {
                if !in_tree[i] {
                    continue;
                }
                for j in 0..m {
                    if in_tree[j] {
                        continue;
                    }
                    let ka = &layout.keys[picked[i]];
                    let kb = &layout.keys[picked[j]];
                    let d = (ka.cx - kb.cx) * (ka.cx - kb.cx) + (ka.cy - kb.cy) * (ka.cy - kb.cy);
                    if d < best.2 {
                        best = (i, j, d);
                    }
                }
            }
            in_tree[best.1] = true;
            edges.push((picked[best.0], picked[best.1]));
        }

        // Rasterize the thin edges into a per-key map once per figure.
        let mut lines = vec![0.0f32; n_keys];
        for &(ea, eb) in &edges {
            let ka = &layout.keys[ea];
            let kb = &layout.keys[eb];
            for (i, k) in layout.keys.iter().enumerate() {
                let d = seg_dist(k.cx, k.cy, ka.cx, ka.cy, kb.cx, kb.cy);
                let v = smoothstep(0.065, 0.012, d);
                if v > lines[i] {
                    lines[i] = v;
                }
            }
        }

        let stars = picked
            .iter()
            .map(|&key| CStar { key, ph: self.rng.range(0.0, TAU), fr: self.rng.range(0.6, 1.7) })
            .collect();
        ConstSet { stars, lines }
    }
}

impl Effect for Constellation {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let count = get_f32(ctx.params, "stars", 8.0).round() as usize;
        let interval = get_f32(ctx.params, "interval", 20.0).max(4.0);
        let t = ctx.t;

        if self.cur.stars.is_empty() {
            self.cur = self.generate(ctx.layout, count);
        }
        self.age += ctx.dt;
        if self.age > interval && self.fade >= 1.0 {
            let fresh = self.generate(ctx.layout, count);
            self.old = std::mem::replace(&mut self.cur, fresh);
            self.fade = 0.0;
            self.age = 0.0;
            self.flare_t = 99.0;
        }
        if self.fade < 1.0 {
            self.fade = (self.fade + ctx.dt / 2.5).min(1.0);
        }

        // Occasional flare of one star in the current figure.
        self.flare_t += ctx.dt;
        if self.fade >= 1.0 && self.flare_t > 3.0 && self.rng.chance(ctx.dt * 0.12) {
            self.flare_i = self.rng.below(self.cur.stars.len());
            self.flare_t = 0.0;
        }
        let fe = (1.0 - self.flare_t / 1.1).max(0.0);
        let flare_e = fe * fe;

        let wn = self.fade;
        let wo = 1.0 - self.fade;

        // Night-sky bed plus the crossfading line work.
        for (i, k) in ctx.layout.keys.iter().enumerate() {
            let lv = self.cur.lines.get(i).copied().unwrap_or(0.0) * wn
                + self.old.lines.get(i).copied().unwrap_or(0.0) * wo;
            let bg = 0.03 + 0.025 * noise2(k.cx * 1.5, k.cy * 1.5 + t * 0.05, self.seed);
            let mut c = ctx.palette.sample_clamped(0.30).scale(bg);
            if lv > 0.003 {
                let shimmer = 0.75 + 0.25 * (t * 1.1 + k.cx * 3.0 + k.cy * 2.0).sin();
                c = c.max(ctx.palette.sample_clamped(0.55).scale(lv * shimmer * 0.28));
            }
            out.set(k.led, c);
        }

        // Stars: gentle twinkle, one occasionally flaring with a halo.
        for (si, st) in self.cur.stars.iter().enumerate() {
            let k = &ctx.layout.keys[st.key];
            let tw = 0.55 + 0.45 * (t * st.fr + st.ph).sin();
            let mut b = (0.30 + 0.70 * tw) * wn;
            let mut white = 0.10 * tw * wn;
            if si == self.flare_i && flare_e > 0.0 {
                b += 2.2 * flare_e;
                white += 0.55 * flare_e;
                for &nb in &ctx.layout.neighbors[st.key] {
                    out.max(ctx.layout.keys[nb].led, ctx.palette.sample_clamped(0.78).scale(0.55 * flare_e));
                }
            }
            out.max(k.led, ctx.palette.sample_clamped(0.88).scale(b.min(1.4)).add(Col::WHITE.scale(white)));
        }
        for st in &self.old.stars {
            let k = &ctx.layout.keys[st.key];
            let tw = 0.55 + 0.45 * (t * st.fr + st.ph).sin();
            let b = (0.30 + 0.70 * tw) * wo;
            out.max(k.led, ctx.palette.sample_clamped(0.88).scale(b).add(Col::WHITE.scale(0.10 * tw * wo)));
        }
    }
}

// ---------------------------------------------------------------- Black Hole

struct BlackHole {
    seed: u32,
    rng: Rng,
    /// Seconds since the last polar-jet flash started.
    jet_t: f32,
}

impl Effect for BlackHole {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let rh = get_f32(ctx.params, "horizon", 0.22);
        let rb = get_f32(ctx.params, "ring", 1.0);
        let a = ctx.layout.aspect;
        let t = ctx.t;

        // The singularity drifts very slowly around the middle of the board.
        let hx = a * 0.5 + (noise2(t * 0.02, 3.1, self.seed) - 0.5) * (a * 0.55);
        let hy = 0.5 + (noise2(t * 0.018, 9.4, self.seed) - 0.5) * 0.6;
        let ring_r = rh + 0.10;

        // Rare polar jet, perpendicular to the accretion ring.
        self.jet_t += ctx.dt;
        if self.jet_t > 1.5 && self.rng.chance(ctx.dt * 0.05) {
            self.jet_t = 0.0;
        }
        let jet_e = (1.0 - self.jet_t * 1.4).max(0.0);

        for k in &ctx.layout.keys {
            let dx = k.cx - hx;
            let dy = k.cy - hy;
            let d2 = dx * dx + dy * dy;
            let d = d2.sqrt();

            // Background dust; sample position bends toward the hole the
            // closer the light passes to it, and the horizon eats it.
            let bend = (0.28 * rh / (d - rh * 0.6).max(0.05)).min(0.5);
            let field = fbm3(k.cx * 0.8, k.cy * 0.8, t * 0.03, self.seed);
            let vis = smoothstep(rh * 0.5, rh * 1.15, d);
            let mut c = ctx
                .palette
                .sample_clamped(field * 0.45 + bend)
                .scale(0.30 * vis * (0.55 + 0.45 * field));

            // Accretion ring with orbiting lumps and a doppler-bright side.
            let g = puff((d - ring_r) * (d - ring_r), 0.075);
            if g > 0.01 {
                let ang = dy.atan2(dx);
                let lump = 0.5 + 0.5 * ((ang * 3.0 - t * 2.4).sin() * 0.6 + (ang * 7.0 + t * 3.7).sin() * 0.4);
                let dopp = 0.7 + 0.3 * (ang - t * 0.2).sin();
                let ri = g * rb * (0.35 + 0.65 * lump) * dopp;
                c = c.max(ctx.palette.sample_clamped(0.72 + 0.22 * lump).scale(ri));
                c = c.add(Col::WHITE.scale(ri * ri * 0.18));
            }

            // Polar jet flash: a tight vertical beam out of both poles.
            if jet_e > 0.0 {
                let j = (-(dx * dx) / (2.0 * 0.055 * 0.055)).exp()
                    * smoothstep(ring_r * 0.4, ring_r * 0.9, dy.abs())
                    * (-(dy.abs() - ring_r) * 0.9).exp().min(1.0);
                if j > 0.01 {
                    c = c.add(ctx.palette.sample_clamped(0.95).scale(j * jet_e * 1.1));
                    c = c.add(Col::WHITE.scale(j * jet_e * 0.25));
                }
            }

            out.set(k.led, c);
        }
    }
}

// ---------------------------------------------------------------- Solar Wind

struct WindP {
    x: f32,
    y: f32,
    sp: f32,
    hue: f32,
}

struct SolarWind {
    seed: u32,
    rng: Rng,
    parts: Vec<WindP>,
    /// Per-key ribbon trail left by passing particles.
    field: Vec<f32>,
    /// Palette offset of whichever particle last dominated a key.
    fhue: Vec<f32>,
    /// Flare front progress across the board; >= 1 means idle.
    flare_u: f32,
}

impl SolarWind {
    fn spawn(&mut self, a: f32, dx: f32, dy: f32) {
        if self.parts.len() >= 70 {
            return;
        }
        self.parts.push(WindP {
            x: if dx > 0.5 {
                -0.12
            } else if dx < -0.5 {
                a + 0.12
            } else {
                self.rng.range(-0.05, a + 0.05)
            },
            y: if dy > 0.5 {
                -0.12
            } else if dy < -0.5 {
                1.12
            } else {
                self.rng.range(-0.05, 1.05)
            },
            sp: self.rng.range(0.7, 1.25),
            hue: self.rng.range(0.0, 0.22),
        });
    }
}

impl Effect for SolarWind {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let density = get_f32(ctx.params, "density", 22.0);
        let frate = get_f32(ctx.params, "flare_rate", 2.0);
        let (dxs, dys) = dir_vec(ctx.params);
        let (px, py) = (-dys, dxs);
        let a = ctx.layout.aspect;
        let t = ctx.t;

        // Ribbons decay; density breathes slowly.
        let decay = (-ctx.dt * 3.0).exp();
        for f in self.field.iter_mut() {
            *f *= decay;
        }
        let breathe = 0.65 + 0.35 * (t * 0.11 + self.seed as f32 * 0.7).sin();
        let target = (density * breathe).round() as usize;

        // Flare timing: Poisson-triggered front that crosses in ~1 second.
        if self.flare_u >= 1.0 {
            if self.rng.chance(ctx.dt * frate / 60.0) {
                self.flare_u = 0.0;
                for _ in 0..6 {
                    self.spawn(a, dxs, dys);
                }
            }
        } else {
            self.flare_u += ctx.dt / 1.15;
        }
        let flare_env = if self.flare_u < 1.0 { 1.0 - self.flare_u } else { 0.0 };

        if self.parts.len() < target && self.rng.chance(ctx.dt * 12.0) {
            self.spawn(a, dxs, dys);
        }

        // Advect particles through a shared curl field.
        let boost = 1.0 + 1.2 * flare_env;
        let mut i = 0;
        while i < self.parts.len() {
            let p = &mut self.parts[i];
            let curl = (noise2(p.x * 1.5 + t * 0.2, p.y * 1.5 - t * 0.15, self.seed) - 0.5) * 1.5;
            let v = 0.85 * p.sp * boost;
            p.x += (dxs + px * curl) * v * ctx.dt;
            p.y += (dys + py * curl) * v * ctx.dt;
            if p.x < -0.2 || p.x > a + 0.2 || p.y < -0.2 || p.y > 1.2 {
                self.parts.swap_remove(i);
            } else {
                i += 1;
            }
        }

        // Deposit ribbon trails.
        let dep_boost = 1.0 + 0.9 * flare_env;
        for p in &self.parts {
            for (ki, k) in ctx.layout.keys.iter().enumerate() {
                let d2 = (k.cx - p.x) * (k.cx - p.x) + (k.cy - p.y) * (k.cy - p.y);
                if d2 < 0.09 {
                    let g = (puff(d2, 0.095) * dep_boost).min(1.2);
                    if g > self.field[ki] {
                        self.field[ki] = g;
                        self.fhue[ki] = p.hue;
                    }
                }
            }
        }

        let horizontal = dxs.abs() > 0.5;
        let span = if horizontal { a } else { 1.0 };
        let front = -0.3 + self.flare_u * (span + 0.6);
        for (ki, k) in ctx.layout.keys.iter().enumerate() {
            // faint plasma haze drifting with the wind
            let bgn = noise2(k.cx * 1.4 - t * 0.45 * dxs, k.cy * 1.4 - t * 0.45 * dys, self.seed);
            let mut c = ctx.palette.sample(0.08 + 0.22 * bgn).scale(0.04 + 0.06 * bgn);

            let f = self.field[ki];
            if f > 0.005 {
                c = c.max(ctx.palette.sample_clamped(0.20 + self.fhue[ki] + 0.55 * f).scale(f));
                if f > 0.75 {
                    c = c.add(Col::WHITE.scale((f - 0.75) * 0.45));
                }
            }

            if flare_env > 0.0 {
                let along = if horizontal {
                    if dxs > 0.0 {
                        k.cx
                    } else {
                        a - k.cx
                    }
                } else if dys > 0.0 {
                    k.cy
                } else {
                    1.0 - k.cy
                };
                let g = puff((along - front) * (along - front), 0.15);
                c = c.add(ctx.palette.sample_clamped(0.88).scale(g * 0.95 + flare_env * 0.06));
                c = c.add(Col::WHITE.scale(g * 0.2));
            }

            out.set(k.led, c);
        }
    }
}
