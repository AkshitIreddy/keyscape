//! Kinetic — autonomous moving things: flocks, comets, sweeps, snakes,
//! spirals, shuttles and orbits. Everything here is simulated with simple
//! physics so motion carries inertia instead of teleporting.

use super::*;
use crate::color::{smoothstep, Col};
use crate::math::noise2;
use crate::params::{get_f32, ParamSpec};
use std::f32::consts::{PI, TAU};

pub fn effects() -> Vec<EffectInfo> {
    vec![
        EffectInfo {
            id: "swarm",
            name: "Swarm",
            category: "Kinetic",
            blurb: "A flock of boids chases an unseen wanderer, scattering when startled.",
            needs_input: false,
            default_palette: "aurora",
            extras: || {
                vec![
                    ParamSpec::slider("boids", "Boids", 6.0, 28.0, 1.0, 16.0),
                    ParamSpec::slider("cohesion", "Cohesion", 0.2, 2.0, 0.05, 1.0),
                ]
            },
            make: |layout, seed| {
                let mut rng = Rng::new(seed);
                let a = layout.aspect;
                let boids = (0..16)
                    .map(|_| Boid {
                        x: rng.range(a * 0.3, a * 0.7),
                        y: rng.range(0.3, 0.7),
                        vx: rng.range(-0.3, 0.3),
                        vy: rng.range(-0.3, 0.3),
                    })
                    .collect();
                let first = rng.range(6.0, 12.0);
                Box::new(Swarm {
                    seed: seed as u32,
                    rng,
                    boids,
                    trail: vec![Col::BLACK; layout.keys.len()],
                    fear: 0.0,
                    next_startle: first,
                })
            },
        },
        EffectInfo {
            id: "comet_billiards",
            name: "Comet Billiards",
            category: "Kinetic",
            blurb: "Comets carom off the walls, trailing fire and trading sparks.",
            needs_input: false,
            default_palette: "synthwave",
            extras: || {
                vec![
                    ParamSpec::slider("comets", "Comets", 1.0, 6.0, 1.0, 3.0),
                    ParamSpec::slider("tail", "Tail length", 0.5, 3.0, 0.05, 1.5),
                ]
            },
            make: |layout, seed| {
                Box::new(Billiards {
                    seed: seed as u32,
                    rng: Rng::new(seed),
                    comets: Vec::new(),
                    sparks: Vec::new(),
                    tail: vec![Col::BLACK; layout.keys.len()],
                })
            },
        },
        EffectInfo {
            id: "radar_sweep",
            name: "Radar Sweep",
            category: "Kinetic",
            blurb: "A drifting radar beam paints a phosphor wake and hidden contacts.",
            needs_input: false,
            default_palette: "matrix",
            extras: || {
                vec![
                    ParamSpec::slider("sweep", "Sweep rate", 0.1, 0.8, 0.02, 0.3),
                    ParamSpec::slider("contacts", "Contacts", 2.0, 8.0, 1.0, 5.0),
                ]
            },
            make: |layout, seed| {
                Box::new(Radar {
                    seed: seed as u32,
                    rng: Rng::new(seed),
                    theta: 0.0,
                    wake: vec![0.0; layout.keys.len()],
                    contacts: Vec::new(),
                })
            },
        },
        EffectInfo {
            id: "snake_trio",
            name: "Snake Trio",
            category: "Kinetic",
            blurb: "Three snakes hunt glowing pellets, grow, shed, and dodge each other.",
            needs_input: false,
            default_palette: "toxic",
            extras: || {
                vec![
                    ParamSpec::slider("cap", "Length cap", 6.0, 16.0, 1.0, 10.0),
                    ParamSpec::slider("pellets", "Pellets", 1.0, 6.0, 1.0, 3.0),
                ]
            },
            make: |layout, seed| {
                let mut rng = Rng::new(seed);
                let nkeys = layout.keys.len();
                let mut occ = vec![false; nkeys];
                let mut snakes = Vec::new();
                for i in 0..3 {
                    let mut head = rng.below(nkeys);
                    for _ in 0..20 {
                        if !occ[head] {
                            break;
                        }
                        head = rng.below(nkeys);
                    }
                    occ[head] = true;
                    let mut body = vec![head];
                    for _ in 0..3 {
                        let last = *body.last().unwrap();
                        let opts: Vec<usize> =
                            layout.neighbors[last].iter().copied().filter(|&c| !occ[c]).collect();
                        if opts.is_empty() {
                            break;
                        }
                        let nx = opts[rng.below(opts.len())];
                        occ[nx] = true;
                        body.push(nx);
                    }
                    let ang = rng.range(0.0, TAU);
                    let prev = if body.len() > 1 { body[1] } else { body[0] };
                    let jitter = rng.range(0.0, SNAKE_STEP);
                    snakes.push(Snake {
                        body,
                        prev_head: prev,
                        hx: ang.cos(),
                        hy: ang.sin(),
                        step_t: jitter,
                        grow: 0,
                        shed: 0.0,
                        shrunk: true,
                        phase: i as f32 / 3.0,
                    });
                }
                Box::new(SnakeTrio { rng, snakes, pellets: Vec::new() })
            },
        },
        EffectInfo {
            id: "spiral_bloom",
            name: "Spiral Bloom",
            category: "Kinetic",
            blurb: "Spirals bloom outward from a wandering origin, then unwind into their counterparts.",
            needs_input: false,
            default_palette: "sakura",
            extras: || {
                vec![
                    ParamSpec::slider("tightness", "Arm tightness", 0.04, 0.14, 0.005, 0.08),
                    ParamSpec::slider("pause", "Bloom pause", 0.0, 2.0, 0.05, 0.6),
                ]
            },
            make: |_, seed| {
                let mut rng = Rng::new(seed);
                let theta0 = rng.range(0.0, TAU);
                Box::new(SpiralBloom {
                    seed: seed as u32,
                    rng,
                    arms: [
                        Arm { dir: 1.0, shift: 0.0, state: 1, r_tip: 0.0, hold: 0.0, theta0 },
                        Arm { dir: -1.0, shift: 0.37, state: 0, r_tip: 0.0, hold: 0.0, theta0: 0.0 },
                    ],
                })
            },
        },
        EffectInfo {
            id: "loom_weave",
            name: "Loom Weave",
            category: "Kinetic",
            blurb: "A shuttle weaves the board row by row; the cloth ripples, unravels, re-weaves.",
            needs_input: false,
            default_palette: "royal",
            extras: || {
                vec![
                    ParamSpec::slider("shuttle", "Shuttle rate", 0.5, 3.0, 0.05, 1.2),
                    ParamSpec::slider("contrast", "Thread contrast", 0.0, 1.0, 0.05, 0.6),
                ]
            },
            make: |layout, seed| {
                // Group keys into physical rows, sorted top to bottom.
                let mut groups: Vec<(f32, f32, f32, u8)> = Vec::new(); // y, x0, x1, row id
                for rid in 0..=7u8 {
                    let grp: Vec<_> = layout.keys.iter().filter(|k| k.row == rid).collect();
                    if grp.is_empty() {
                        continue;
                    }
                    let y = grp.iter().map(|k| k.cy).sum::<f32>() / grp.len() as f32;
                    let x0 = grp.iter().map(|k| k.cx).fold(f32::MAX, f32::min);
                    let x1 = grp.iter().map(|k| k.cx).fold(f32::MIN, f32::max);
                    groups.push((y, x0 - 0.10, x1 + 0.10, rid));
                }
                groups.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                let rows: Vec<RowInfo> =
                    groups.iter().map(|g| RowInfo { y: g.0, x0: g.1, x1: g.2 }).collect();
                let row_of: Vec<usize> = layout
                    .keys
                    .iter()
                    .map(|k| groups.iter().position(|g| g.3 == k.row).unwrap_or(0))
                    .collect();
                let x = rows[0].x0;
                Box::new(Loom {
                    seed: seed as u32,
                    rows,
                    row_of,
                    woven: vec![0.0; layout.keys.len()],
                    state: 0,
                    row_i: 0,
                    x,
                    ripple: 0.0,
                    phase: 0.0,
                })
            },
        },
        EffectInfo {
            id: "orrery",
            name: "Orrery",
            category: "Kinetic",
            blurb: "A tiny solar system: elliptical orbits, a moon, perihelion glints, rare comets.",
            needs_input: false,
            default_palette: "midnight",
            extras: || {
                vec![
                    ParamSpec::slider("planets", "Planets", 2.0, 6.0, 1.0, 4.0),
                    ParamSpec::slider("trail", "Trail length", 0.3, 2.5, 0.05, 1.0),
                ]
            },
            make: |layout, seed| {
                let mut rng = Rng::new(seed);
                let planets = (0..6)
                    .map(|i| Planet {
                        a: 0.16 + 0.09 * i as f32,
                        e: rng.range(0.12, 0.30),
                        orient: rng.range(0.0, TAU),
                        th: rng.range(0.0, TAU),
                        pal: 0.15 + 0.14 * i as f32,
                    })
                    .collect();
                Box::new(Orrery {
                    seed: seed as u32,
                    rng,
                    planets,
                    moon_th: 0.0,
                    trail: vec![Col::BLACK; layout.keys.len()],
                    comet: None,
                })
            },
        },
    ]
}

