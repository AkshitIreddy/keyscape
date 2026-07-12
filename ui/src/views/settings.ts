import { core } from "../ipc";
import { showOnboarding } from "../onboarding";
import { toggle } from "../params";
import { ACCENTS, applyUiPrefs, patchSettings, refreshStatus, saveUiPrefs, store } from "../state";
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

function select(
  options: [string, string][],
  value: string,
  onChange: (v: string) => void,
  width = "150px"
): HTMLElement {
  const sel = document.createElement("select");
  sel.style.width = width;
  for (const [v, label] of options) {
    const o = document.createElement("option");
    o.value = v;
    o.textContent = label;
    if (v === value) o.selected = true;
    sel.appendChild(o);
  }
  sel.addEventListener("change", () => {
    sfx.click();
    onChange(sel.value);
  });
  return sel;
}

export function renderSettings(root: HTMLElement): (() => void) | void {
  root.innerHTML = "";
  const view = document.createElement("div");
  view.className = "view";
  const s = store.status?.settings ?? {};

  // ---------------- search
  const searchWrap = document.createElement("div");
  searchWrap.className = "search-box";
  searchWrap.innerHTML = `<span class="search-ic">⌕</span>`;
  const search = document.createElement("input");
  search.type = "text";
  search.placeholder = "Search settings… (brightness, accent, autostart, rear bar…)";
  searchWrap.appendChild(search);
  view.appendChild(searchWrap);

  const grid = document.createElement("div");
  grid.className = "settings-grid";

  // ---------------- General
  const general = document.createElement("div");
  general.className = "panel";
  general.innerHTML = `<h3>General</h3><div class="sub">Lighting engine behavior.</div>`;

  general.appendChild(
    row(
      "Hardware brightness",
      "The keyboard's own LED brightness level.",
      select(
        [["1", "Low"], ["2", "Medium"], ["3", "High"]],
        String(s.brightness ?? 3),
        (v) => patchSettings("brightness", { brightness: Number(v) }, 60),
        "120px"
      )
    )
  );
  general.appendChild(
    row(
      "Master intensity",
      "Software dimmer applied to every effect.",
      slider(0.1, 1, 0.05, s.master ?? 1, (v) => patchSettings("master", { master: v }))
    )
  );
  general.appendChild(
    row(
      "Effect transition",
      "Crossfade length when switching effects.",
      slider(0.1, 2, 0.1, s.transition ?? 0.4, (v) => patchSettings("transition", { transition: v }), (v) => `${v.toFixed(1)} s`)
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
      "Rear bar (experimental)",
      "Hardware limit: the G634's rear strip is a firmware-effect-only zone and can't hold a color while the keyboard streams per-key data. These modes briefly flash the board to paint it and the color won't reliably persist — off is recommended. See Guide → Known limitations.",
      select(
        [["off", "Off (recommended)"], ["static", "Fixed color"], ["follow", "Follow effect"]],
        s.rear?.mode ?? "off",
        (v) => patchSettings("rear.mode", { rear: { mode: v } }, 60)
      )
    )
  );
  {
    const color = document.createElement("input");
    color.type = "color";
    color.value = s.rear?.color ?? "#7C5CFF";
    color.className = "color-input";
    color.addEventListener("input", () =>
      patchSettings("rear.color", { rear: { color: color.value.toUpperCase() } }, 300)
    );
    general.appendChild(
      row("Rear bar fixed color", "Used when the rear bar is set to Fixed color.", color)
    );
  }
  general.appendChild(
    row(
      "Typing effects",
      "Allow typing-reactive effects to see key positions (never characters; never leaves the engine).",
      toggle(s.input_reactive ?? true, (v) => patchSettings("input_reactive", { input_reactive: v }))
    )
  );
  general.appendChild(
    row(
      "Start with Windows",
      "Launch the lighting core at login so effects survive reboots.",
      toggle(s.autostart ?? true, (v) => patchSettings("autostart", { autostart: v }, 60))
    )
  );
  grid.appendChild(general);

  // ---------------- ASUS service guard
  const guard = document.createElement("div");
  guard.className = "panel";
  guard.innerHTML = `<h3>ASUS lighting service</h3>
    <div class="sub">Armoury Crate's LightingService fights over the same device. Stopping it
    is the clean fix; until then Keyscape counters with periodic re-sends.</div>`;
  const status = document.createElement("div");
  status.className = "row";
  status.innerHTML = `<div class="lbl">Service status</div><div class="kv"><b id="guard-state">…</b></div>`;
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
    <div class="hint">Stops and disables LightingService via a UAC prompt. Reversible any time;
    the rest of Armoury Crate keeps working.</div></div>`;
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

  // ---------------- Appearance & sound
  const appear = document.createElement("div");
  appear.className = "panel";
  appear.innerHTML = `<h3>Appearance & sound</h3><div class="sub">How the app itself looks, feels and sounds.</div>`;

  {
    const swatches = document.createElement("div");
    swatches.style.cssText = "display:flex;gap:8px;flex-wrap:wrap;flex:none";
    for (const [id, [a1, a2]] of Object.entries(ACCENTS)) {
      const b = document.createElement("button");
      b.className = "accent-swatch" + (store.ui.accent === id ? " on" : "");
      b.title = id;
      b.style.background = `linear-gradient(135deg, ${a1}, ${a2})`;
      b.addEventListener("click", () => {
        sfx.click();
        store.ui.accent = id;
        applyUiPrefs();
        saveUiPrefs();
        swatches.querySelectorAll(".accent-swatch").forEach((x) => x.classList.remove("on"));
        b.classList.add("on");
      });
      swatches.appendChild(b);
    }
    appear.appendChild(row("Accent color", "Recolors every gradient and highlight in the app.", swatches));
  }
  appear.appendChild(
    row(
      "Font",
      "Interface typeface.",
      select(
        [["default", "Segoe (default)"], ["classic", "Classic serif"], ["mono", "Monospace"]],
        store.ui.font,
        (v) => {
          store.ui.font = v;
          applyUiPrefs();
          saveUiPrefs();
        }
      )
    )
  );
  appear.appendChild(
    row(
      "Interface size",
      "Scales the entire window content.",
      select(
        [["0.9", "Compact"], ["1", "Normal"], ["1.1", "Large"], ["1.25", "Extra large"]],
        String(store.ui.fontSize),
        (v) => {
          store.ui.fontSize = Number(v);
          applyUiPrefs();
          saveUiPrefs();
        }
      )
    )
  );
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
      "Sound theme",
      "The character of the interface sounds.",
      select(
        [["soft", "Soft"], ["crisp", "Crisp"], ["chime", "Chime"], ["retro", "Retro"]],
        store.ui.soundTheme,
        (v) => {
          store.ui.soundTheme = v;
          saveUiPrefs();
          sfx.select();
        }
      )
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
        applyUiPrefs();
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
  {
    const tour = document.createElement("button");
    tour.className = "btn";
    tour.textContent = "Replay welcome tour";
    tour.addEventListener("click", () => {
      sfx.click();
      showOnboarding(true);
    });
    appear.appendChild(row("Welcome tour", "The feature walkthrough from first launch.", tour));
  }
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
    <div class="hint">v${st?.version ?? "?"} · ${store.effects.length} effects · HID ${st?.hid_connected ? "connected" : "disconnected"} ·
    uptime ${Math.floor((st?.uptime_sec ?? 0) / 60)} min · ROG Strix SCAR 16 (G634JZ)</div></div>`;
  perf.appendChild(about);
  grid.appendChild(perf);

  view.appendChild(grid);
  root.appendChild(view);

  // ---------------- search behavior
  search.addEventListener("input", () => {
    const q = search.value.trim().toLowerCase();
    grid.querySelectorAll<HTMLElement>(".panel").forEach((panel) => {
      const titleHit = panel.querySelector("h3")?.textContent?.toLowerCase().includes(q) ?? false;
      let any = false;
      panel.querySelectorAll<HTMLElement>(".row").forEach((r) => {
        const hit = !q || titleHit || r.textContent!.toLowerCase().includes(q);
        r.style.display = hit ? "" : "none";
        if (hit) any = true;
      });
      panel.style.display = !q || any || titleHit ? "" : "none";
    });
  });

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
