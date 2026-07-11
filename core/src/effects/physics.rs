//! Physics — simulations you can *feel*: falling sand, buoyant wax, fields,
//! pendulums, orbits, reaction-diffusion and rippling cloth.

use super::*;
use crate::color::{smoothstep, Col};
use crate::layout::Key;
use crate::math::noise2;
use crate::params::{get_f32, get_str, ParamSpec};

pub fn effects() -> Vec<EffectInfo> {
    vec![
        EffectInfo {
            id: "sandfall",
            name: "Sandfall",
            category: "Physics",
            blurb: "Grains pour from the top, pile into colored strata, then melt away.",
            needs_input: false,
            default_palette: "copper",
            extras: || {
                vec![
                    ParamSpec::slider("pour", "Pour rate", 1.0, 20.0, 0.5, 6.0),
                    ParamSpec::slider("melt_speed", "Melt speed", 0.3, 4.0, 0.1, 1.2),
                ]
            },
            make: |_, seed| Box::new(Sandfall::new(seed)),
        },
        EffectInfo {
            id: "lava_lamp",
            name: "Lava Lamp",
            category: "Physics",
            blurb: "Wax blobs heat up at the base, rise, cool and sink back down.",
            needs_input: false,
            default_palette: "sunset",
            extras: || {
                vec![
                    ParamSpec::slider("blobs", "Blob count", 2.0, 6.0, 1.0, 4.0),
                    ParamSpec::slider("viscosity", "Viscosity", 0.0, 1.0, 0.05, 0.35),
                ]
            },
            make: |_, seed| Box::new(LavaLamp::new(seed)),
        },
        EffectInfo {
            id: "magnetic_poles",
            name: "Magnetic Poles",
            category: "Physics",
            blurb: "Iron filings trace the field of two wandering poles; polarity snaps.",
            needs_input: false,
            default_palette: "ultraviolet",
            extras: || {
                vec![
                    ParamSpec::slider("contrast", "Field contrast", 0.5, 2.5, 0.05, 1.2),
                    ParamSpec::slider("flip_rate", "Flip rate", 0.1, 1.5, 0.05, 0.4),
                ]
            },
            make: |_, seed| Box::new(MagneticPoles::new(seed)),
        },
        EffectInfo {
            id: "pendulum_wave",
            name: "Pendulum Wave",
            category: "Physics",
            blurb: "21 detuned pendulums weave patterns, snap into phase, and unravel.",
            needs_input: false,
            default_palette: "glacier",
            extras: || {
                vec![
                    ParamSpec::slider("spread", "Period spread", 0.2, 2.0, 0.05, 1.0),
                    ParamSpec::slider("bob", "Bob size", 0.06, 0.30, 0.01, 0.14),
                ]
            },
            make: |_, _| Box::new(PendulumWave),
        },
        EffectInfo {
            id: "chaos_pendulum",
            name: "Chaos Pendulum",
            category: "Physics",
            blurb: "A double pendulum scribbles glowing ink that never repeats.",
            needs_input: false,
            default_palette: "synthwave",
            extras: || {
                vec![
                    ParamSpec::slider("trail", "Trail life", 0.3, 4.0, 0.05, 1.6),
                    ParamSpec::slider("arms", "Arm brightness", 0.0, 1.0, 0.05, 0.35),
                ]
            },
            make: |layout, seed| Box::new(ChaosPendulum::new(layout, seed)),
        },
        EffectInfo {
            id: "gravity_wells",
            name: "Gravity Wells",
            category: "Physics",
            blurb: "Particles slingshot around two drifting attractors, flaring with speed.",
            needs_input: false,
            default_palette: "midnight",
            extras: || {
                vec![
                    ParamSpec::slider("particles", "Particles", 4.0, 24.0, 1.0, 14.0),
                    ParamSpec::slider("gravity", "Gravity", 0.3, 3.0, 0.05, 1.0),
                ]
            },
            make: |layout, seed| Box::new(GravityWells::new(layout, seed)),
        },
        EffectInfo {
            id: "reaction_diffusion",
            name: "Reaction Diffusion",
            category: "Physics",
            blurb: "Gray-Scott chemistry: living spots and stripes crawl across the keys.",
            needs_input: false,
            default_palette: "toxic",
            extras: || {
                vec![
                    ParamSpec::select("regime", "Regime", vec!["spots", "stripes", "mitosis"], "spots"),
                    ParamSpec::slider("iterations", "Sim speed", 1.0, 8.0, 1.0, 4.0),
                ]
            },
            make: |_, seed| Box::new(ReactionDiffusion::new(seed)),
        },
        EffectInfo {
            id: "cloth_ripple",
            name: "Cloth Ripple",
            category: "Physics",
            blurb: "Wind gusts roll pressure fronts across a rippling sheet.",
            needs_input: false,
            default_palette: "hologram",
            extras: || {
                vec![
                    ParamSpec::slider("damping", "Damping", 0.2, 6.0, 0.05, 1.5),
                    ParamSpec::slider("gusts", "Gust rate", 0.1, 2.5, 0.05, 0.7),
                ]
            },
            make: |_, seed| Box::new(ClothRipple::new(seed)),
        },
    ]
}

