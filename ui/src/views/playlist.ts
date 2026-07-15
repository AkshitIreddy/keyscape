import { core } from "../ipc";
import { toggle } from "../params";
import { patchSettings, refreshStatus, store } from "../state";
import { sfx } from "../sound";

// Curated "moods" — one-click presets that fill the rotation with effects that
// share a feel. Typing-reactive effects are deliberately left out: they need
// you to be at the keyboard, so they'd sit dark in an unattended playlist.
// Ids are filtered against the live registry, so a stale id just drops out.
const MOODS: { key: string; label: string; ids: string[] }[] = [
  {
    key: "calm",
    label: "Calm",
    ids: ["nebula_drift", "deep_field", "zen_garden", "moon_phases", "ink_water",
      "bioluminescence", "aurora_veil", "coral_reef", "pollen_drift", "lava_lamp",
      "candlelight", "solar_sync"],
  },
  {
    key: "energetic",
    label: "Energetic",
    ids: ["meteor_storm", "comet_billiards", "swarm", "gravity_wells", "chaos_pendulum",
      "glitch_cascade", "packet_flow", "firewall", "snake_trio", "thunderstorm",
      "radar_sweep", "spiral_bloom"],
  },
  {
    key: "cosmic",
    label: "Cosmic",
    ids: ["nebula_drift", "deep_field", "meteor_storm", "pulsar", "supernova_cycle",
      "constellation", "black_hole", "solar_wind", "orrery", "moon_phases"],
  },
  {
    key: "nature",
    label: "Nature",
    ids: ["ocean_tide", "ivy_growth", "coral_reef", "pollen_drift", "firefly_meadow",
      "bioluminescence", "aurora_veil", "thunderstorm", "sandfall", "zen_garden"],
  },
  {
    key: "retro",
    label: "Retro",
    ids: ["glitch_cascade", "bad_signal", "packet_flow", "game_of_life", "rule_cascade",
      "bitcrush", "firewall"],
  },
];

const sameSet = (a: Set<string>, b: Set<string>) =>
  a.size === b.size && [...a].every((x) => b.has(x));