// ---------------------------------------------------------------- helpers

/// Additive gaussian splat at (x, y) in iso space — the standard way to
/// light a moving point without single-key stepping.
fn splat(out: &mut Frame, layout: &Layout, x: f32, y: f32, sigma: f32, col: Col) {
    let inv = 1.0 / (2.0 * sigma * sigma);
    let cut = 9.0 * sigma * sigma; // 3 sigma
    for k in &layout.keys {
        let d2 = (k.cx - x) * (k.cx - x) + (k.cy - y) * (k.cy - y);
        if d2 < cut {
            out.add(k.led, col.scale((-d2 * inv).exp()));
        }
    }
}

/// Max-blend the same splat into a per-key trail canvas, so decayed history
/// and the fresh head never over-accumulate.
fn splat_max(buf: &mut [Col], layout: &Layout, x: f32, y: f32, sigma: f32, col: Col) {
    let inv = 1.0 / (2.0 * sigma * sigma);
    let cut = 9.0 * sigma * sigma;
    for (i, k) in layout.keys.iter().enumerate() {
        let d2 = (k.cx - x) * (k.cx - x) + (k.cy - y) * (k.cy - y);
        if d2 < cut {
            buf[i] = buf[i].max(col.scale((-d2 * inv).exp()));
        }
    }
}

fn decay(buf: &mut [Col], keep: f32) {
    for c in buf.iter_mut() {
        *c = c.scale(keep);
    }
}

/// Smoothly wandering point driven by value noise, stretched so it actually
/// visits the edges of the given rectangle.
fn wander(t: f32, seed: u32, x0: f32, x1: f32, y0: f32, y1: f32) -> (f32, f32) {
    let sh = |n: f32| ((n - 0.5) * 1.9 + 0.5).clamp(0.0, 1.0);
    let nx = sh(noise2(t, 17.3, seed));
    let ny = sh(noise2(t + 53.7, 4.9, seed ^ 0x9E37));
    (x0 + (x1 - x0) * nx, y0 + (y1 - y0) * ny)
}

// ---------------------------------------------------------------- Swarm

struct Boid {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

struct Swarm {
    seed: u32,
    rng: Rng,
    boids: Vec<Boid>,
    trail: Vec<Col>,
    fear: f32,
    next_startle: f32,
}

impl Effect for Swarm {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let want = (get_f32(ctx.params, "boids", 16.0).round() as usize).clamp(4, 28);
        let coh = get_f32(ctx.params, "cohesion", 1.0);
        let a = ctx.layout.aspect;
        let dt = ctx.dt.min(0.08);

        while self.boids.len() < want {
            let b = Boid {
                x: self.rng.range(a * 0.35, a * 0.65),
                y: self.rng.range(0.3, 0.7),
                vx: self.rng.range(-0.4, 0.4),
                vy: self.rng.range(-0.4, 0.4),
            };
            self.boids.push(b);
        }
        self.boids.truncate(want);

        // Invisible target the flock is drawn toward.
        let (tx, ty) = wander(ctx.t * 0.07, self.seed, 0.25, a - 0.25, 0.18, 0.82);