// ------------------------------------------------------------- grid helpers

const GW: usize = 21;
const GH: usize = 7;

/// Map a key to its cell on the coarse GW x GH sim grid by physical position.
fn cell_of(k: &Key, aspect: f32) -> (usize, usize) {
    let gx = (k.cx / aspect * GW as f32) as usize;
    let gy = (k.cy * GH as f32) as usize;
    (gx.min(GW - 1), gy.min(GH - 1))
}

/// Bilinear sample of a GW x GH scalar field at a key center.
fn sample_grid(g: &[f32], k: &Key, aspect: f32) -> f32 {
    let fx = (k.cx / aspect * GW as f32 - 0.5).clamp(0.0, (GW - 1) as f32);
    let fy = (k.cy * GH as f32 - 0.5).clamp(0.0, (GH - 1) as f32);
    let (x0, y0) = (fx as usize, fy as usize);
    let (x1, y1) = ((x0 + 1).min(GW - 1), (y0 + 1).min(GH - 1));
    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
    let a = g[y0 * GW + x0] + (g[y0 * GW + x1] - g[y0 * GW + x0]) * tx;
    let b = g[y1 * GW + x0] + (g[y1 * GW + x1] - g[y1 * GW + x0]) * tx;
    a + (b - a) * ty
}

/// Distance from point p to segment a-b (iso space).
fn seg_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 1e-9 { ((px - ax) * dx + (py - ay) * dy) / len2 } else { 0.0 };
    let t = t.clamp(0.0, 1.0);
    let (qx, qy) = (ax + dx * t, ay + dy * t);
    ((px - qx) * (px - qx) + (py - qy) * (py - qy)).sqrt()
}

// ---------------------------------------------------------------- Sandfall

const EMPTY: f32 = -1.0;

struct Sandfall {
    /// Palette position of the grain in each cell, or EMPTY.
    cells: [[f32; GW]; GH],
    /// "Just moved" heat per cell, for a falling sparkle.
    hot: [[f32; GW]; GH],
    acc: f32,
    spawn_acc: f32,
    /// Rows melted from the bottom; negative while not melting.
    melt: f32,
    scan_flip: bool,
}

impl Sandfall {
    fn new(_seed: u64) -> Sandfall {
        Sandfall {
            cells: [[EMPTY; GW]; GH],
            hot: [[0.0; GW]; GH],
            acc: 0.0,
            spawn_acc: 0.0,
            melt: -1.0,
            scan_flip: false,
        }
    }

    fn step(&mut self, pour: f32, melt_speed: f32, pal_pos: f32, rng: &mut Rng) {
        const DT: f32 = 1.0 / 30.0;
        if self.melt >= 0.0 {
            // Melt line sweeps up from the bottom, erasing rows.
            self.melt += melt_speed * DT * 3.5;
            let line = GH as f32 - self.melt;
            for gy in 0..GH {
                if gy as f32 + 0.5 >= line {
                    for gx in 0..GW {
                        self.cells[gy][gx] = EMPTY;
                    }
                }
            }
            if self.melt >= GH as f32 + 1.5 {
                self.melt = -1.0;
                self.spawn_acc = 0.0;
            }
            return;
        }

        // Pour new grains at random top columns, color frozen at spawn time.
        self.spawn_acc += pour * DT;
        while self.spawn_acc >= 1.0 {
            self.spawn_acc -= 1.0;
            let gx = rng.below(GW);
            if self.cells[0][gx] < 0.0 {
                self.cells[0][gx] = (pal_pos + rng.range(-0.03, 0.03)).clamp(0.0, 0.97);
                self.hot[0][gx] = 1.0;
            }
        }

        // Gravity pass, bottom-up so each grain moves at most once per step.
        // Alternate horizontal scan direction to avoid a drift bias.
        self.scan_flip = !self.scan_flip;
        for gy in (0..GH - 1).rev() {
            for i in 0..GW {
                let gx = if self.scan_flip { GW - 1 - i } else { i };
                let v = self.cells[gy][gx];
                if v < 0.0 {
                    continue;
                }
                if self.cells[gy + 1][gx] < 0.0 {
                    self.cells[gy + 1][gx] = v;
                    self.cells[gy][gx] = EMPTY;
                    self.hot[gy + 1][gx] = 1.0;
                    continue;
                }
                // Blocked below: try a diagonal slide.
                let side: i32 = if rng.chance(0.5) { 1 } else { -1 };
                for s in [side, -side] {
                    let nx = gx as i32 + s;
                    if nx >= 0 && (nx as usize) < GW && self.cells[gy + 1][nx as usize] < 0.0 {
                        self.cells[gy + 1][nx as usize] = v;
                        self.cells[gy][gx] = EMPTY;
                        self.hot[gy + 1][nx as usize] = 1.0;
                        break;
                    }
                }
            }
        }

        // When the board is ~85% full, start the melt.
        let filled = self.cells.iter().flatten().filter(|v| **v >= 0.0).count();
        if filled as f32 >= 0.85 * (GW * GH) as f32 {
            self.melt = 0.0;
        }
    }
}