export function renderPlaylist(root: HTMLElement): (() => void) | void {
  root.innerHTML = "";
  const view = document.createElement("div");
  view.className = "view";

  const pl = store.status?.settings?.playlist ?? {
    enabled: false,
    shuffle: true,
    interval_sec: 120,
    effects: [] as string[],
    shuffle_palettes: false,
  };
  const chosen = new Set<string>(pl.effects ?? []);
  const known = new Set(store.effects.map((e) => e.id));
  const moodIds = (m: { ids: string[] }) => m.ids.filter((id) => known.has(id));

  const send = () => {
    patchSettings("playlist", {
      playlist: {
        enabled: pl.enabled,
        shuffle: pl.shuffle,
        interval_sec: pl.interval_sec,
        effects: [...chosen],
        shuffle_palettes: pl.shuffle_palettes,
      },
    });
  };

  const panel = document.createElement("div");
  panel.className = "panel";
  panel.innerHTML = `<h3>Playlist</h3>
    <div class="sub">Rotate through effects automatically. Leave every effect unchecked to rotate through the whole library.</div>`;

  const rowEnable = document.createElement("div");
  rowEnable.className = "row";
  rowEnable.innerHTML = `<div><div class="lbl">Enabled</div><div class="hint">Cycle to a new effect on the interval below.</div></div>`;
  rowEnable.appendChild(
    toggle(Boolean(pl.enabled), (v) => {
      pl.enabled = v;
      send();
    })
  );

  const rowMode = document.createElement("div");
  rowMode.className = "row";
  rowMode.innerHTML = `<div class="lbl">Order</div>`;
  const sel = document.createElement("select");
  sel.style.width = "160px";
  sel.innerHTML = `<option value="shuffle">Shuffle</option><option value="sequence">In order</option>`;
  sel.value = pl.shuffle ? "shuffle" : "sequence";
  sel.addEventListener("change", () => {
    sfx.click();
    pl.shuffle = sel.value === "shuffle";
    send();
  });
  rowMode.appendChild(sel);

  const rowPal = document.createElement("div");
  rowPal.className = "row";
  rowPal.innerHTML = `<div><div class="lbl">Shuffle palettes</div><div class="hint">Give each effect a random color palette each time it comes up. The effect's own saved palette isn't changed.</div></div>`;
  rowPal.appendChild(
    toggle(Boolean(pl.shuffle_palettes), (v) => {
      pl.shuffle_palettes = v;
      send();
    })
  );

  const rowInt = document.createElement("div");
  rowInt.className = "row";
  const intLbl = document.createElement("div");
  intLbl.innerHTML = `<div class="lbl">Switch every</div><div class="hint" id="int-warn"></div>`;
  const intWarn = intLbl.querySelector("#int-warn") as HTMLElement;
  const intVal = document.createElement("div");
  intVal.className = "val";
  intVal.style.minWidth = "52px";
  const range = document.createElement("input");
  range.type = "range";
  range.min = "30"; // hard floor: below this effects barely settle before switching
  range.max = "1800";
  range.step = "30";
  range.style.width = "220px";
  range.value = String(pl.interval_sec ?? 120);
  const paint = () => {
    const s = Number(range.value);
    intVal.textContent = s >= 60 ? `${Math.round(s / 60)} min` : `${s} s`;
    range.style.setProperty("--fill", `${((s - 30) / 1770) * 100}%`);
    // Effects are continuous — none "ends" — but many (Reaction Diffusion,
    // Ivy Growth, Game of Life…) take ~10-30 s to develop their character.
    if (s < 60) {
      intWarn.textContent = "Very short — slow-developing effects barely appear before the next switch.";
      intWarn.style.color = "var(--danger)";
    } else {
      intWarn.textContent = "";
    }
  };
  paint();
  range.addEventListener("input", () => {
    paint();
    pl.interval_sec = Number(range.value);
    send();
  });
  const intWrap = document.createElement("div");
  intWrap.style.display = "flex";
  intWrap.style.alignItems = "center";
  intWrap.style.gap = "12px";
  intWrap.append(range, intVal);
  rowInt.append(intLbl, intWrap);

  const rowNext = document.createElement("div");
  rowNext.className = "row";
  rowNext.innerHTML = `<div><div class="lbl">Skip ahead</div><div class="hint">Jump to the next effect right now.</div></div>`;
  const nextBtn = document.createElement("button");
  nextBtn.className = "btn primary";
  nextBtn.textContent = "Next effect ›";
  nextBtn.addEventListener("click", async () => {
    sfx.select();
    await core.req("next");
    setTimeout(() => void refreshStatus(), 250);
  });
  rowNext.appendChild(nextBtn);

  panel.append(rowEnable, rowMode, rowPal, rowInt, rowNext);
  view.appendChild(panel);

  // effect checklist + mood presets
  const listPanel = document.createElement("div");
  listPanel.className = "panel";
  listPanel.style.marginTop = "14px";
  listPanel.innerHTML = `<h3>Effects in rotation</h3>`;
  const sub = document.createElement("div");
  sub.className = "sub";
  listPanel.appendChild(sub);

  const boxes = new Map<string, HTMLInputElement>();
  const setSub = () => {
    sub.textContent = chosen.size === 0 ? "All effects (nothing checked)" : `${chosen.size} selected`;
  };

  // mood chip row
  const moodRow = document.createElement("div");
  moodRow.className = "mood-row";
  const chips: { el: HTMLElement; match: () => boolean }[] = [];
  const paintChips = () => chips.forEach((c) => c.el.classList.toggle("on", c.match()));

  const addChip = (label: string, match: () => boolean, apply: () => void) => {
    const chip = document.createElement("button");
    chip.className = "mood-chip";
    chip.textContent = label;
    chip.addEventListener("click", () => {
      sfx.click();
      apply();
      setSub();
      for (const [id, cb] of boxes) cb.checked = chosen.has(id);
      paintChips();
      send();
    });
    moodRow.appendChild(chip);
    chips.push({ el: chip, match });
  };

  addChip(
    "All effects",
    () => chosen.size === 0,
    () => chosen.clear()
  );
  for (const m of MOODS) {
    const ids = moodIds(m);
    if (ids.length === 0) continue;
    addChip(
      m.label,
      () => sameSet(chosen, new Set(ids)),
      () => {
        chosen.clear();
        for (const id of ids) chosen.add(id);
      }
    );
  }
  listPanel.appendChild(moodRow);

  for (const e of store.effects) {
    const rowEl = document.createElement("label");
    rowEl.className = "fx-check";
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = chosen.has(e.id);
    cb.addEventListener("change", () => {
      cb.checked ? chosen.add(e.id) : chosen.delete(e.id);
      setSub();
      paintChips();
      sfx.click();
      send();
    });
    boxes.set(e.id, cb);
    const nm = document.createElement("span");
    nm.textContent = e.name;
    const cat = document.createElement("span");
    cat.className = "cat";
    cat.textContent = e.category;
    rowEl.append(cb, nm, cat);
    listPanel.appendChild(rowEl);
  }
  setSub();
  paintChips();
  view.appendChild(listPanel);

  root.appendChild(view);
}