        let n = self.boids.len() as f32;
        let (mut mx, mut my, mut ax, mut ay) = (0.0, 0.0, 0.0, 0.0);
        for b in &self.boids {
            mx += b.x;
            my += b.y;
            ax += b.vx;
            ay += b.vy;
        }
        mx /= n;
        my /= n;
        ax /= n;
        ay /= n;

        // Occasional fright: an outward kick, then cohesion flips sign while
        // fear is high so the flock scatters before regrouping.
        if ctx.t >= self.next_startle {
            self.next_startle = ctx.t + self.rng.range(7.0, 15.0);
            self.fear = 1.0;
            for b in &mut self.boids {
                let (dx, dy) = (b.x - mx, b.y - my);
                let d = (dx * dx + dy * dy).sqrt().max(0.05);
                let kick = self.rng.range(0.9, 1.6);
                b.vx += dx / d * kick + self.rng.range(-0.4, 0.4);
                b.vy += dy / d * kick + self.rng.range(-0.4, 0.4);
            }
        }
        self.fear *= (-dt * 1.1).exp();
        let fear = self.fear;

        let snap: Vec<(f32, f32)> = self.boids.iter().map(|b| (b.x, b.y)).collect();
        for (i, b) in self.boids.iter_mut().enumerate() {
            let mut fx = (tx - b.x) * 0.9 * (1.0 - 0.8 * fear);
            let mut fy = (ty - b.y) * 0.9 * (1.0 - 0.8 * fear);
            let cf = 1.5 * coh * (1.0 - 2.4 * fear); // negative while startled
            fx += (mx - b.x) * cf;
            fy += (my - b.y) * cf;
            fx += (ax - b.vx) * 1.4 * (1.0 - 0.6 * fear);
            fy += (ay - b.vy) * 1.4 * (1.0 - 0.6 * fear);
            for (j, &(ox, oy)) in snap.iter().enumerate() {
                if i == j {
                    continue;
                }
                let (dx, dy) = (b.x - ox, b.y - oy);
                let d2 = dx * dx + dy * dy;
                if d2 < 0.14 * 0.14 {
                    let d = d2.sqrt().max(1e-4);
                    let push = (0.14 - d) / 0.14 * 3.2;
                    fx += dx / d * push;
                    fy += dy / d * push;
                }
            }
            let m = 0.12;
            if b.x < m {
                fx += (m - b.x) * 22.0;
            }
            if b.x > a - m {
                fx -= (b.x - (a - m)) * 22.0;
            }
            if b.y < m {
                fy += (m - b.y) * 22.0;
            }
            if b.y > 1.0 - m {
                fy -= (b.y - (1.0 - m)) * 22.0;
            }
            b.vx += fx * dt;
            b.vy += fy * dt;
            let sp = (b.vx * b.vx + b.vy * b.vy).sqrt().max(1e-5);
            let k = sp.clamp(0.18, 0.85 * (1.0 + 1.4 * fear)) / sp;
            b.vx *= k;
            b.vy *= k;
            b.x = (b.x + b.vx * dt).clamp(0.02, a - 0.02);
            b.y = (b.y + b.vy * dt).clamp(0.02, 0.98);
        }

        // Short motion-blur trail: decayed max-canvas with heads stamped on top.
        decay(&mut self.trail, (-dt * 6.5).exp());
        for b in &self.boids {
            let hue = b.vy.atan2(b.vx) / TAU; // heading angle -> palette position
            let col = ctx.palette.sample(hue).scale(0.9 + 0.5 * fear);
            splat_max(&mut self.trail, ctx.layout, b.x, b.y, 0.105, col);
        }
        for (i, k) in ctx.layout.keys.iter().enumerate() {
            out.set(k.led, self.trail[i]);
        }
    }
}

// ---------------------------------------------------------------- Comet Billiards

struct Comet {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    squash: f32,
}

struct Spark {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    col: Col,
}

struct Billiards {
    seed: u32,
    rng: Rng,
    comets: Vec<Comet>,
    sparks: Vec<Spark>,
    tail: Vec<Col>,
}

impl Effect for Billiards {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let want = (get_f32(ctx.params, "comets", 3.0).round() as usize).clamp(1, 6);
        let tail_len = get_f32(ctx.params, "tail", 1.5).max(0.3);
        let a = ctx.layout.aspect;
        let dt = ctx.dt.min(0.08);

        while self.comets.len() < want {
            let ang = self.rng.range(0.0, TAU);
            let sp = self.rng.range(0.38, 0.55);
            let c = Comet {
                x: self.rng.range(a * 0.2, a * 0.8),
                y: self.rng.range(0.2, 0.8),
                vx: ang.cos() * sp,
                vy: ang.sin() * sp,
                squash: 0.0,
            };
            self.comets.push(c);
        }
        self.comets.truncate(want);

        // Energy slowly breathes, so the whole table speeds up and relaxes.
        let energy = 0.5 + 0.9 * noise2(ctx.t * 0.07, 2.6, self.seed);

        let m = 0.07;
        for c in &mut self.comets {
            c.x += c.vx * energy * dt;
            c.y += c.vy * energy * dt;
            if c.x < m {
                c.x = 2.0 * m - c.x;
                c.vx = -c.vx;
                c.squash = 1.0;
            }
            if c.x > a - m {
                c.x = 2.0 * (a - m) - c.x;
                c.vx = -c.vx;
                c.squash = 1.0;
            }
            if c.y < m {
                c.y = 2.0 * m - c.y;
                c.vy = -c.vy;
                c.squash = 1.0;
            }
            if c.y > 1.0 - m {
                c.y = 2.0 * (1.0 - m) - c.y;
                c.vy = -c.vy;
                c.squash = 1.0;
            }
            c.squash *= (-dt * 6.0).exp();
        }