impl Effect for Sandfall {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let pour = get_f32(ctx.params, "pour", 6.0);
        let melt_speed = get_f32(ctx.params, "melt_speed", 1.2);
        // The pour color drifts slowly through the palette => strata.
        let pal_pos = (ctx.t * 0.018).fract();

        self.acc += ctx.dt.min(0.25);
        while self.acc >= 1.0 / 30.0 {
            self.acc -= 1.0 / 30.0;
            self.step(pour, melt_speed, pal_pos, ctx.rng);
        }

        let hot_decay = (-ctx.dt * 5.0).exp();
        for row in self.hot.iter_mut() {
            for h in row.iter_mut() {
                *h *= hot_decay;
            }
        }

        let melt_line = GH as f32 - self.melt.max(0.0);
        for k in &ctx.layout.keys {
            let (gx, gy) = cell_of(k, ctx.layout.aspect);
            let v = self.cells[gy][gx];
            if v >= 0.0 {
                let hot = self.hot[gy][gx];
                let mut c = ctx.palette.sample_clamped(v).scale(0.65 + 0.55 * hot);
                if self.melt >= 0.0 {
                    // Rows near the melt line glow white-hot before vanishing.
                    let glow = smoothstep(1.4, 0.0, (melt_line - (gy as f32 + 0.5)).abs());
                    c = c.add(ctx.palette.sample_clamped(0.97).scale(glow * 0.9));
                }
                out.set(k.led, c);
            } else {
                // Faint dusty backdrop so empty air isn't dead black.
                let dust = noise2(gx as f32 * 1.7, gy as f32 * 1.7 + ctx.t * 0.6, 7);
                out.set(k.led, ctx.palette.sample_clamped(0.08).scale(0.04 + 0.05 * dust));
            }
        }
    }
}

// ---------------------------------------------------------------- Lava Lamp

struct Blob {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    heat: f32,
    r0: f32,
    phase: f32,
}

struct LavaLamp {
    seed: u32,
    blobs: Vec<Blob>,
}

impl LavaLamp {
    fn new(seed: u64) -> LavaLamp {
        let mut rng = Rng::new(seed);
        let blobs = (0..6)
            .map(|_| Blob {
                x: rng.range(0.4, 2.1),
                y: rng.range(0.3, 0.9),
                vx: 0.0,
                vy: 0.0,
                heat: rng.range(0.2, 0.8),
                r0: rng.range(0.22, 0.34),
                phase: rng.range(0.0, 6.28),
            })
            .collect();
        LavaLamp { seed: seed as u32, blobs }
    }
}

impl Effect for LavaLamp {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let count = (get_f32(ctx.params, "blobs", 4.0).round() as usize).clamp(2, 6);
        let visc = get_f32(ctx.params, "viscosity", 0.35);
        let dt = ctx.dt.min(0.05);
        let aspect = ctx.layout.aspect;
        let drag = 0.6 + 3.2 * visc;

        for (i, b) in self.blobs.iter_mut().take(count).enumerate() {
            // Heat exchange: hot plate at the bottom, cool glass at the top.
            b.heat += (b.y - 0.42) * 0.55 * dt;
            b.heat = b.heat.clamp(0.0, 1.0);
            // Buoyancy: hot wax rises (y is down), cold wax sinks.
            b.vy += (0.5 - b.heat) * 0.55 * dt;
            // Lazy sideways wander.
            b.vx += (noise2(ctx.t * 0.13, i as f32 * 9.1, self.seed) - 0.5) * 0.16 * dt;
            b.vx -= b.vx * drag * dt;
            b.vy -= b.vy * drag * dt;
            b.x += b.vx * dt;
            b.y += b.vy * dt;
            if b.x < 0.25 {
                b.x = 0.25;
                b.vx = b.vx.abs() * 0.5;
            }
            if b.x > aspect - 0.25 {
                b.x = aspect - 0.25;
                b.vx = -b.vx.abs() * 0.5;
            }
            if b.y < 0.10 {
                b.y = 0.10;
                b.vy = b.vy.abs() * 0.3;
            }
            if b.y > 0.94 {
                b.y = 0.94;
                b.vy = -b.vy.abs() * 0.3;
            }
        }

