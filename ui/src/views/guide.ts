import aiPrompt from "../../../docs/ai-effect-prompt.txt?raw";
import { showOnboarding } from "../onboarding";
import { store } from "../state";
import { sfx } from "../sound";

interface Section {
  icon: string;
  title: string;
  sub: string;
  body: string;
}

interface Group {
  label: string;
  sections: Section[];
}

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

const GROUPS: Group[] = [
  {
    label: "Getting started",
    sections: [
      {
        icon: "◈",
        title: "How Keyscape works",
        sub: "Two processes, one tray icon, zero cost when closed",
        body: `A tiny <b>lighting core</b> (13&nbsp;MB, ~2% of one CPU core) owns the keyboard
          and runs your effects from login to logout — this window is just a remote control and
          can be closed any time without touching the lights. The core lives in the tray:
          left-click opens this window, right-click pauses lighting or quits. Settings persist at
          <code>%APPDATA%\\Keyscape\\config.json</code>; binaries live in
          <code>%LOCALAPPDATA%\\Keyscape\\bin</code>.`,
      },
      {
        icon: "▤",
        title: "The four lighting zones",
        sub: "Keyboard · lid logo · front bar · rear strip",
        body: `<b>Keyboard</b> — 88 per-key LEDs driven at up to 30&nbsp;fps (a full hardware
          write costs ~16&nbsp;ms, which is why 30 is the cap). <b>Lid logo</b> and <b>front
          light bar</b> — mirror the scene with a brightness floor so they're always alive
          ("Aux glow" in Settings). <b>Rear lid strip</b> — hardware quirk: it only holds solid
          firmware colors, never per-LED data (verified with a per-index sweep on this machine).
          Keyscape tints it to match shortly after each effect switch and refreshes rarely; you
          can also pin it to a fixed color or turn it off (Settings → Rear bar).`,
      },
    ],
  },
  {
    label: "Lighting",
    sections: [
      {
        icon: "✺",
        title: "Effects, palettes, playlist",
        sub: "50 built-ins, 22 palettes, key masks, rotation",
        body: `Every effect is fully parameterized — speed, intensity, palette, key masks,
          plus per-effect controls — and everything edits live. The <b>Playlist</b> view rotates
          any subset on a timer, shuffled or in order. Typing-reactive effects see key
          <i>positions</i> only (never characters), and the input hook exists only while one is
          active.`,
      },
      {
        icon: "♫",
        title: "Music mode",
        sub: "Opt-in, system audio, never the microphone",
        body: `When enabled (Audio view), Keyscape analyses what's <i>playing</i> via WASAPI
          loopback and the beat modulates the current effect's speed, brightness and palette
          rather than replacing it. Custom effects receive the analysis too, via
          <code>req.audio</code>. Off by default; the capture thread only exists while enabled.`,
      },
    ],
  },
  {
    label: "Custom effects",
    sections: [
      {
        icon: "⌁",
        title: "Write your own — full tutorial",
        sub: "One .js file, live in the gallery instantly",
        body: `Effects are single <code>.js</code> files on Keyscape's embedded engine (nothing
          to install). Add them in the <b>Custom</b> tab (upload validates and reports exact
          errors) or drop files into <code>%APPDATA%\\Keyscape\\effects</code> and hit
          <i>Reload scripts</i>.<br><br>
          <b>Anatomy of an effect file:</b>
          <pre class="code">${CODE_ANATOMY.replace(/</g, "&lt;")}</pre>
          <b>Everything render() can use:</b>
          <pre class="code">${CODE_REQ.replace(/</g, "&lt;")}</pre>
          <b>Return format:</b> an array with one <code>[r,g,b]</code> (0-255) per key in
          <code>keys</code> order, or a sparse object <code>{keyIndex: [r,g,b]}</code>.<br><br>
          <b>Rules:</b> ES2020; no <code>import</code>, network, filesystem or timers. Each
          render call has a 60&nbsp;ms budget; overruns abort the frame and 10 in a row kill the
          effect (red heartbeat on Esc). The engine applies Speed, Intensity, masks and zone
          glow on top of you.<br><br>
          <b>Design for 88 LEDs:</b> features need to be a couple of keys wide (≥&nbsp;0.25
          cx/cy units); layer two or three motions at different timescales; use
          <code>req.palette</code> instead of hardcoded colors.<br><br>
          <b>Debugging:</b> upload rejections show the exact error; a red pulsing Esc means a
          runtime death — run <code>keyscape-core.exe run your_effect_id</code> in a terminal to
          see the exception.`,
      },
      {
        icon: "🤖",
        title: "Let an AI write your effect",
        sub: "No coding needed — download the prompt file",
        body: `The button below downloads a prompt file that teaches any AI chat (ChatGPT,
          Claude, Gemini…) the complete Keyscape effect contract — geometry, rules, a working
          example.<br><br>
          <b>The workflow:</b><br>
          1. Download the prompt and paste its contents into the AI chat (or attach it).<br>
          2. Describe your idea at the end — e.g. <i>"raindrops that ripple outward in blues,
          more drops when bass hits"</i>.<br>
          3. Save the reply as <code>anything.js</code> and upload it in the <b>Custom</b> tab —
          validation tells you exactly what to paste back if the AI slipped.<br><br>
          <button class="btn primary" id="guide-ai-dl">🤖 Download AI prompt (.txt)</button>`,
      },
    ],
  },
  {
    label: "Housekeeping",
    sections: [
      {
        icon: "⛨",
        title: "Armoury Crate & the ASUS service",
        sub: "Why lights can flicker, and the one-click fix",
        body: `ASUS's LightingService writes to the same device. While it runs, Keyscape
          re-sends its frame every 2 seconds so your lighting wins, but the ASUS animation can
          flash through. Settings → ASUS lighting service → <b>Disable service</b> stops it
          permanently behind one UAC prompt — reversible there too. The rest of Armoury Crate
          keeps working.`,
      },
      {
        icon: "🛠",
        title: "When something looks wrong",
        sub: "Self-healing, diagnostics, where files live",
        body: `Zones dark after sleep fix themselves within 2 seconds (state re-assert). Blank
          Start Menu icon = Windows icon cache (sign out/in). To map which LED index drives
          which physical zone, run <code>keyscape-core.exe --zone-test</code> from a terminal —
          it pauses and restarts the core itself. Full docs ship in the repo and install folder
          (<code>docs/</code>).`,
      },
    ],
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
    <div class="sub">Everything worth knowing about Keyscape${v ? ` v${v}` : ""}. Search below,
    or replay the welcome tour any time.</div>`;
  const introBtns = document.createElement("div");
  introBtns.style.cssText = "display:flex;gap:8px;flex-wrap:wrap";
  const tourBtn = document.createElement("button");
  tourBtn.className = "btn";
  tourBtn.textContent = "▶ Replay welcome tour";
  tourBtn.addEventListener("click", () => {
    sfx.click();
    showOnboarding(true);
  });
  introBtns.appendChild(tourBtn);
  intro.appendChild(introBtns);
  view.appendChild(intro);

  const searchWrap = document.createElement("div");
  searchWrap.className = "search-box";
  searchWrap.innerHTML = `<span class="search-ic">⌕</span>`;
  const search = document.createElement("input");
  search.type = "text";
  search.placeholder = "Search the guide… (rear bar, AI, palette, service…)";
  searchWrap.appendChild(search);
  view.appendChild(searchWrap);

  const groupEls: { wrap: HTMLElement; secs: { el: HTMLDetailsElement; text: string }[] }[] = [];
  for (const g of GROUPS) {
    const wrap = document.createElement("div");
    const label = document.createElement("div");
    label.className = "guide-group";
    label.textContent = g.label;
    wrap.appendChild(label);
    const secs: { el: HTMLDetailsElement; text: string }[] = [];
    for (const s of g.sections) {
      const d = document.createElement("details");
      d.className = "guide-sec";
      const sum = document.createElement("summary");
      sum.innerHTML = `<span class="gs-icon">${s.icon}</span>
        <span class="gs-text"><span class="gs-title">${s.title}</span>
        <span class="gs-sub">${s.sub}</span></span>
        <span class="gs-chev">›</span>`;
      const body = document.createElement("div");
      body.className = "guide-body";
      body.innerHTML = s.body;
      d.append(sum, body);
      wrap.appendChild(d);
      secs.push({ el: d, text: (s.title + " " + s.sub + " " + s.body).toLowerCase() });
    }
    view.appendChild(wrap);
    groupEls.push({ wrap, secs });
  }

  root.appendChild(view);

  search.addEventListener("input", () => {
    const q = search.value.trim().toLowerCase();
    for (const g of groupEls) {
      let any = false;
      for (const s of g.secs) {
        const hit = !q || s.text.includes(q);
        s.el.style.display = hit ? "" : "none";
        if (hit) any = true;
        if (q && hit) s.el.open = true;
        if (!q) s.el.open = false;
      }
      g.wrap.style.display = any ? "" : "none";
    }
  });

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
