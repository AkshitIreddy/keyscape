import { core } from "../ipc";
import { toggle } from "../params";
import { patchSettings, refreshStatus, saveUiPrefs, store } from "../state";
import { sfx } from "../sound";

function row(lbl: string, hint: string, ctl: HTMLElement): HTMLElement {
  const r = document.createElement("div");
  r.className = "row";
  const left = document.createElement("div");
  left.innerHTML = `<div class="lbl">${lbl}</div>${hint ? `<div class="hint">${hint}</div>` : ""}`;
  r.append(left, ctl);
  return r;
}

function slider(
  min: number,
  max: number,
  step: number,
  value: number,
  onInput: (v: number) => void,
  fmt: (v: number) => string = (v) => v.toFixed(step >= 1 ? 0 : 2)
): HTMLElement {
  const wrap = document.createElement("div");
  wrap.style.cssText = "display:flex;align-items:center;gap:10px";
  const val = document.createElement("span");
  val.className = "val";
  val.style.minWidth = "44px";
  const r = document.createElement("input");
  r.type = "range";
  r.min = String(min);
  r.max = String(max);
  r.step = String(step);
  r.value = String(value);
  r.style.width = "170px";
  const paint = () => {
    val.textContent = fmt(Number(r.value));
    r.style.setProperty("--fill", `${((Number(r.value) - min) / (max - min)) * 100}%`);
  };
  paint();
  r.addEventListener("input", () => {
    paint();
    onInput(Number(r.value));
  });
  wrap.append(r, val);
  return wrap;
}