        for k in &ctx.layout.keys {
            // Metaball field: sum of gaussians with slowly pulsing radii.
            let mut field = 0.0;
            for b in self.blobs.iter().take(count) {
                let r = b.r0 * (1.0 + 0.22 * (ctx.t * 0.33 + b.phase).sin());
                let (dx, dy) = (k.cx - b.x, k.cy - b.y);
                field += (-(dx * dx + dy * dy) / (r * r)).exp();
            }
            let inside = smoothstep(0.52, 0.95, field);
            let halo = smoothstep(0.16, 0.52, field) * (1.0 - inside);
            // Warm glow pooled at the lamp base.
            let base = ctx.palette.sample_clamped(0.10).scale(0.03 + 0.09 * k.cy * k.cy);
            let t_pal = 0.30 + 0.65 * (field * 0.55).min(1.0);
            let c = ctx
                .palette
                .sample_clamped(t_pal)
                .scale(0.20 * halo + (0.45 + 0.55 * (field - 1.0).clamp(0.0, 1.0)) * inside);
            out.set(k.led, base.max(c));
        }
    }
}

// ----------------------------------------------------------- Magnetic Poles

struct MagneticPoles {
    seed: u32,
    polarity: f32,
    clock: f32,
    next_flip: f32,
    /// Ripple age since the last flip (starts large = no ripple).
    ripple_age: f32,
    ripple_x: f32,
    ripple_y: f32,
}

impl MagneticPoles {
    fn new(seed: u64) -> MagneticPoles {
        MagneticPoles {
            seed: seed as u32,
            polarity: 1.0,
            clock: 0.0,
            next_flip: 6.0,
            ripple_age: 99.0,
            ripple_x: 0.0,
            ripple_y: 0.0,
        }
    }
}

impl Effect for MagneticPoles {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let contrast = get_f32(ctx.params, "contrast", 1.2);
        let flip_rate = get_f32(ctx.params, "flip_rate", 0.4);
        let aspect = ctx.layout.aspect;

        // Two poles wander on independent smooth noise paths.
        let tt = ctx.t * 0.075;
        let p1x = (noise2(tt, 3.17, self.seed) * 0.85 + 0.07) * aspect;
        let p1y = noise2(tt, 17.7, self.seed) * 0.85 + 0.07;
        let p2x = (noise2(tt, 29.3, self.seed ^ 0xA5A5) * 0.85 + 0.07) * aspect;
        let p2y = noise2(tt, 41.9, self.seed ^ 0xA5A5) * 0.85 + 0.07;

        // Occasional polarity flip with a fast ripple from between the poles.
        self.clock += ctx.dt;
        self.ripple_age += ctx.dt;
        if self.clock >= self.next_flip {
            self.clock = 0.0;
            self.next_flip = ctx.rng.range(1.4, 3.2) / flip_rate.max(0.05);
            self.polarity = -self.polarity;
            self.ripple_age = 0.0;
            self.ripple_x = (p1x + p2x) * 0.5;
            self.ripple_y = (p1y + p2y) * 0.5;
        }
        let ripple_r = self.ripple_age * 3.4;
        let ripple_amp = (1.0 - self.ripple_age / 0.8).max(0.0);

        for k in &ctx.layout.keys {
            // Superposed field of a + and a - point charge.
            let mut bx = 0.0;
            let mut by = 0.0;
            for (px, py, q) in [(p1x, p1y, self.polarity), (p2x, p2y, -self.polarity)] {
                let (dx, dy) = (k.cx - px, k.cy - py);
                let d2 = dx * dx + dy * dy + 0.004;
                let inv = q / (d2 * d2.sqrt());
                bx += dx * inv;
                by += dy * inv;
            }
            let mag = (bx * bx + by * by).sqrt();
            // Field-line angle picks the hue; magnitude sets brightness.
            let angle = by.atan2(bx) / std::f32::consts::TAU + 0.5;
            let mut bright = (mag * 0.25).powf(0.45).clamp(0.0, 1.0).powf(contrast);
            // Grainy iron-filing texture.
            let grain = 0.8 + 0.2 * noise2(k.cx * 9.0, k.cy * 9.0, self.seed ^ 0x77);
            bright = (0.06 + 0.94 * bright) * grain;
            let mut c = ctx.palette.sample(angle + ctx.t * 0.008).scale(bright);
            if ripple_amp > 0.0 {
                let (dx, dy) = (k.cx - self.ripple_x, k.cy - self.ripple_y);
                let d = (dx * dx + dy * dy).sqrt();
                let ring = (-((d - ripple_r) / 0.14).powi(2)).exp() * ripple_amp;
                c = c.add(ctx.palette.sample_clamped(0.95).scale(ring * 0.9));
            }
            out.set(k.led, c);
        }
    }
}

// ------------------------------------------------------------ Pendulum Wave

struct PendulumWave;

impl Effect for PendulumWave {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let spread = get_f32(ctx.params, "spread", 1.0);
        let bob = get_f32(ctx.params, "bob", 0.14);