        // Close encounters spray sparks.
        for i in 0..self.comets.len() {
            for j in (i + 1)..self.comets.len() {
                let dx = self.comets[i].x - self.comets[j].x;
                let dy = self.comets[i].y - self.comets[j].y;
                if dx * dx + dy * dy < 0.22 * 0.22
                    && self.sparks.len() < 60
                    && self.rng.chance((dt * 9.0).min(0.5))
                {
                    let (px, py) = (
                        (self.comets[i].x + self.comets[j].x) * 0.5,
                        (self.comets[i].y + self.comets[j].y) * 0.5,
                    );
                    let base = ctx.palette.sample_clamped(0.95);
                    let col = Col::lerp(base, Col::WHITE, 0.4);
                    for _ in 0..2 {
                        let ang = self.rng.range(0.0, TAU);
                        let sp = self.rng.range(0.35, 0.85);
                        let life = self.rng.range(0.3, 0.5);
                        self.sparks.push(Spark {
                            x: px,
                            y: py,
                            vx: ang.cos() * sp,
                            vy: ang.sin() * sp,
                            life,
                            col,
                        });
                    }
                }
            }
        }
        for s in &mut self.sparks {
            let drag = (-dt * 1.5).exp();
            s.vx *= drag;
            s.vy *= drag;
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            s.life -= dt;
        }
        self.sparks.retain(|s| s.life > 0.0);

        // Long decaying tails through the trail canvas.
        decay(&mut self.tail, (-dt * 3.2 / tail_len).exp());
        for (i, c) in self.comets.iter().enumerate() {
            let phase = i as f32 / want as f32;
            let col = ctx.palette.sample(phase + ctx.t * 0.02);
            splat_max(&mut self.tail, ctx.layout, c.x, c.y, 0.095, col.scale(0.95));
        }
        for (i, k) in ctx.layout.keys.iter().enumerate() {
            out.set(k.led, self.tail[i]);
        }
        // Bright heads with a squash flash on impact.
        for (i, c) in self.comets.iter().enumerate() {
            let phase = i as f32 / want as f32;
            let col = ctx.palette.sample(phase + ctx.t * 0.02);
            let sigma = 0.085 * (1.0 + 0.5 * c.squash);
            splat(out, ctx.layout, c.x, c.y, sigma, col.scale(0.35 + 1.1 * c.squash));
        }
        for s in &self.sparks {
            splat(out, ctx.layout, s.x, s.y, 0.05, s.col.scale((s.life / 0.5).clamp(0.0, 1.0)));
        }
    }
}

// ---------------------------------------------------------------- Radar Sweep

struct Contact {
    key: usize,
    blip: f32,
    echo: f32,
    sweeps: i32,
}

struct Radar {
    seed: u32,
    rng: Rng,
    theta: f32,
    wake: Vec<f32>,
    contacts: Vec<Contact>,
}

impl Radar {
    /// Beam intensity at a key: perpendicular distance to the rotating ray.
    fn beam(cx: f32, cy: f32, th: f32, k: &crate::layout::Key) -> f32 {
        let (dx, dy) = (k.cx - cx, k.cy - cy);
        let r = (dx * dx + dy * dy).sqrt();
        let phi = dy.atan2(dx);
        let dphi = (phi - th + PI).rem_euclid(TAU) - PI;
        if dphi.cos() <= 0.0 {
            return 0.0;
        }
        let off = r * dphi.sin();
        let g = (-(off * off) / (2.0 * 0.05 * 0.05)).exp();
        g * (1.0 - 0.3 * smoothstep(0.6, 1.4, r))
    }
}

impl Effect for Radar {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let rate = get_f32(ctx.params, "sweep", 0.3).clamp(0.05, 1.0);
        let want = (get_f32(ctx.params, "contacts", 5.0).round() as usize).clamp(2, 8);
        let a = ctx.layout.aspect;
        let nkeys = ctx.layout.keys.len();

        while self.contacts.len() < want {
            let key = self.rng.below(nkeys);
            let sweeps = 2 + self.rng.below(3) as i32;
            self.contacts.push(Contact { key, blip: 0.0, echo: -1.0, sweeps });
        }
        self.contacts.truncate(want);

        let (cx, cy) = wander(ctx.t * 0.03, self.seed, a * 0.3, a * 0.7, 0.3, 0.7);

        self.theta += ctx.dt * rate * TAU;
        if self.theta >= TAU {
            self.theta -= TAU;
            // Sweep completed: contacts quietly relocate every few sweeps.
            for c in &mut self.contacts {
                c.sweeps -= 1;
                if c.sweeps <= 0 {
                    c.key = self.rng.below(nkeys);
                    c.sweeps = 2 + self.rng.below(3) as i32;
                    c.blip = 0.0;
                    c.echo = -1.0;
                }
            }
        }
        let th = self.theta;

        // Beam + phosphor wake.
        let keep = (-ctx.dt * 2.6 * rate).exp();
        for (i, k) in ctx.layout.keys.iter().enumerate() {
            let b = Radar::beam(cx, cy, th, k);
            self.wake[i] = (self.wake[i] * keep).max(b);
        }

        // Contacts only light while the beam passes them, then decay until
        // the next pass; sometimes a fainter echo blips shortly after.
        for c in &mut self.contacts {
            let b = Radar::beam(cx, cy, th, &ctx.layout.keys[c.key]);
            if b > 0.55 {
                if c.blip < 0.4 && c.echo < 0.0 && self.rng.chance(0.35) {
                    c.echo = ctx.t + 0.30;
                }
                c.blip = 1.0;
            }
            if c.echo > 0.0 && ctx.t >= c.echo {
                c.blip = c.blip.max(0.8);
                c.echo = -1.0;
            }
            c.blip *= (-ctx.dt * 3.0 * rate).exp();
        }

        for (i, k) in ctx.layout.keys.iter().enumerate() {
            let h = self.wake[i];
            if h > 0.004 {
                let col = ctx.palette.sample_clamped(0.15 + 0.75 * h);
                out.set(k.led, col.scale(0.04 + 0.96 * h.powf(1.6)));
            }
        }
        for c in &self.contacts {
            if c.blip > 0.01 {
                let k = &ctx.layout.keys[c.key];
                let col = ctx.palette.sample_clamped(0.97);
                splat(out, ctx.layout, k.cx, k.cy, 0.07, col.scale(c.blip * 1.15));
            }
        }
        // Rotating hub.
        splat(out, ctx.layout, cx, cy, 0.05, ctx.palette.sample_clamped(0.85).scale(0.3));
    }
}

// ---------------------------------------------------------------- Snake Trio

const SNAKE_STEP: f32 = 0.11;

struct Snake {
    body: Vec<usize>, // head first
    prev_head: usize,
    hx: f32,
    hy: f32,
    step_t: f32,
    grow: u32,
    shed: f32,
    shrunk: bool,
    phase: f32,
}

