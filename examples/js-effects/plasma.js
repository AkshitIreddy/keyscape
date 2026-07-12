// Keyscape example effect: classic plasma, tinted by the user's palette.
// Copy into  %APPDATA%\Keyscape\effects\  and restart the core (tray > Quit
// lighting core, then relaunch Keyscape) to see it under "Custom".
//
// Globals: keys = [{i, led, cx, cy, row, col, name}], state = {}, seed.
// render(req) gets {t, dt, params, palette (16 [r,g,b] samples), taps, audio}
// and returns one [r, g, b] (0-255) per key.

const EFFECT = {
  id: "js_plasma",
  name: "JS Plasma",
  category: "Custom",
  blurb: "Example JavaScript effect: three interfering sine fields.",
  palette: "synthwave",
  params: [
    { key: "scale", label: "Scale", kind: "slider", min: 0.5, max: 3.0, step: 0.1, default: 1.4 },
    { key: "warp", label: "Warp", kind: "slider", min: 0.0, max: 1.0, step: 0.05, default: 0.5 },
  ],
};

function render(req) {
  const s = (req.params.scale ?? 1.4) * 3.0;
  const warp = req.params.warp ?? 0.5;
  const pal = req.palette;
  const t = req.t;

  return keys.map((k) => {
    const v =
      Math.sin(k.cx * s + t) +
      Math.sin((k.cy * s + t * 1.31) * 1.7) +
      Math.sin((k.cx + k.cy) * s * 0.8 + t * 0.7 + warp * Math.sin(t * 0.4) * 4);
    const f = (v + 3) / 6; // 0..1
    const c = pal[Math.min(15, Math.floor(f * 16))];
    const bright = 0.35 + 0.65 * f;
    return [c[0] * bright, c[1] * bright, c[2] * bright];
  });
}
