import { core } from "../ipc";
import { buildControls, paletteCss } from "../params";
import { categories, effectById, paramsFor, refreshStatus, sendParams, store } from "../state";
import { sfx } from "../sound";

let currentCat = "All";

export function renderEffects(root: HTMLElement): (() => void) | void {
  root.innerHTML = "";
  const view = document.createElement("div");
  view.className = "view";

  // category chips
  const chips = document.createElement("div");
  chips.className = "chips";
  for (const c of ["All", ...categories()]) {
    const chip = document.createElement("button");
    chip.className = "chip" + (c === currentCat ? " active" : "");
    chip.textContent = c;
    chip.addEventListener("click", () => {
      currentCat = c;
      sfx.click();
      renderEffects(root);
    });
    chips.appendChild(chip);
  }
  view.appendChild(chips);

  const layout = document.createElement("div");
  layout.className = "effects-layout";

  // cards
  const grid = document.createElement("div");
  grid.className = "cards";
  const activeId = store.status?.effect as string | undefined;
  const list = store.effects.filter((e) => currentCat === "All" || e.category === currentCat);
  for (const e of list) {
    const card = document.createElement("div");
    card.className = "card" + (e.id === activeId ? " active" : "");
    const sw = document.createElement("div");
    sw.className = "swatch";
    const palId = (paramsFor(e.id).palette as string) || e.default_palette;
    const pal = store.palettes.find((p) => p.id === palId);
    sw.style.background = pal ? paletteCss(pal.stops) : "#223";
    const h = document.createElement("h4");
    h.textContent = e.name;
    if (e.needs_input) {
      const b = document.createElement("span");
      b.className = "badge-input";
      b.textContent = "typing";
      h.appendChild(b);
    }
    const p = document.createElement("p");
    p.textContent = e.blurb;
    card.append(sw, h, p);
    card.addEventListener("mouseenter", () => sfx.hover());
    card.addEventListener("click", async () => {
      if (store.status) store.status.effect = e.id;
      sfx.select();
      await core.req("set_effect", { id: e.id });
      setTimeout(() => void refreshStatus(), 250);
      renderEffects(root);
    });
    grid.appendChild(card);
  }
  layout.appendChild(grid);

  // param panel for the active effect
  const panel = document.createElement("div");
  panel.className = "panel sticky";
  const info = activeId ? effectById(activeId) : undefined;
  if (info) {
    const h3 = document.createElement("h3");
    h3.textContent = info.name;
    const sub = document.createElement("div");
    sub.className = "sub";
    sub.textContent = info.blurb;
    panel.append(h3, sub);
    const live = paramsFor(info.id);
    panel.appendChild(
      buildControls(info.specs, live, (key, value) => {
        live[key] = value;
        sendParams(info.id, live);
      })
    );
  } else {
    panel.innerHTML = `<h3>No effect selected</h3><div class="sub">Pick an effect from the gallery.</div>`;
  }
  layout.appendChild(panel);

  view.appendChild(layout);
  root.appendChild(view);
}