struct SnakeTrio {
    rng: Rng,
    snakes: Vec<Snake>,
    pellets: Vec<usize>,
}

impl Effect for SnakeTrio {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let cap = (get_f32(ctx.params, "cap", 10.0).round() as usize).clamp(6, 16);
        let want_p = (get_f32(ctx.params, "pellets", 3.0).round() as usize).clamp(1, 6);
        let dt = ctx.dt.min(0.08);
        let keys = &ctx.layout.keys;
        let nkeys = keys.len();

        let mut occ = vec![false; nkeys];
        for s in &self.snakes {
            for &b in &s.body {
                occ[b] = true;
            }
        }

        if self.pellets.len() < want_p {
            for _ in 0..40 {
                if self.pellets.len() >= want_p {
                    break;
                }
                let k = self.rng.below(nkeys);
                if !occ[k] && !self.pellets.contains(&k) {
                    self.pellets.push(k);
                }
            }
        }
        self.pellets.truncate(want_p);

        let heads: Vec<(f32, f32)> = self
            .snakes
            .iter()
            .map(|s| {
                let k = &keys[s.body[0]];
                (k.cx, k.cy)
            })
            .collect();

        for si in 0..self.snakes.len() {
            let (do_step, head, shx, shy);
            {
                let s = &mut self.snakes[si];
                // Shed: flash, then shrink back to base length.
                if s.shed > 0.0 {
                    s.shed -= dt;
                    if s.shed < 0.30 && !s.shrunk {
                        s.shrunk = true;
                        if s.body.len() > 4 {
                            for &b in &s.body[4..] {
                                occ[b] = false;
                            }
                            s.body.truncate(4);
                        }
                    }
                }
                s.step_t -= dt;
                do_step = s.step_t <= 0.0;
                if do_step {
                    s.step_t = (s.step_t + SNAKE_STEP).max(0.0);
                }
                head = s.body[0];
                shx = s.hx;
                shy = s.hy;
            }
            if !do_step {
                continue;
            }

            // Direction toward the nearest pellet.
            let hk = (keys[head].cx, keys[head].cy);
            let (mut pdx, mut pdy) = (0.0, 0.0);
            let mut best_d = f32::MAX;
            for &p in &self.pellets {
                let (dx, dy) = (keys[p].cx - hk.0, keys[p].cy - hk.1);
                let d2 = dx * dx + dy * dy;
                if d2 < best_d {
                    best_d = d2;
                    let l = d2.sqrt().max(1e-4);
                    pdx = dx / l;
                    pdy = dy / l;
                }
            }

            // Score candidate moves: keep heading, chase food, dodge others.
            let mut choice: Option<(usize, f32, f32)> = None;
            let mut best_score = f32::MIN;
            for &cand in &ctx.layout.neighbors[head] {
                if occ[cand] {
                    continue;
                }
                let (dx, dy) = (keys[cand].cx - hk.0, keys[cand].cy - hk.1);
                let l = (dx * dx + dy * dy).sqrt().max(1e-4);
                let (ux, uy) = (dx / l, dy / l);
                let mut score =
                    1.1 * (ux * shx + uy * shy) + 1.7 * (ux * pdx + uy * pdy) + self.rng.range(0.0, 0.5);
                for (oi, &(ox, oy)) in heads.iter().enumerate() {
                    if oi == si {
                        continue;
                    }
                    let d = ((keys[cand].cx - ox) * (keys[cand].cx - ox)
                        + (keys[cand].cy - oy) * (keys[cand].cy - oy))
                        .sqrt();
                    if d < 0.30 {
                        score -= 2.2 * (1.0 - d / 0.30);
                    }
                }
                if score > best_score {
                    best_score = score;
                    choice = Some((cand, ux, uy));
                }
            }

            if let Some((cand, ux, uy)) = choice {
                let ate = self.pellets.iter().position(|&p| p == cand);
                {
                    let s = &mut self.snakes[si];
                    s.prev_head = s.body[0];
                    s.body.insert(0, cand);
                    occ[cand] = true;
                    s.hx = s.hx * 0.35 + ux * 0.65;
                    s.hy = s.hy * 0.35 + uy * 0.65;
                    let hl = (s.hx * s.hx + s.hy * s.hy).sqrt().max(1e-4);
                    s.hx /= hl;
                    s.hy /= hl;
                    if ate.is_some() {
                        s.grow += 2; // eating grows the snake by 2
                    }
                    if s.grow > 0 {
                        s.grow -= 1;
                    } else if s.body.len() > 1 {
                        let t = s.body.pop().unwrap();
                        occ[t] = false;
                    }
                    if s.body.len() >= cap && s.shed <= 0.0 {
                        s.shed = 0.55;
                        s.shrunk = false;
                    }
                }
                if let Some(pi) = ate {
                    // Respawn the pellet somewhere free, away from all heads.
                    let mut spot = self.pellets[pi];
                    for _ in 0..40 {
                        let c2 = self.rng.below(nkeys);
                        if occ[c2] || self.pellets.contains(&c2) {
                            continue;
                        }
                        spot = c2;
                        let far = heads.iter().all(|&(ox, oy)| {
                            let k2 = &keys[c2];
                            (k2.cx - ox) * (k2.cx - ox) + (k2.cy - oy) * (k2.cy - oy) > 0.09
                        });
                        if far {
                            break;
                        }
                    }
                    self.pellets[pi] = spot;
                }
            } else {
                // Boxed in: shuffle heading and wait a step.
                let ang = self.rng.range(0.0, TAU);
                let s = &mut self.snakes[si];
                s.hx = ang.cos();
                s.hy = ang.sin();
            }
        }

        // Pellets pulse invitingly.
        for (pi, &p) in self.pellets.iter().enumerate() {
            let k = &keys[p];
            let pulse = 0.6 + 0.4 * (ctx.t * 4.5 + pi as f32 * 1.9).sin();
            let col = ctx.palette.sample_clamped(0.92);
            out.max(k.led, col.scale(0.55 + 0.45 * pulse));
            splat(out, ctx.layout, k.cx, k.cy, 0.06, col.scale(0.35 * pulse));
        }

