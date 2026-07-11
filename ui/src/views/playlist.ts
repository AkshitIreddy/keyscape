import { core } from "../ipc";
import { toggle } from "../params";
import { patchSettings, refreshStatus, store } from "../state";
import { sfx } from "../sound";

export function renderPlaylist(root: HTMLElement): (() => void) | void {
  root.innerHTML = "";
  const view = document.createElement("div");
  view.className = "view";

  const pl = store.status?.settings?.playlist ?? {
    enabled: false,
    shuffle: true,
    interval_sec: 300,
    effects: [] as string[],
  };
  const chosen = new Set<string>(pl.effects ?? []);

  const send = () => {
    patchSettings("playlist", {
      playlist: {
        enabled: pl.enabled,
        shuffle: pl.shuffle,
        interval_sec: pl.interval_sec,
        effects: [...chosen],
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

  const rowInt = document.createElement("div");
  rowInt.className = "row";
  const intLbl = document.createElement("div");
  intLbl.innerHTML = `<div class="lbl">Switch every</div>`;
  const intVal = document.createElement("div");
  intVal.className = "val";
  intVal.style.minWidth = "52px";
  const range = document.createElement("input");
  range.type = "range";
  range.min = "30";
  range.max = "1800";
  range.step = "30";
  range.style.width = "220px";
  range.value = String(pl.interval_sec ?? 300);
  const paint = () => {
    const s = Number(range.value);
    intVal.textContent = s >= 60 ? `${Math.round(s / 60)} min` : `${s} s`;
    range.style.setProperty("--fill", `${((s - 30) / 1770) * 100}%`);
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

  panel.append(rowEnable, rowMode, rowInt, rowNext);
  view.appendChild(panel);

  // effect checklist
  const listPanel = document.createElement("div");
  listPanel.className = "panel";
  listPanel.style.marginTop = "14px";
  listPanel.innerHTML = `<h3>Effects in rotation</h3><div class="sub">${
    chosen.size === 0 ? "All effects (nothing checked)" : `${chosen.size} selected`
  }</div>`;
  const sub = listPanel.querySelector(".sub")!;
  for (const e of store.effects) {
    const row = document.createElement("label");
    row.className = "fx-check";
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = chosen.has(e.id);
    cb.addEventListener("change", () => {
      cb.checked ? chosen.add(e.id) : chosen.delete(e.id);
      sub.textContent = chosen.size === 0 ? "All effects (nothing checked)" : `${chosen.size} selected`;
      sfx.click();
      send();
    });
    const nm = document.createElement("span");
    nm.textContent = e.name;
    const cat = document.createElement("span");
    cat.className = "cat";
    cat.textContent = e.category;
    row.append(cb, nm, cat);
    listPanel.appendChild(row);
  }
  view.appendChild(listPanel);

  root.appendChild(view);
}
