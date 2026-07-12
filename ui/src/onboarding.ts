// First-run tour: a card-carousel overlay walking through every feature.
// Shows once (store.ui.onboarded), replayable from Settings and the Guide.

import { saveUiPrefs, store } from "./state";
import { sfx } from "./sound";

const STEPS: { icon: string; title: string; body: string }[] = [
  {
    icon: "◈",
    title: "Welcome to Keyscape",
    body: `Your keyboard's lighting now runs on a tiny always-on <b>core</b> — closing this
      window never turns the lights off. The core lives in the system tray: left-click opens
      this window, right-click pauses lighting or quits.`,
  },
  {
    icon: "✺",
    title: "52 effects and counting",
    body: `The <b>Effects</b> gallery holds 50 hand-built effects across 7 categories — plus
      yours. Click any card to apply it instantly; every parameter on the right edits live:
      speed, intensity, 22 color palettes, key masks and per-effect controls.`,
  },
  {
    icon: "▤",
    title: "All four lighting zones",
    body: `Keyboard, lid logo, front light bar — all follow the scene in real time. The
      <b>rear lid strip</b> is hardware-limited to solid colors, so it picks up a matching
      tint shortly after you switch effects (and can be set to a fixed color or off, in
      Settings → Rear bar).`,
  },
  {
    icon: "⌁",
    title: "Make your own effects",
    body: `The <b>Custom</b> tab accepts JavaScript effect files — upload, and they're live
      instantly. Can't code? Download the <b>AI prompt file</b> there, paste it into ChatGPT
      or any AI with a one-line idea, and upload what it writes. The Guide has the full
      tutorial.`,
  },
  {
    icon: "♫",
    title: "Playlist & music mode",
    body: `<b>Playlist</b> rotates any set of effects on a timer. <b>Audio</b> makes the
      current effect dance to whatever's playing — strictly opt-in, captures system audio
      (never the microphone), and fully off until you enable it.`,
  },
  {
    icon: "⚙",
    title: "Make it yours",
    body: `<b>Settings</b> covers everything — brightness, frame rate, transitions, accent
      colors, fonts, interface sounds, autostart — with search to find any option fast. One
      recommended stop: <i>ASUS lighting service → Disable service</i> keeps Armoury Crate
      from fighting over the LEDs.`,
  },
];

export function showOnboarding(force = false) {
  if (!force && store.ui.onboarded) return;
  if (document.getElementById("onboard")) return;

  let idx = 0;
  const overlay = document.createElement("div");
  overlay.id = "onboard";
  overlay.innerHTML = `
    <div class="ob-card">
      <div class="ob-icon"></div>
      <h2 class="ob-title"></h2>
      <div class="ob-body"></div>
      <div class="ob-dots"></div>
      <div class="ob-actions">
        <button class="btn" id="ob-skip">Skip tour</button>
        <div style="flex:1"></div>
        <button class="btn" id="ob-back">‹ Back</button>
        <button class="btn primary" id="ob-next">Next ›</button>
      </div>
    </div>`;
  document.body.appendChild(overlay);

  const el = (s: string) => overlay.querySelector(s) as HTMLElement;
  const paint = () => {
    const s = STEPS[idx];
    el(".ob-icon").textContent = s.icon;
    el(".ob-title").textContent = s.title;
    el(".ob-body").innerHTML = s.body;
    el(".ob-dots").innerHTML = STEPS.map(
      (_, i) => `<span class="ob-dot${i === idx ? " on" : ""}"></span>`
    ).join("");
    el("#ob-back").style.visibility = idx === 0 ? "hidden" : "visible";
    el("#ob-next").textContent = idx === STEPS.length - 1 ? "Get started ✦" : "Next ›";
  };
  const finish = () => {
    overlay.remove();
    store.ui.onboarded = true;
    saveUiPrefs();
  };

  el("#ob-skip").addEventListener("click", () => {
    sfx.click();
    finish();
  });
  el("#ob-back").addEventListener("click", () => {
    sfx.click();
    idx = Math.max(0, idx - 1);
    paint();
  });
  el("#ob-next").addEventListener("click", () => {
    if (idx === STEPS.length - 1) {
      sfx.select();
      finish();
    } else {
      sfx.click();
      idx++;
      paint();
    }
  });
  paint();
}