        for s in &self.snakes {
            let len = s.body.len().max(2) as f32;
            let flash = if s.shed > 0.0 {
                (s.shed * 18.0).sin().abs() * (s.shed / 0.55)
            } else {
                0.0
            };
            for (bi, &b) in s.body.iter().enumerate() {
                let f = bi as f32 / (len - 1.0);
                let mut col = ctx.palette.sample(s.phase + 0.16 - 0.30 * f);
                if flash > 0.0 {
                    col = Col::lerp(col, Col::WHITE, flash * 0.8);
                }
                out.max(keys[b].led, col.scale((1.0 - 0.78 * f) * (0.85 + 0.6 * flash)));
            }
            // Head glides between the previous and current key.
            let p = 1.0 - (s.step_t / SNAKE_STEP).clamp(0.0, 1.0);
            let e = smoothstep(0.0, 1.0, p);
            let (k0, k1) = (&keys[s.prev_head], &keys[s.body[0]]);
            let hx = k0.cx + (k1.cx - k0.cx) * e;
            let hy = k0.cy + (k1.cy - k0.cy) * e;
            let hcol = ctx.palette.sample(s.phase + 0.20);
            splat(out, ctx.layout, hx, hy, 0.085, hcol.scale(0.9 + 0.5 * flash));
        }
    }
}

// ---------------------------------------------------------------- Spiral Bloom

#[derive(Clone, Copy)]
struct Arm {
    dir: f32,
    shift: f32,
    state: u8, // 0 dormant, 1 blooming, 2 holding, 3 unwinding
    r_tip: f32,
    hold: f32,
    theta0: f32,
}

struct SpiralBloom {
    seed: u32,
    rng: Rng,
    arms: [Arm; 2],
}

impl Effect for SpiralBloom {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let tight = get_f32(ctx.params, "tightness", 0.08).clamp(0.03, 0.2);
        let pause = get_f32(ctx.params, "pause", 0.6).max(0.0);
        let a = ctx.layout.aspect;
        let dt = ctx.dt.min(0.08);

        let (ox, oy) = wander(ctx.t * 0.035, self.seed, a * 0.28, a * 0.72, 0.28, 0.72);
        // Full bloom reaches the farthest corner.
        let corners = [(0.0f32, 0.0f32), (a, 0.0), (0.0, 1.0), (a, 1.0)];
        let rmax = corners
            .iter()
            .map(|&(x, y)| ((x - ox) * (x - ox) + (y - oy) * (y - oy)).sqrt())
            .fold(0.0f32, f32::max)
            + 0.06;

        // Bloom -> hold a beat -> unwind, while the counter-rotating partner
        // in a shifted palette phase starts blooming.
        for i in 0..2 {
            let mut arm = self.arms[i];
            let other_dormant = self.arms[1 - i].state == 0;
            let mut spawn_other = false;
            match arm.state {
                1 => {
                    arm.r_tip += dt * 0.34;
                    if arm.r_tip >= rmax {
                        arm.r_tip = rmax;
                        arm.state = 2;
                        arm.hold = pause.max(0.15);
                    }
                }
                2 => {
                    arm.hold -= dt;
                    if arm.hold <= 0.0 && other_dormant {
                        arm.state = 3;
                        spawn_other = true;
                    }
                }
                3 => {
                    arm.r_tip -= dt * 0.5;
                    if arm.r_tip <= 0.0 {
                        arm.r_tip = 0.0;
                        arm.state = 0;
                    }
                }
                _ => {}
            }
            self.arms[i] = arm;
            if spawn_other {
                self.arms[1 - i] = Arm {
                    dir: -arm.dir,
                    shift: (arm.shift + self.rng.range(0.30, 0.45)).fract(),
                    state: 1,
                    r_tip: 0.0,
                    hold: 0.0,
                    theta0: self.rng.range(0.0, TAU),
                };
            }
        }

        for arm in self.arms.iter().copied() {
            if arm.state == 0 {
                continue;
            }
            let tip = (arm.r_tip / tight).max(0.001);
            let breathe = if arm.state == 2 { 0.85 + 0.15 * (ctx.t * 2.6).sin() } else { 1.0 };
            for k in &ctx.layout.keys {
                let (dx, dy) = (k.cx - ox, k.cy - oy);
                let r = (dx * dx + dy * dy).sqrt();
                let psi = dy.atan2(dx);
                // Nearest winding of r = tight * theta through this key.
                let base = (arm.dir * (psi - arm.theta0)).rem_euclid(TAU);
                let n = ((r / tight - base) / TAU).round();
                let mut w = 0.0f32;
                let mut th_best = 0.0f32;
                for cand in [base + TAU * n, base + TAU * (n - 1.0), base + TAU * (n + 1.0)] {
                    if cand < 0.0 || cand > tip {
                        continue;
                    }
                    let d = (r - tight * cand).abs();
                    let g = (-(d * d) / (2.0 * 0.05 * 0.05)).exp();
                    if g > w {
                        w = g;
                        th_best = cand;
                    }
                }
                if w < 0.003 {
                    continue;
                }
                let prog = (th_best / tip).clamp(0.0, 1.0);
                let bright = (0.16 + 0.84 * prog.powf(1.5)) * breathe;
                let col = ctx.palette.sample(arm.shift + th_best * 0.045 + ctx.t * 0.008);
                out.add(k.led, col.scale(w * bright));
            }
            // Bright growing (or retreating) tip.
            let psi_tip = arm.theta0 + arm.dir * tip;
            let tx = ox + arm.r_tip * psi_tip.cos();
            let ty = oy + arm.r_tip * psi_tip.sin();
            let tip_gain = match arm.state {
                1 => 1.15,
                3 => 0.55,
                _ => 0.85 + 0.2 * (ctx.t * 2.6).sin(),
            };
            let tcol = ctx.palette.sample(arm.shift + tip * 0.045);
            splat(out, ctx.layout, tx, ty, 0.09, tcol.scale(tip_gain));
        }
    }
}

// ---------------------------------------------------------------- Loom Weave

#[derive(Clone, Copy)]
struct RowInfo {
    y: f32,
    x0: f32,
    x1: f32,
}

struct Loom {
    seed: u32,
    rows: Vec<RowInfo>,
    row_of: Vec<usize>, // key index -> sorted row index
    woven: Vec<f32>,
    state: u8, // 0 weaving, 1 ripple, 2 unravel
    row_i: usize,
    x: f32,
    ripple: f32,
    phase: f32,
}