        // Classic pendulum wave: column i completes (N0 + spread*i) swings per
        // grand cycle, so everything realigns periodically.
        const CYCLE: f32 = 42.0;
        const N0: f32 = 22.0;
        let amp = 0.36;
        let mut ys = [0.0f32; GW];
        let mut phases = [0.0f32; GW];
        let (mut sum_c, mut sum_s) = (0.0f32, 0.0f32);
        for i in 0..GW {
            let w = std::f32::consts::TAU * (N0 + spread * i as f32) / CYCLE;
            let ph = w * ctx.t;
            phases[i] = ph;
            ys[i] = 0.5 + amp * ph.sin();
            sum_c += ph.cos();
            sum_s += ph.sin();
        }
        // Kuramoto order parameter: ~1 when all pendulums swing in phase.
        let order = (sum_c * sum_c + sum_s * sum_s).sqrt() / GW as f32;
        let flash = smoothstep(0.94, 0.995, order);

        for k in &ctx.layout.keys {
            let (gx, _) = cell_of(k, ctx.layout.aspect);
            let y = ys[gx];
            // Soft gaussian bob profile within the column.
            let g = (-((k.cy - y) / bob).powi(2)).exp();
            // Color rides the bob's vertical position through the palette.
            let pos = ((y - (0.5 - amp)) / (2.0 * amp)).clamp(0.0, 1.0);
            let bob_c = ctx.palette.sample_clamped(0.15 + 0.72 * pos);
            // Dim "string" trace above/below keeps columns readable.
            let string = 0.05 * smoothstep(0.5, 0.0, (k.cy - y).abs());
            let swing = 0.55 + 0.45 * (phases[gx] + std::f32::consts::FRAC_PI_2).sin().abs();
            let mut c = bob_c.scale(g * swing + string);
            if flash > 0.0 {
                c = c.add(ctx.palette.sample_clamped(0.92).scale(flash * 0.7));
            }
            out.set(k.led, c);
        }
    }
}

// ----------------------------------------------------------- Chaos Pendulum

struct ChaosPendulum {
    th1: f32,
    th2: f32,
    w1: f32,
    w2: f32,
    /// Per-key ink trail (decaying).
    heat: Vec<f32>,
}

impl ChaosPendulum {
    fn new(layout: &Layout, seed: u64) -> ChaosPendulum {
        let mut rng = Rng::new(seed);
        ChaosPendulum {
            th1: std::f32::consts::PI * rng.range(0.75, 1.05),
            th2: std::f32::consts::PI * rng.range(0.45, 0.95),
            w1: 0.0,
            w2: 0.0,
            heat: vec![0.0; layout.keys.len()],
        }
    }

    /// One physics substep (equal masses / lengths, angles from down-vertical).
    fn substep(&mut self, h: f32) {
        const G: f32 = 9.81;
        let d = self.th1 - self.th2;
        let den = 3.0 - (2.0 * d).cos();
        let a1 = (-3.0 * G * self.th1.sin()
            - G * (self.th1 - 2.0 * self.th2).sin()
            - 2.0 * d.sin() * (self.w2 * self.w2 + self.w1 * self.w1 * d.cos()))
            / den;
        let a2 = 2.0 * d.sin()
            * (2.0 * self.w1 * self.w1 + 2.0 * G * self.th1.cos() + self.w2 * self.w2 * d.cos())
            / den;
        // Whisper of drag keeps the integration tame...
        self.w1 += (a1 - 0.03 * self.w1) * h;
        self.w2 += (a2 - 0.03 * self.w2) * h;
        self.th1 += self.w1 * h;
        self.th2 += self.w2 * h;
    }

    fn energy(&self) -> f32 {
        0.5 * (self.w1 * self.w1 + self.w2 * self.w2)
            + 9.81 * (2.0 - self.th1.cos() - self.th2.cos())
    }
}

impl Effect for ChaosPendulum {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let life = get_f32(ctx.params, "trail", 1.6);
        let arms = get_f32(ctx.params, "arms", 0.35);
        let aspect = ctx.layout.aspect;
        let dt = ctx.dt.min(0.05);

        // ...and a kick revives it when the swing dies down.
        if self.energy() < 6.0 {
            self.w1 += ctx.rng.range(3.0, 5.0) * if ctx.rng.chance(0.5) { 1.0 } else { -1.0 };
        }

        let (ax, ay) = (aspect * 0.5, 0.04);
        const ARM: f32 = 0.42;
        let decay = (-dt / life.max(0.05)).exp();
        for h in self.heat.iter_mut() {
            *h *= decay;
        }

        // Integrate in small substeps, inking the trail as the tip moves.
        const STEPS: usize = 8;
        let h = dt / STEPS as f32;
        for _ in 0..STEPS {
            self.substep(h);
            let jx = ax + ARM * self.th1.sin();
            let jy = ay + ARM * self.th1.cos();
            let tx = jx + ARM * self.th2.sin();
            let ty = jy + ARM * self.th2.cos();
            for (i, k) in ctx.layout.keys.iter().enumerate() {
                let (dx, dy) = (k.cx - tx, k.cy - ty);
                let d2 = dx * dx + dy * dy;
                if d2 < 0.05 {
                    self.heat[i] = (self.heat[i] + (-d2 / 0.011).exp() * h * 16.0).min(1.4);
                }
            }
        }

