import { store } from "../state";

const SECTIONS: { title: string; body: string }[] = [
  {
    title: "How Keyscape works",
    body: `Two processes. A tiny <b>lighting core</b> (13&nbsp;MB, ~2% of one CPU core)
      owns the keyboard and runs your effects from login to logout — this window is just a
      remote control and can be closed any time without touching the lights. The core lives in
      the tray: left-click opens this window, right-click pauses lighting or quits the core.`,
  },
  {
    title: "The four lighting zones",
    body: `<b>Keyboard</b> — 88 per-key LEDs, the star of the show. <b>Lid logo</b> and
      <b>front light bar</b> — mirror the scene with a brightness floor so they're always
      alive ("Aux glow" in Settings). <b>Rear lid strip</b> — special: the hardware only lets
      firmware effects color it, so Keyscape repaints it with a matching solid color at most
      every ~12 seconds; the one-frame blink when that happens is the hardware's cost of
      business, and its color intentionally lags the scene.`,
  },
  {
    title: "Effects, palettes, playlist",
    body: `50 built-in effects across 7 categories, every one parameterized — speed,
      intensity, palette (22 built-ins), key masks, plus per-effect controls. Everything edits
      live. The <b>Playlist</b> view rotates any subset on a timer, shuffled or in order.
      Typing-reactive effects see key <i>positions</i> only (never characters), and the input
      hook exists only while one is active.`,
  },
  {
    title: "Write your own effects (JavaScript)",
    body: `Drop a <code>.js</code> file into <code>%APPDATA%\\Keyscape\\effects</code> —
      two examples are already there. A file is an <code>EFFECT</code> manifest (name,
      params…) plus a <code>render(req)</code> function returning one <code>[r,g,b]</code>
      per key at ~30&nbsp;fps, with the user's palette, key taps and audio features handed in.
      Scripts run on an embedded engine (nothing to install) under a 60&nbsp;ms frame budget —
      a buggy script shows a red heartbeat on Esc instead of breaking the board. Restart the
      core (tray → Quit, relaunch) to pick up new files. Full guide:
      <code>docs/js-effects.md</code> in the repo.`,
  },
  {
    title: "Music mode",
    body: `Strictly opt-in (Audio view). When enabled, Keyscape listens to what's
      <i>playing</i> via WASAPI loopback — never the microphone — and the beat modulates the
      current effect's speed, brightness and palette rather than replacing it. Turn it off and
      the capture thread is gone.`,
  },
  {
    title: "Armoury Crate & the ASUS service",
    body: `ASUS's LightingService writes to the same device. While it runs, Keyscape
      re-sends its frame every 2 seconds so your lighting wins, but the ASUS animation can
      flash through. Settings → ASUS lighting service → <b>Disable service</b> stops it
      permanently behind one UAC prompt — reversible there too. Everything else in Armoury
      Crate keeps working.`,
  },
  {
    title: "When something looks wrong",
    body: `Zones dark after sleep fix themselves within 2 seconds (the core re-asserts
      hardware state). Blank Start Menu icon = Windows icon cache (sign out/in). To map which
      LED index drives which physical zone, run
      <code>keyscape-core.exe --zone-test</code> from a terminal — it pauses and restarts the
      core by itself. More: <code>docs/troubleshooting.md</code>.`,
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
    <div class="sub">Everything worth knowing about Keyscape${v ? ` v${v}` : ""}, in two minutes.
    Full documentation lives in the repo's <code>docs/</code> folder.</div>`;
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
}