impl Loom {
    fn dirp(ri: usize) -> f32 {
        if ri % 2 == 0 {
            1.0
        } else {
            -1.0
        }
    }

    fn start_x(rows: &[RowInfo], ri: usize, d: f32) -> f32 {
        if d > 0.0 {
            rows[ri].x0 - 0.1
        } else {
            rows[ri].x1 + 0.1
        }
    }
}

impl Effect for Loom {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let rate = get_f32(ctx.params, "shuttle", 1.2).clamp(0.3, 4.0);
        let contrast = get_f32(ctx.params, "contrast", 0.6).clamp(0.0, 1.0);
        let dt = ctx.dt.min(0.08);
        let keys = &ctx.layout.keys;
        let nrows = self.rows.len();

        match self.state {
            0 => {
                // Shuttle lays a thread boustrophedon, row by row.
                let d = Loom::dirp(self.row_i);
                self.x += d * rate * dt;
                for (i, k) in keys.iter().enumerate() {
                    if self.row_of[i] != self.row_i {
                        continue;
                    }
                    let ahead = if d > 0.0 { self.x - k.cx } else { k.cx - self.x };
                    let t = smoothstep(-0.06, 0.10, ahead);
                    if t > self.woven[i] {
                        self.woven[i] = t;
                    }
                }
                let row = self.rows[self.row_i];
                let done = if d > 0.0 { self.x >= row.x1 + 0.15 } else { self.x <= row.x0 - 0.15 };
                if done {
                    for i in 0..keys.len() {
                        if self.row_of[i] == self.row_i {
                            self.woven[i] = 1.0;
                        }
                    }
                    self.row_i += 1;
                    if self.row_i >= nrows {
                        self.state = 1;
                        self.ripple = 0.0;
                        self.row_i = nrows - 1;
                    } else {
                        self.x = Loom::start_x(&self.rows, self.row_i, Loom::dirp(self.row_i));
                    }
                }
            }
            1 => {
                // The finished cloth ripples once.
                self.ripple += dt;
                if self.ripple > 1.0 {
                    self.state = 2;
                    self.row_i = nrows - 1;
                    self.x = Loom::start_x(&self.rows, self.row_i, -Loom::dirp(self.row_i));
                }
            }
            _ => {
                // Unravel quickly, thread by thread, back the way it was laid.
                let d = -Loom::dirp(self.row_i);
                self.x += d * rate * 2.6 * dt;
                for (i, k) in keys.iter().enumerate() {
                    if self.row_of[i] != self.row_i {
                        continue;
                    }
                    let ahead = if d > 0.0 { self.x - k.cx } else { k.cx - self.x };
                    let t = 1.0 - smoothstep(-0.06, 0.10, ahead);
                    if t < self.woven[i] {
                        self.woven[i] = t;
                    }
                }
                let row = self.rows[self.row_i];
                let done = if d > 0.0 { self.x >= row.x1 + 0.15 } else { self.x <= row.x0 - 0.15 };
                if done {
                    for i in 0..keys.len() {
                        if self.row_of[i] == self.row_i {
                            self.woven[i] = 0.0;
                        }
                    }
                    if self.row_i == 0 {
                        // Re-weave in a new palette phase.
                        self.state = 0;
                        self.phase = (self.phase + 0.17).fract();
                        self.x = Loom::start_x(&self.rows, 0, Loom::dirp(0));
                    } else {
                        self.row_i -= 1;
                        self.x = Loom::start_x(&self.rows, self.row_i, -Loom::dirp(self.row_i));
                    }
                }
            }
        }

        let wave_y = if self.state == 1 { -0.15 + self.ripple * 1.5 } else { -10.0 };
        for (i, k) in keys.iter().enumerate() {
            let w = self.woven[i];
            // Faint vertical warp threads shimmer where cloth hasn't formed.
            let colf = if k.col % 2 == 0 { 1.0 } else { 0.5 };
            let sh = 0.05 + 0.06 * noise2(k.col as f32 * 2.3, ctx.t * 0.8, self.seed);
            let warp = ctx.palette.sample(self.phase + 0.45).scale(sh * colf);
            // Woven cloth: over/under checker in two palette phases.
            let over = (k.row as usize + k.col as usize) % 2 == 0;
            let (ph, br) = if over {
                (self.phase + 0.03, 0.62)
            } else {
                (self.phase + 0.16 + 0.22 * contrast, 0.62 - 0.30 * contrast)
            };
            let mut cloth_b = br;
            if self.state == 1 {
                let dy = k.cy - wave_y;
                cloth_b *= 1.0 + 1.2 * (-(dy * dy) / (2.0 * 0.1 * 0.1)).exp();
            }
            let cloth = ctx.palette.sample(ph).scale(cloth_b);
            out.set(k.led, Col::lerp(warp, cloth, w));
        }

        if self.state != 1 {
            let ry = self.rows[self.row_i].y;
            let gain = if self.state == 0 { 1.1 } else { 0.6 };
            splat(out, ctx.layout, self.x, ry, 0.075, ctx.palette.sample_clamped(0.93).scale(gain));
            // Fresh thread glows just behind the shuttle.
            for (i, k) in keys.iter().enumerate() {
                if self.row_of[i] != self.row_i {
                    continue;
                }
                let dx = k.cx - self.x;
                let g = (-(dx * dx) / (2.0 * 0.16 * 0.16)).exp();
                out.add(k.led, ctx.palette.sample_clamped(0.8).scale(0.5 * g * self.woven[i]));
            }
        }
    }
}

// ---------------------------------------------------------------- Orrery

const TILT: f32 = 0.5; // orbital plane seen at an angle

struct Planet {
    a: f32,
    e: f32,
    orient: f32,
    th: f32,
    pal: f32,
}

struct Orrery {
    seed: u32,
    rng: Rng,
    planets: Vec<Planet>,
    moon_th: f32,
    trail: Vec<Col>,
    comet: Option<[f32; 5]>, // x, y, vx, vy, life
}

impl Effect for Orrery {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let count = (get_f32(ctx.params, "planets", 4.0).round() as usize).clamp(2, 6);
        let trail_len = get_f32(ctx.params, "trail", 1.0).clamp(0.2, 3.0);
        let a_iso = ctx.layout.aspect;
        let dt = ctx.dt.min(0.08);