export function renderSettings(root: HTMLElement): (() => void) | void {
  root.innerHTML = "";
  const view = document.createElement("div");
  view.className = "view";
  const s = store.status?.settings ?? {};

  const grid = document.createElement("div");
  grid.className = "settings-grid";

  // ---------------- General
  const general = document.createElement("div");
  general.className = "panel";
  general.innerHTML = `<h3>General</h3><div class="sub">Lighting engine behavior.</div>`;

  const bright = document.createElement("select");
  bright.style.width = "120px";
  bright.innerHTML = `<option value="1">Low</option><option value="2">Medium</option><option value="3">High</option>`;
  bright.value = String(s.brightness ?? 3);
  bright.addEventListener("change", () => {
    sfx.click();
    patchSettings("brightness", { brightness: Number(bright.value) }, 60);
  });
  general.appendChild(row("Hardware brightness", "The keyboard's own LED brightness level.", bright));

  general.appendChild(
    row(
      "Master intensity",
      "Software dimmer applied to every effect.",
      slider(0.1, 1, 0.05, s.master ?? 1, (v) => patchSettings("master", { master: v }))
    )
  );
  general.appendChild(
    row(
      "Pause lighting",
      "Blanks the board without stopping the core.",
      toggle(Boolean(s.paused), (v) => patchSettings("paused", { paused: v }, 40))
    )
  );
  general.appendChild(
    row(
      "Aux glow",
      "Mirror the scene onto the lid logo and front light bar.",
      toggle(s.aux_glow ?? true, (v) => patchSettings("aux_glow", { aux_glow: v }))
    )
  );
  general.appendChild(
    row(
      "Typing effects",
      "Allow typing-reactive effects to see key positions (never characters; never leaves the engine).",
      toggle(s.input_reactive ?? true, (v) => patchSettings("input_reactive", { input_reactive: v }))
    )
  );
  grid.appendChild(general);

  // ---------------- ASUS service guard
  const guard = document.createElement("div");
  guard.className = "panel";
  guard.innerHTML = `<h3>ASUS lighting service</h3>
    <div class="sub">Armoury Crate's LightingService fights over the same device. Keyscape counters
    with periodic re-sends (works, but the ASUS animation can flash through for a moment). Stopping
    the service entirely is cleaner.</div>`;
  const status = document.createElement("div");
  status.className = "row";
  status.innerHTML = `<div class="lbl">Service status</div><div class="kv"><b id="guard-state">â€¦</b></div>`;
  guard.appendChild(status);
  guard.appendChild(
    row(
      "Manage while running",
      "Try to stop the service when the core starts (needs elevation) and restore it on exit.",
      toggle(s.guard?.manage_lighting_service ?? true, (v) =>
        patchSettings("guard", { guard: { manage_lighting_service: v } })
      )
    )
  );
  const fixRow = document.createElement("div");
  fixRow.className = "row";
  fixRow.innerHTML = `<div><div class="lbl">Permanent fix</div>
    <div class="hint">Stops and disables LightingService via a UAC prompt. Your other Armoury Crate
    functions keep working; only lighting control transfers to Keyscape. Reversible any time.</div></div>`;
  const btns = document.createElement("div");
  btns.style.cssText = "display:flex;gap:8px;flex:none";
  const fixBtn = document.createElement("button");
  fixBtn.className = "btn primary";
  fixBtn.textContent = "Disable service";
  fixBtn.addEventListener("click", async () => {
    sfx.select();
    await core.req("guard_fix");
  });
  const restoreBtn = document.createElement("button");
  restoreBtn.className = "btn";
  restoreBtn.textContent = "Re-enable";
  restoreBtn.addEventListener("click", async () => {
    sfx.click();
    await core.req("guard_restore");
  });
  btns.append(fixBtn, restoreBtn);
  fixRow.appendChild(btns);
  guard.appendChild(fixRow);
  grid.appendChild(guard);

  // ---------------- Appearance
  const appear = document.createElement("div");
  appear.className = "panel";
  appear.innerHTML = `<h3>Appearance & sound</h3><div class="sub">How the app itself feels.</div>`;
  appear.appendChild(
    row(
      "Interface sounds",
      "Subtle synthesized ticks and chimes.",
      toggle(store.ui.sounds, (v) => {
        store.ui.sounds = v;
        saveUiPrefs();
      })
    )
  );
  appear.appendChild(
    row(
      "Sound volume",
      "",
      slider(0.05, 1, 0.05, store.ui.volume, (v) => {
        store.ui.volume = v;
        saveUiPrefs();
      })
    )
  );
  appear.appendChild(
    row(
      "Motion",
      "Background drift and view transitions. Honors your OS reduced-motion setting too.",
      toggle(store.ui.motion, (v) => {
        store.ui.motion = v;
        document.documentElement.dataset.motion = v ? "on" : "off";
        saveUiPrefs();
      })
    )
  );
  appear.appendChild(
    row(
      "Preview glow",
      "Bloom around bright keys in the live preview.",
      toggle(store.ui.glow, (v) => {
        store.ui.glow = v;
        saveUiPrefs();
      })
    )
  );
  grid.appendChild(appear);

  // ---------------- Performance & about
  const perf = document.createElement("div");
  perf.className = "panel";
  perf.innerHTML = `<h3>Performance</h3><div class="sub">The engine only touches the USB bus when
    pixels change, and drops to 4 fps when the scene is static.</div>`;
  perf.appendChild(
    row(
      "Frame rate cap",
      "A full-board HID write takes ~16 ms, so 30 fps is the hardware sweet spot.",
      slider(15, 60, 5, s.fps ?? 30, (v) => patchSettings("fps", { fps: v }), (v) => `${v} fps`)
    )
  );
  perf.appendChild(
    row(
      "Gamma",
      "Perceptual-to-LED response curve.",
      slider(1.0, 2.6, 0.1, s.gamma ?? 1.8, (v) => patchSettings("gamma", { gamma: v }))
    )
  );
  const about = document.createElement("div");
  about.className = "row";
  const st = store.status;
  about.innerHTML = `<div><div class="lbl">Keyscape core</div>
    <div class="hint">${store.effects.length} effects Â· HID ${st?.hid_connected ? "connected" : "disconnected"} Â·
    uptime ${Math.floor((st?.uptime_sec ?? 0) / 60)} min Â· ROG Strix SCAR 16 (G634JZ)</div></div>`;
  perf.appendChild(about);
  grid.appendChild(perf);

  view.appendChild(grid);
  root.appendChild(view);

  // async guard status
  void core.req("guard_running").then((r) => {
    const el = document.getElementById("guard-state");
    if (el && r.ok) {
      el.textContent = r.running ? "Running (contending)" : "Stopped";
      (el as HTMLElement).style.color = r.running ? "var(--danger)" : "var(--acc1)";
      store.guardRunning = r.running;
    }
  });

  const t = window.setInterval(() => void refreshStatus(), 3000);
  return () => clearInterval(t);
}