        let jx = ax + ARM * self.th1.sin();
        let jy = ay + ARM * self.th1.cos();
        let tx = jx + ARM * self.th2.sin();
        let ty = jy + ARM * self.th2.cos();
        // Hue drifts with total energy: lazy swings sit low in the palette,
        // violent flailing climbs high.
        let base = 0.12 + 0.6 * (1.0 - (-self.energy() * 0.045).exp());

        for (i, k) in ctx.layout.keys.iter().enumerate() {
            let ink = self.heat[i].min(1.0);
            let mut c = ctx
                .palette
                .sample_clamped(base + 0.22 * ink)
                .scale(ink * ink * 0.95 + ink * 0.05);
            if arms > 0.0 {
                let d = seg_dist(k.cx, k.cy, ax, ay, jx, jy)
                    .min(seg_dist(k.cx, k.cy, jx, jy, tx, ty));
                let a = (-(d / 0.055).powi(2)).exp() * arms * 0.5;
                c = c.add(ctx.palette.sample_clamped(0.9).scale(a));
            }
            // Hot tip.
            let (dx, dy) = (k.cx - tx, k.cy - ty);
            let tip = (-(dx * dx + dy * dy) / 0.006).exp();
            c = c.add(ctx.palette.sample_clamped(0.97).scale(tip * 0.9));
            out.set(k.led, c);
        }
    }
}

// ------------------------------------------------------------ Gravity Wells

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

struct GravityWells {
    seed: u32,
    parts: Vec<Particle>,
    heat: Vec<f32>,
    hue: Vec<f32>,
}

impl GravityWells {
    fn new(layout: &Layout, seed: u64) -> GravityWells {
        let mut rng = Rng::new(seed);
        let parts = (0..24)
            .map(|_| Particle {
                x: rng.range(0.2, 2.3),
                y: rng.range(0.1, 0.9),
                vx: rng.range(-0.4, 0.4),
                vy: rng.range(-0.4, 0.4),
            })
            .collect();
        GravityWells {
            seed: seed as u32,
            parts,
            heat: vec![0.0; layout.keys.len()],
            hue: vec![0.0; layout.keys.len()],
        }
    }
}

impl Effect for GravityWells {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let count = (get_f32(ctx.params, "particles", 14.0).round() as usize).clamp(4, 24);
        let grav = get_f32(ctx.params, "gravity", 1.0);
        let aspect = ctx.layout.aspect;
        let dt = ctx.dt.min(0.05);

        // Two attractors drift on smooth noise paths.
        let tt = ctx.t * 0.055;
        let wells = [
            (
                (noise2(tt, 5.3, self.seed) * 0.7 + 0.15) * aspect,
                noise2(tt, 13.1, self.seed) * 0.7 + 0.15,
            ),
            (
                (noise2(tt, 23.9, self.seed ^ 0x3C3C) * 0.7 + 0.15) * aspect,
                noise2(tt, 31.7, self.seed ^ 0x3C3C) * 0.7 + 0.15,
            ),
        ];

        let decay = (-dt / 0.45).exp();
        for h in self.heat.iter_mut() {
            *h *= decay;
        }

        let g = grav * 0.30;
        for p in self.parts.iter_mut().take(count) {
            // Softened inverse-square pull toward both wells.
            for (wx, wy) in wells {
                let (dx, dy) = (wx - p.x, wy - p.y);
                let d2 = dx * dx + dy * dy + 0.02;
                let a = g / (d2 * d2.sqrt());
                p.vx += dx * a * dt;
                p.vy += dy * a * dt;
            }
            let sp = (p.vx * p.vx + p.vy * p.vy).sqrt();
            if sp > 3.2 {
                let s = 3.2 / sp;
                p.vx *= s;
                p.vy *= s;
            }
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            // Flung off the board? Wrap around the far edge.
            if p.x < -0.15 {
                p.x += aspect + 0.3;
            }
            if p.x > aspect + 0.15 {
                p.x -= aspect + 0.3;
            }
            if p.y < -0.15 {
                p.y += 1.3;
            }
            if p.y > 1.15 {
                p.y -= 1.3;
            }
        }

