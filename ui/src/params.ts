// Auto-generated parameter editors from the daemon's ParamSpec schema —
// no per-effect UI code anywhere.

import { Spec, store } from "./state";
import { sfx } from "./sound";

export function buildControls(
  specs: Spec[],
  values: Record<string, any>,
  onChange: (key: string, value: any) => void
): HTMLElement {
  const root = document.createElement("div");
  // palette control renders biggest — push to the end, mask just before it
  const order = [...specs].sort((a, b) => weight(a) - weight(b));
  for (const s of order) {
    root.appendChild(control(s, values[s.key] ?? s.default, onChange));
  }
  return root;
}

function weight(s: Spec): number {
  if (s.kind === "palette") return 3;
  if (s.kind === "mask") return 2;
  if (s.kind === "select") return 1;
  return 0;
}

function control(s: Spec, value: any, onChange: (k: string, v: any) => void): HTMLElement {
  const wrap = document.createElement("div");
  wrap.className = "ctl";

  if (s.kind === "slider") {
    const row = document.createElement("div");
    row.className = "ctl-row";
    const lbl = document.createElement("label");
    lbl.textContent = s.label;
    const val = document.createElement("span");
    val.className = "val";
    row.append(lbl, val);
    const input = document.createElement("input");
    input.type = "range";
    input.min = String(s.min ?? 0);
    input.max = String(s.max ?? 1);
    input.step = String(s.step ?? 0.01);
    input.value = String(typeof value === "number" ? value : Number(s.default));
    const paint = () => {
      const lo = Number(input.min);
      const hi = Number(input.max);
      const f = ((Number(input.value) - lo) / (hi - lo)) * 100;
      input.style.setProperty("--fill", `${f}%`);
      val.textContent = fmt(Number(input.value), Number(input.step));
    };
    paint();
    input.addEventListener("input", () => {
      paint();
      onChange(s.key, Number(input.value));
    });
    wrap.append(row, input);
    return wrap;
  }

  if (s.kind === "toggle") {
    const row = document.createElement("div");
    row.className = "ctl-row";
    const lbl = document.createElement("label");
    lbl.textContent = s.label;
    row.append(lbl, toggle(Boolean(value), (v) => onChange(s.key, v)));
    wrap.append(row);
    return wrap;
  }

  if (s.kind === "select" || s.kind === "mask") {
    const row = document.createElement("div");
    row.className = "ctl-row";
    const lbl = document.createElement("label");
    lbl.textContent = s.label;
    row.append(lbl);
    const sel = document.createElement("select");
    for (const opt of s.options ?? []) {
      const o = document.createElement("option");
      o.value = opt;
      o.textContent = opt[0].toUpperCase() + opt.slice(1);
      if (opt === value) o.selected = true;
      sel.appendChild(o);
    }
    sel.addEventListener("change", () => {
      sfx.click();
      onChange(s.key, sel.value);
    });
    wrap.append(row, sel);
    return wrap;
  }

  if (s.kind === "palette") {
    const row = document.createElement("div");
    row.className = "ctl-row";
    const lbl = document.createElement("label");
    lbl.textContent = s.label;
    row.append(lbl);
    wrap.append(row);
    const grid = document.createElement("div");
    grid.className = "pal-grid";
    for (const p of store.palettes) {
      const el = document.createElement("div");
      el.className = "pal" + (p.id === value ? " active" : "");
      const strip = document.createElement("div");
      strip.className = "strip";
      strip.style.background = paletteCss(p.stops);
      const nm = document.createElement("div");
      nm.className = "nm";
      nm.textContent = p.name;
      el.append(strip, nm);
      el.addEventListener("click", () => {
        grid.querySelectorAll(".pal").forEach((n) => n.classList.remove("active"));
        el.classList.add("active");
        sfx.click();
        onChange(s.key, p.id);
      });
      grid.appendChild(el);
    }
    wrap.append(grid);
    return wrap;
  }

  return wrap;
}

export function toggle(checked: boolean, onChange: (v: boolean) => void): HTMLElement {
  const label = document.createElement("label");
  label.className = "toggle";
  const input = document.createElement("input");
  input.type = "checkbox";
  input.checked = checked;
  const knob = document.createElement("span");
  knob.className = "knob";
  input.addEventListener("change", () => {
    input.checked ? sfx.toggleOn() : sfx.toggleOff();
    onChange(input.checked);
  });
  label.append(input, knob);
  return label;
}

export function paletteCss(stops: { t: number; c: string }[]): string {
  const parts = stops.map((s) => `${s.c} ${Math.round(s.t * 100)}%`);
  return `linear-gradient(90deg, ${parts.join(", ")})`;
}

function fmt(v: number, step: number): string {
  return step >= 1 ? String(Math.round(v)) : v.toFixed(2);
}
