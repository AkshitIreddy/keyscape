import aiPrompt from "../../../docs/ai-effect-prompt.txt?raw";
import { store } from "../state";
import { sfx } from "../sound";

const CODE_ANATOMY = `const EFFECT = {
  id: "my_waves",              // unique snake_case id
  name: "My Waves",
  category: "Custom",          // optional gallery category
  blurb: "What it looks like", // optional
  palette: "oceanic",          // default palette id
  needs_input: false,          // true = receive key taps
  params: [                    // optional; UI builds the controls
    { key: "scale", label: "Scale", kind: "slider",
      min: 0.5, max: 3, step: 0.1, default: 1.0 },
    { key: "flip", label: "Flip", kind: "toggle", default: false },
    { key: "mode", label: "Mode", kind: "select",
      options: ["soft", "sharp"], default: "soft" },
  ],
};

function setup() {          // optional, runs once
  state.phase = 0;          // \`state\` is yours, persists between frames
}

function render(req) {      // ~30x per second
  return keys.map(k => {
    const f = (Math.sin(k.cx * 4 + req.t) + 1) / 2;
    const c = req.palette[Math.floor(f * 15)];
    return [c[0] * f, c[1] * f, c[2] * f];   // [r,g,b] 0-255 per key
  });
}`;

const CODE_REQ = `req.t / req.dt   time + delta, ALREADY speed-scaled — never add
                 your own speed param
req.params       current values of your EFFECT.params
req.palette      16 [r,g,b] samples of the user's palette — use it,
                 so your effect follows their theme
req.taps         presses since last frame: [[keyIndex, cx, cy], ...]
                 (only with needs_input: true)
req.audio        {level, bass, mid, treble, beat} in music mode, else null

keys             88 keys: {i, led, cx, cy, row, col, name}
                 cx: 0 (left) → ~2.48 (right), cy: 0 (top) → 1 (bottom)
                 one key pitch ≈ 0.155; rows 0-6 top→bottom
state            your persistent scratch object
seed             stable random integer for this run`;