        for (i, k) in ctx.layout.keys.iter().enumerate() {
            let mut head = 0.0f32;
            for p in self.parts.iter().take(count) {
                let (dx, dy) = (k.cx - p.x, k.cy - p.y);
                let d2 = dx * dx + dy * dy;
                if d2 > 0.06 {
                    continue;
                }
                let sp = (p.vx * p.vx + p.vy * p.vy).sqrt();
                let flare = 0.35 + 0.75 * smoothstep(0.3, 2.2, sp);
                let w = (-d2 / 0.007).exp() * flare;
                head = head.max(w);
                // Ink the trail field, remembering how fast the pass was.
                if w > self.heat[i] {
                    self.heat[i] = w.min(1.2);
                    self.hue[i] = smoothstep(0.3, 2.2, sp);
                }
            }
            let trail = self.heat[i].min(1.0);
            let mut c = ctx
                .palette
                .sample_clamped(0.18 + 0.55 * self.hue[i] * trail + 0.15 * trail)
                .scale(trail * trail);
            // Faint glow marks each well's position.
            for (wx, wy) in wells {
                let (dx, dy) = (k.cx - wx, k.cy - wy);
                let d = (dx * dx + dy * dy).sqrt();
                let ring = (-((d - 0.13) / 0.07).powi(2)).exp() * 0.14
                    + (-(d / 0.05).powi(2)).exp() * 0.10;
                c = c.max(ctx.palette.sample_clamped(0.35).scale(ring));
            }
            c = c.add(ctx.palette.sample_clamped(0.9).scale(head * 0.8));
            out.set(k.led, c);
        }
    }
}

// ------------------------------------------------------- Reaction Diffusion

const RW: usize = 42;
const RH: usize = 14;

struct ReactionDiffusion {
    seed: u32,
    u: Vec<f32>,
    v: Vec<f32>,
    u2: Vec<f32>,
    v2: Vec<f32>,
    check: f32,
}

impl ReactionDiffusion {
    fn new(seed: u64) -> ReactionDiffusion {
        let mut rd = ReactionDiffusion {
            seed: seed as u32,
            u: vec![1.0; RW * RH],
            v: vec![0.0; RW * RH],
            u2: vec![1.0; RW * RH],
            v2: vec![0.0; RW * RH],
            check: 0.0,
        };
        let mut rng = Rng::new(seed);
        for _ in 0..6 {
            rd.splat(&mut rng);
        }
        rd
    }

    fn splat(&mut self, rng: &mut Rng) {
        let cx = rng.below(RW) as i32;
        let cy = rng.below(RH) as i32;
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                if dx * dx + dy * dy > 5 {
                    continue;
                }
                let x = (cx + dx).rem_euclid(RW as i32) as usize;
                let y = (cy + dy).rem_euclid(RH as i32) as usize;
                self.v[y * RW + x] = 0.9;
                self.u[y * RW + x] = 0.4;
            }
        }
    }

    /// One Gray-Scott step (toroidal 9-point Laplacian, dt = 1).
    fn step(&mut self, f: f32, kk: f32) {
        for y in 0..RH {
            let ym = (y + RH - 1) % RH;
            let yp = (y + 1) % RH;
            for x in 0..RW {
                let xm = (x + RW - 1) % RW;
                let xp = (x + 1) % RW;
                let i = y * RW + x;
                let lap = |g: &[f32]| {
                    -g[i] + 0.2 * (g[y * RW + xm] + g[y * RW + xp] + g[ym * RW + x] + g[yp * RW + x])
                        + 0.05 * (g[ym * RW + xm] + g[ym * RW + xp] + g[yp * RW + xm] + g[yp * RW + xp])
                };
                let (u, v) = (self.u[i], self.v[i]);
                let uvv = u * v * v;
                self.u2[i] = (u + lap(&self.u) - uvv + f * (1.0 - u)).clamp(0.0, 1.0);
                self.v2[i] = (v + 0.5 * lap(&self.v) + uvv - (f + kk) * v).clamp(0.0, 1.0);
            }
        }
        std::mem::swap(&mut self.u, &mut self.u2);
        std::mem::swap(&mut self.v, &mut self.v2);
    }
}

impl Effect for ReactionDiffusion {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let iters = (get_f32(ctx.params, "iterations", 4.0).round() as usize).clamp(1, 8);
        // Three classic regimes; feed/kill also breathe slowly inside each so
        // the texture keeps reorganizing instead of freezing.
        let (f0, k0) = match get_str(ctx.params, "regime", "spots") {
            "stripes" => (0.055, 0.062),
            "mitosis" => (0.0367, 0.0649),
            _ => (0.030, 0.061),
        };
        let f = f0 + (noise2(ctx.t * 0.04, 2.2, self.seed) - 0.5) * 0.006;
        let kk = k0 + (noise2(ctx.t * 0.04, 8.8, self.seed) - 0.5) * 0.003;

        for _ in 0..iters {
            self.step(f, kk);
        }

        // If the dish goes uniform (all v died out), reseed a noise blob.
        self.check += ctx.dt;
        if self.check > 1.5 {
            self.check = 0.0;
            let vmax = self.v.iter().fold(0.0f32, |a, b| a.max(*b));
            if vmax < 0.03 {
                let mut rng = Rng::new(self.seed as u64 ^ (ctx.t.to_bits() as u64));
                self.splat(&mut rng);
            }
        }

