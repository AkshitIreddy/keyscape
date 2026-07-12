// Keyscape example effect: sparks fly sideways from every key you press.
// Demonstrates the taps stream, persistent state, and needs_input.

const EFFECT = {
  id: "js_typing_sparks",
  name: "JS Typing Sparks",
  category: "Custom",
  blurb: "Example JavaScript effect: keypresses shoot sideways sparks.",
  palette: "ember",
  needs_input: true,
  params: [
    { key: "sparks", label: "Sparks per key", kind: "slider", min: 1, max: 8, step: 1, default: 4 },
    { key: "fade", label: "Fade speed", kind: "slider", min: 0.5, max: 4.0, step: 0.1, default: 1.6 },
  ],
};

function setup() {
  state.sparks = []; // each: {x, y, vx, life}
  state.rnd = seed >>> 0 || 1;
}

// tiny deterministic PRNG so the effect looks the same for a given seed
function rand() {
  state.rnd = (state.rnd * 1664525 + 1013904223) >>> 0;
  return state.rnd / 4294967296;
}

function render(req) {
  const nPer = Math.round(req.params.sparks ?? 4);
  const fade = req.params.fade ?? 1.6;
  const pal = req.palette;

  for (const [_, cx, cy] of req.taps) {
    for (let i = 0; i < nPer; i++) {
      state.sparks.push({
        x: cx, y: cy,
        vx: (0.4 + rand() * 1.2) * (rand() < 0.5 ? -1 : 1),
        life: 1.0,
      });
    }
  }

  state.sparks = state.sparks
    .map((sp) => ({ ...sp, x: sp.x + sp.vx * req.dt, life: sp.life - req.dt * fade }))
    .filter((sp) => sp.life > 0)
    .slice(-200);

  return keys.map((k) => {
    // faint idle floor so the board never looks dead
    const base = pal[2];
    let r = base[0] * 0.04, g = base[1] * 0.04, b = base[2] * 0.04;
    for (const sp of state.sparks) {
      const d2 = (k.cx - sp.x) ** 2 + (k.cy - sp.y) ** 2;
      if (d2 < 0.04) {
        const w = sp.life * Math.max(0, 1 - d2 / 0.04);
        const c = pal[Math.min(15, Math.floor(sp.life * 15))];
        r += c[0] * w;
        g += c[1] * w;
        b += c[2] * w;
      }
    }
    return [r, g, b];
  });
}