        let (sx, sy) = wander(ctx.t * 0.02, self.seed, a_iso * 0.5 - 0.35, a_iso * 0.5 + 0.35, 0.36, 0.62);

        decay(&mut self.trail, (-dt * 4.2 / trail_len).exp());

        // Integrate planets: equal-area sweep, so perihelion runs fast.
        let mut ppos: Vec<(f32, f32, f32)> = Vec::with_capacity(count); // x, y, glint
        for p in self.planets.iter_mut().take(count) {
            let om = 0.17 / p.a; // inner fast, outer slow
            let r0 = p.a * (1.0 - p.e * p.e) / (1.0 + p.e * p.th.cos());
            p.th = (p.th + om * (p.a / r0) * (p.a / r0) * dt).rem_euclid(TAU);
            let r = p.a * (1.0 - p.e * p.e) / (1.0 + p.e * p.th.cos());
            let (px, py) = (r * p.th.cos(), r * p.th.sin());
            let (s, c) = p.orient.sin_cos();
            let x = sx + px * c - py * s;
            let y = sy + (px * s + py * c) * TILT;
            let wth = (p.th + PI).rem_euclid(TAU) - PI; // 0 at perihelion
            let glint = (-(wth * wth) / (2.0 * 0.32 * 0.32)).exp();
            ppos.push((x, y, glint));
        }

        // Stamp planets (and their perihelion glints) into the trail canvas.
        for (pi, p) in self.planets.iter().take(count).enumerate() {
            let (x, y, glint) = ppos[pi];
            let base = ctx.palette.sample(p.pal);
            let col = Col::lerp(base, ctx.palette.sample_clamped(0.97), glint * 0.6);
            splat_max(&mut self.trail, ctx.layout, x, y, 0.075, col.scale(0.8 + 0.9 * glint));
        }

        // Rare comet crossing the system, bent by the sun.
        let mut kill_comet = false;
        if let Some(cm) = self.comet.as_mut() {
            let (dx, dy) = (sx - cm[0], sy - cm[1]);
            let r2 = (dx * dx + dy * dy).max(0.02);
            let rl = r2.sqrt();
            let g = 0.45 / r2;
            cm[2] += dx / rl * g * dt;
            cm[3] += dy / rl * g * dt;
            cm[0] += cm[2] * dt;
            cm[1] += cm[3] * dt;
            cm[4] -= dt;
            let ccol = Col::lerp(ctx.palette.sample_clamped(0.9), Col::WHITE, 0.35);
            splat_max(&mut self.trail, ctx.layout, cm[0], cm[1], 0.06, ccol.scale(0.9));
            kill_comet = cm[4] <= 0.0
                || cm[0] < -0.3
                || cm[0] > a_iso + 0.3
                || cm[1] < -0.3
                || cm[1] > 1.3;
        } else if self.rng.chance(dt * 0.05) {
            let from_left = self.rng.chance(0.5);
            let x = if from_left { -0.2 } else { a_iso + 0.2 };
            let y = self.rng.range(0.0, 1.0);
            let (tx, ty) = (sx + self.rng.range(-0.3, 0.3), sy + self.rng.range(-0.2, 0.2));
            let (dx, dy) = (tx - x, ty - y);
            let l = (dx * dx + dy * dy).sqrt().max(0.1);
            let sp = self.rng.range(0.55, 0.8);
            self.comet = Some([x, y, dx / l * sp, dy / l * sp, 9.0]);
        }
        if kill_comet {
            self.comet = None;
        }

        // Faint orbit rings.
        let rings: Vec<(f32, f32, f32, f32, f32, Col)> = self
            .planets
            .iter()
            .take(count)
            .map(|p| {
                let (s, c) = p.orient.sin_cos();
                let b = p.a * (1.0 - p.e * p.e).sqrt();
                (s, c, p.a, p.e, b, ctx.palette.sample(p.pal).scale(0.055))
            })
            .collect();
        for k in &ctx.layout.keys {
            let dx = k.cx - sx;
            let dy = (k.cy - sy) / TILT;
            let mut acc = Col::BLACK;
            for &(s, c, pa, pe, pb, col) in &rings {
                let rx = dx * c + dy * s;
                let ry = -dx * s + dy * c;
                let qx = (rx + pa * pe) / pa;
                let qy = ry / pb;
                let d = ((qx * qx + qy * qy).sqrt() - 1.0).abs() * pb;
                let g = (-(d * d) / (2.0 * 0.03 * 0.03)).exp();
                if g > 0.02 {
                    acc = acc.add(col.scale(g));
                }
            }
            if acc.luma() > 0.0005 {
                out.add(k.led, acc);
            }
        }

        // Trails and heads over the rings.
        for (i, k) in ctx.layout.keys.iter().enumerate() {
            out.max(k.led, self.trail[i]);
        }
        for (pi, _) in self.planets.iter().take(count).enumerate() {
            let (x, y, glint) = ppos[pi];
            let base = ctx.palette.sample(self.planets[pi].pal);
            splat(out, ctx.layout, x, y, 0.055, base.scale(0.35 + 0.8 * glint));
        }

        // The second planet carries a tiny moon.
        if count >= 2 {
            self.moon_th = (self.moon_th + dt * 3.4).rem_euclid(TAU);
            let (px, py, _) = ppos[1];
            let mx = px + 0.085 * self.moon_th.cos();
            let my = py + 0.055 * self.moon_th.sin();
            splat(out, ctx.layout, mx, my, 0.045, ctx.palette.sample_clamped(0.75).scale(0.5));
        }

        // Sun: soft corona plus a hot core, gently pulsing.
        let corona = ctx.palette.sample_clamped(0.82).scale(0.32 + 0.06 * (ctx.t * 1.7).sin());
        splat(out, ctx.layout, sx, sy, 0.16, corona);
        splat(out, ctx.layout, sx, sy, 0.065, ctx.palette.sample_clamped(0.97).scale(0.85));

        // Comet head on top.
        if let Some(cm) = self.comet {
            let ccol = Col::lerp(ctx.palette.sample_clamped(0.9), Col::WHITE, 0.5);
            splat(out, ctx.layout, cm[0], cm[1], 0.05, ccol.scale(0.6));
        }
    }
}