        for k in &ctx.layout.keys {
            // Average this key's 2x2 sub-grid block.
            let (gx, gy) = cell_of(k, ctx.layout.aspect);
            let (bx, by) = (gx * 2, gy * 2);
            let i0 = by * RW + bx;
            let vavg = 0.25 * (self.v[i0] + self.v[i0 + 1] + self.v[i0 + RW] + self.v[i0 + RW + 1]);
            let uavg = 0.25 * (self.u[i0] + self.u[i0 + 1] + self.u[i0 + RW] + self.u[i0 + RW + 1]);
            let conc = smoothstep(0.04, 0.38, vavg);
            // Substrate shows faintly where u is rich; the catalyst blazes.
            let bed = ctx.palette.sample_clamped(0.10).scale(0.05 + 0.05 * uavg);
            let c = ctx
                .palette
                .sample_clamped(0.15 + 0.80 * conc)
                .scale(0.10 + 0.90 * conc);
            out.set(k.led, bed.max(c));
        }
    }
}

// -------------------------------------------------------------- Cloth Ripple

struct Gust {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    strength: f32,
    age: f32,
}

struct ClothRipple {
    seed: u32,
    h: Vec<f32>,
    vel: Vec<f32>,
    gusts: Vec<Gust>,
    acc: f32,
}

impl ClothRipple {
    fn new(seed: u64) -> ClothRipple {
        ClothRipple {
            seed: seed as u32,
            h: vec![0.0; GW * GH],
            vel: vec![0.0; GW * GH],
            gusts: Vec::new(),
            acc: 0.0,
        }
    }
}

impl Effect for ClothRipple {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let damping = get_f32(ctx.params, "damping", 1.5);
        let gust_rate = get_f32(ctx.params, "gusts", 0.7);
        let dt = ctx.dt.min(0.1);

        // Noise-timed gusts enter from a side and sweep across the sheet.
        if ctx.rng.chance(dt * gust_rate * (0.4 + 1.2 * noise2(ctx.t * 0.3, 1.0, self.seed))) {
            let from_left = ctx.rng.chance(0.5);
            self.gusts.push(Gust {
                x: if from_left { -2.0 } else { GW as f32 + 2.0 },
                y: ctx.rng.range(0.5, GH as f32 - 0.5),
                vx: if from_left { 1.0 } else { -1.0 } * ctx.rng.range(9.0, 16.0),
                vy: ctx.rng.range(-2.0, 2.0),
                strength: ctx.rng.range(6.0, 14.0),
                age: 0.0,
            });
        }

        // Moving pressure fronts poke the sheet as they travel.
        for g in self.gusts.iter_mut() {
            g.x += g.vx * dt;
            g.y += g.vy * dt;
            g.age += dt;
            for gy in 0..GH {
                for gx in 0..GW {
                    let dx = (gx as f32 - g.x) / 2.2;
                    let dy = (gy as f32 - g.y) / 1.6;
                    let w = (-(dx * dx + dy * dy)).exp();
                    if w > 0.01 {
                        self.vel[gy * GW + gx] -= g.strength * w * dt;
                    }
                }
            }
        }
        self.gusts.retain(|g| g.x > -4.0 && g.x < GW as f32 + 4.0 && g.age < 6.0);

        // Damped 2D wave equation, fixed 120 Hz substeps (Neumann edges).
        self.acc += dt;
        const H: f32 = 1.0 / 120.0;
        const C2: f32 = 160.0;
        while self.acc >= H {
            self.acc -= H;
            for gy in 0..GH {
                for gx in 0..GW {
                    let i = gy * GW + gx;
                    let hc = self.h[i];
                    let l = if gx > 0 { self.h[i - 1] } else { hc };
                    let r = if gx + 1 < GW { self.h[i + 1] } else { hc };
                    let u = if gy > 0 { self.h[i - GW] } else { hc };
                    let d = if gy + 1 < GH { self.h[i + GW] } else { hc };
                    let lap = l + r + u + d - 4.0 * hc;
                    self.vel[i] += (C2 * lap - damping * self.vel[i]) * H;
                }
            }
            for i in 0..GW * GH {
                self.h[i] += self.vel[i] * H;
                self.h[i] *= 1.0 - 0.06 * H;
            }
        }

        for k in &ctx.layout.keys {
            let h = sample_grid(&self.h, k, ctx.layout.aspect);
            // Height maps through the palette; crests catch the light.
            let t_pal = (0.5 + h * 1.5).clamp(0.0, 1.0);
            let lit = 0.28 + 0.72 * smoothstep(-0.45, 0.5, h);
            let spec = smoothstep(0.22, 0.5, h);
            let c = ctx
                .palette
                .sample_clamped(t_pal)
                .scale(lit)
                .add(Col::WHITE.scale(spec * 0.45));
            out.set(k.led, c);
        }
    }
}

