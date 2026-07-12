# Writing your own effects in JavaScript

Keyscape embeds a QuickJS engine in the lighting core, so custom effects are
plain `.js` files — nothing to install, and a buggy script can't take the
engine down (every frame eval runs under a 60 ms interrupt budget).

## Quick start

1. Create `%APPDATA%\Keyscape\effects\my_effect.js` (the installer seeds this
   folder with two examples).
2. Restart the core: tray icon → *Quit lighting core*, then launch Keyscape.
3. Your effect appears in the gallery under its manifest category
   (default **Custom**), params included.

## Anatomy of an effect

```js
const EFFECT = {
  id: "my_waves",              // unique, snake_case
  name: "My Waves",
  category: "Custom",          // optional
  blurb: "What it looks like", // optional
  palette: "oceanic",          // default palette id (see docs/effects.md)
  needs_input: false,          // true = receive key taps
  params: [                    // optional — UI renders these automatically
    { key: "scale", label: "Scale", kind: "slider",
      min: 0.5, max: 3, step: 0.1, default: 1.0 },
    { key: "flip", label: "Flip", kind: "toggle", default: false },
    { key: "mode", label: "Mode", kind: "select",
      options: ["soft", "sharp"], default: "soft" },
  ],
};

function setup() {
  // optional, runs once; `state` is yours to fill
  state.phase = 0;
}

function render(req) {
  // called ~30x/second; must return one [r, g, b] (0-255) per key,
  // in `keys` order — or a sparse { keyIndex: [r, g, b] } object.
  return keys.map(k => {
    const f = (Math.sin(k.cx * 4 + req.t) + 1) / 2;
    const c = req.palette[Math.floor(f * 15)];
    return [c[0] * f, c[1] * f, c[2] * f];
  });
}
```

## What you get

**Globals** (set once before `setup()`):

| Global | Contents |
| --- | --- |
| `keys` | Array of `{i, led, cx, cy, row, col, name}` — real physical geometry. `cx` runs 0…≈2.48 left→right, `cy` 0…1 top→bottom, one key pitch ≈ 0.155 |
| `state` | Empty object that persists across frames — your scratch space |
| `seed` | Random-ish integer, stable for the life of the effect instance |

**Per frame**, `render(req)` receives:

| Field | Contents |
| --- | --- |
| `req.t`, `req.dt` | Effect-local time / delta, already scaled by the user's Speed slider (don't add your own speed param) |
| `req.params` | Current values of your manifest params |
| `req.palette` | 16 `[r, g, b]` samples of the user's chosen palette — use it so themes work |
| `req.taps` | Key-down events since last frame: `[[keyIndex, cx, cy], …]` (needs `needs_input: true`) |
| `req.audio` | `{level, bass, mid, treble, beat}` when music mode is on, else `null` |

## Rules of the road

- The engine applies Intensity, key masks, transitions, and the logo/bars
  glow — don't reimplement them.
- Keep `render` under ~60 ms (that's a *lot* of arithmetic for 88 keys; the
  examples run in well under 1 ms). Overruns abort that frame; 10 consecutive
  failures mark the effect dead — shown as a red heartbeat on Esc.
- Language level is ES2020 (QuickJS): classes, arrow functions, spread,
  optional chaining all fine. No `import`, no network, no filesystem, no
  timers — one `render` call per frame is the whole world.
- Scripts run inside the core process with your user privileges. Only add
  files you trust.

## Debugging

- Manifest or syntax errors are printed to the core's console: run
  `keyscape-core.exe run your_effect_id` from a terminal to see them.
- A red pulsing Esc key means the script died (exception, timeout streak, or
  bad return shape).