const SECTIONS: { title: string; body: string }[] = [
  {
    title: "How Keyscape works",
    body: `Two processes. A tiny <b>lighting core</b> (13&nbsp;MB, ~2% of one CPU core)
      owns the keyboard and runs your effects from login to logout — this window is just a
      remote control and can be closed any time without touching the lights. The core lives in
      the tray: left-click opens this window, right-click pauses lighting or quits the core.
      Settings persist at <code>%APPDATA%\\Keyscape\\config.json</code>; binaries live in
      <code>%LOCALAPPDATA%\\Keyscape\\bin</code>.`,
  },
  {
    title: "The four lighting zones",
    body: `<b>Keyboard</b> — 88 per-key LEDs driven at up to 30&nbsp;fps (a full hardware
      write costs ~16&nbsp;ms, which is why 30 is the cap). <b>Lid logo</b> and <b>front light
      bar</b> — mirror the scene with a brightness floor so they're always alive ("Aux glow" in
      Settings). <b>Rear lid strip</b> — hardware quirk: it only obeys the keyboard's built-in
      firmware effects, never per-LED data (verified with a per-index sweep on this machine).
      Keyscape therefore saves a matching solid color into the firmware right after you switch
      effects and refreshes it at most once a minute — the brief whole-board blink when that
      happens is the hardware's cost of doing business, and the strip's color intentionally
      lags the scene.`,
  },
  {
    title: "Effects, palettes, playlist",
    body: `50 built-in effects across 7 categories, every one parameterized — speed,
      intensity, palette (22 built-ins), key masks, plus per-effect controls, all editing
      live. The <b>Playlist</b> view rotates any subset on a timer, shuffled or in order.
      Typing-reactive effects see key <i>positions</i> only (never characters), and the input
      hook exists only while one is active. Music-reactive styling is opt-in and modulates the
      current effect instead of replacing it.`,
  },
  {
    title: "Write your own effects — full tutorial",
    body: `Effects are single <code>.js</code> files running on Keyscape's embedded engine
      (nothing to install). Add them in the <b>Custom</b> tab (upload button) or drop files into
      <code>%APPDATA%\\Keyscape\\effects</code> and hit <i>Reload scripts</i> — they appear in
      the gallery instantly.<br><br>
      <b>Anatomy of an effect file:</b>
      <pre class="code">${CODE_ANATOMY.replace(/</g, "&lt;")}</pre>
      <b>Everything render() can use:</b>
      <pre class="code">${CODE_REQ.replace(/</g, "&lt;")}</pre>
      <b>Return format:</b> an array with one <code>[r,g,b]</code> (0-255) per key in
      <code>keys</code> order, or a sparse object <code>{keyIndex: [r,g,b]}</code> where
      missing keys stay black.<br><br>
      <b>Rules:</b> ES2020 JavaScript; no <code>import</code>, network, filesystem or timers —
      one render call per frame is the whole world. Each call has a 60&nbsp;ms budget;
      overruns abort the frame and 10 in a row kill the effect (shown as a red heartbeat on
      Esc). The engine applies Speed, Intensity, key masks and zone glow on top of you — don't
      reimplement them.<br><br>
      <b>Design for 88 LEDs:</b> features need to be at least a couple of keys wide
      (≥&nbsp;0.25 in cx/cy units) to read. Layer two or three motions at different timescales
      and lean on <code>req.palette</code> instead of hardcoded colors — that's what makes an
      effect feel native here.<br><br>
      <b>Debugging:</b> upload rejections show the exact manifest/syntax error. A red pulsing
      Esc means the script died at runtime — run
      <code>keyscape-core.exe run your_effect_id</code> in a terminal to see the exception.
      The repo's <code>docs/js-effects.md</code> is the full reference.`,
  },
  {
    title: "Let an AI write your effect",
    body: `You don't need to know JavaScript. The button below downloads a prompt file that
      teaches any AI chat (ChatGPT, Claude, Gemini, anything) exactly how Keyscape effects
      work — the full contract, geometry, rules and a working example.<br><br>
      <b>The workflow:</b><br>
      1. Download the prompt file and paste its contents into the AI chat (or attach it).<br>
      2. Describe your idea at the end — e.g. <i>"raindrops that ripple outward in blues, more
      drops when bass hits"</i>.<br>
      3. Save the AI's reply as <code>anything.js</code> and add it via the <b>Custom</b> tab
      (the upload validates it and tells you exactly what's wrong if the AI slipped).<br>
      4. Tweak by pasting error messages or wishes back into the chat.<br><br>
      <button class="btn primary" id="guide-ai-dl">🤖 Download AI prompt (.txt)</button>`,
  },
  {
    title: "Music mode",
    body: `Strictly opt-in (Audio view). When enabled, Keyscape listens to what's
      <i>playing</i> via WASAPI loopback — never the microphone — and the beat modulates the
      current effect's speed, brightness and palette. Custom effects get the analysis too, via
      <code>req.audio</code>. Turn it off and the capture thread is gone.`,
  },
  {
    title: "Armoury Crate & the ASUS service",
    body: `ASUS's LightingService writes to the same device. While it runs, Keyscape re-sends
      its frame every 2 seconds so your lighting wins, but the ASUS animation can flash
      through. Settings → ASUS lighting service → <b>Disable service</b> stops it permanently
      behind one UAC prompt — reversible there too. Everything else in Armoury Crate keeps
      working.`,
  },
  {
    title: "When something looks wrong",
    body: `Zones dark after sleep fix themselves within 2 seconds (the core re-asserts
      hardware state). Blank Start Menu icon = Windows icon cache (sign out/in). To map which
      LED index drives which physical zone, run
      <code>keyscape-core.exe --zone-test</code> from a terminal — it pauses and restarts the
      core by itself. More: <code>docs/troubleshooting.md</code> in the repo / install folder.`,
  },
];

export function renderGuide(root: HTMLElement): (() => void) | void {
  root.innerHTML = "";
  const view = document.createElement("div");
  view.className = "view";

  const intro = document.createElement("div");
  intro.className = "panel";
  const v = store.status?.version ?? "";
  intro.innerHTML = `<h3>Guide</h3>
    <div class="sub">Everything worth knowing about Keyscape${v ? ` v${v}` : ""}. The deepest
    section is the custom-effects tutorial — and if you'd rather not code, the AI prompt file
    turns any chatbot into your effect author.</div>`;
  view.appendChild(intro);

  for (const s of SECTIONS) {
    const d = document.createElement("details");
    d.className = "guide-sec";
    const sum = document.createElement("summary");
    sum.textContent = s.title;
    const body = document.createElement("div");
    body.className = "guide-body";
    body.innerHTML = s.body;
    d.append(sum, body);
    view.appendChild(d);
  }

  root.appendChild(view);

  document.getElementById("guide-ai-dl")?.addEventListener("click", () => {
    sfx.click();
    const blob = new Blob([aiPrompt], { type: "text/plain" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = "keyscape-ai-effect-prompt.txt";
    a.click();
    URL.revokeObjectURL(a.href);
  });
}
