import { core } from "../ipc";
import { toggle } from "../params";
import { patchSettings, refreshStatus, store } from "../state";

export function renderAudio(root: HTMLElement): (() => void) | void {
  root.innerHTML = "";
  const view = document.createElement("div");
  view.className = "view";

  const audio = store.status?.settings?.audio ?? {
    enabled: false,
    gain: 1.0,
    mod_brightness: true,
    mod_speed: true,
    mod_palette: false,
    amount: 0.7,
  };

  if (!audio.enabled) {
    const banner = document.createElement("div");
    banner.className = "banner";
    banner.innerHTML = `<div>Music mode is <b>off</b>. When enabled, Keyscape listens to what your
      system is <i>playing</i> (WASAPI loopback — never the microphone) and lets the beat drive the
      active effect. Nothing is captured until you switch it on.</div>`;
    view.appendChild(banner);
  }

  const panel = document.createElement("div");
  panel.className = "panel";
  panel.innerHTML = `<h3>Music-reactive lighting</h3>
    <div class="sub">A styling layer over the current effect — audio modulates its speed, brightness
    and palette instead of replacing it with a bespoke visualizer.</div>`;

  const rowEnable = document.createElement("div");
  rowEnable.className = "row";
  rowEnable.innerHTML = `<div><div class="lbl">Enable music mode</div>
    <div class="hint">Starts the loopback analysis thread. Off by default; costs ~0 when off.</div></div>`;
  rowEnable.appendChild(
    toggle(Boolean(audio.enabled), async (v) => {
      await core.req("patch_settings", { patch: { audio: { enabled: v } } });
      await refreshStatus();
      renderAudio(root);
    })
  );
  panel.appendChild(rowEnable);

  const slider = (
    lbl: string,
    hint: string,
    min: number,
    max: number,
    step: number,
    value: number,
    key: string
  ) => {
    const row = document.createElement("div");
    row.className = "row";
    row.innerHTML = `<div><div class="lbl">${lbl}</div><div class="hint">${hint}</div></div>`;
    const wrap = document.createElement("div");
    wrap.style.cssText = "display:flex;align-items:center;gap:10px";
    const val = document.createElement("span");
    val.className = "val";
    val.style.minWidth = "36px";
    const r = document.createElement("input");
    r.type = "range";
    r.min = String(min);
    r.max = String(max);
    r.step = String(step);
    r.value = String(value);
    r.style.width = "180px";
    const paint = () => {
      val.textContent = Number(r.value).toFixed(step >= 1 ? 0 : 2);
      r.style.setProperty("--fill", `${((Number(r.value) - min) / (max - min)) * 100}%`);
    };
    paint();
    r.addEventListener("input", () => {
      paint();
      patchSettings("audio." + key, { audio: { [key]: Number(r.value) } });
    });
    wrap.append(r, val);
    row.appendChild(wrap);
    return row;
  };

  panel.appendChild(
    slider("Sensitivity", "Input gain applied to the analysis.", 0.2, 3, 0.05, audio.gain ?? 1, "gain")
  );
  panel.appendChild(
    slider("Amount", "How strongly music bends the effect.", 0, 1, 0.05, audio.amount ?? 0.7, "amount")
  );

  const mods: [string, string, string, boolean][] = [
    ["Brightness", "Pulses the board with level and beats.", "mod_brightness", audio.mod_brightness],
    ["Speed", "Bass and beats accelerate the effect's time.", "mod_speed", audio.mod_speed],
    ["Palette drift", "Timbre brightness slides the palette phase.", "mod_palette", audio.mod_palette],
  ];
  for (const [lbl, hint, key, val] of mods) {
    const row = document.createElement("div");
    row.className = "row";
    row.innerHTML = `<div><div class="lbl">${lbl}</div><div class="hint">${hint}</div></div>`;
    row.appendChild(
      toggle(Boolean(val), (v) => patchSettings("audio." + key, { audio: { [key]: v } }))
    );
    panel.appendChild(row);
  }
  view.appendChild(panel);

  // live meters
  const meterPanel = document.createElement("div");
  meterPanel.className = "panel";
  meterPanel.style.marginTop = "14px";
  meterPanel.innerHTML = `<h3>Live analysis</h3><div class="sub">${
    audio.enabled ? "What the engine hears right now." : "Enable music mode to see live levels."
  }</div>`;
  const bars: Record<string, HTMLElement> = {};
  for (const name of ["Level", "Bass", "Mid", "Treble", "Beat"]) {
    const row = document.createElement("div");
    row.style.cssText = "display:flex;align-items:center;gap:12px;margin:9px 0";
    const lbl = document.createElement("span");
    lbl.style.cssText = "width:56px;font-size:11.5px;color:var(--text-dim)";
    lbl.textContent = name;
    const meter = document.createElement("div");
    meter.className = "meter";
    meter.style.flex = "1";
    const fill = document.createElement("div");
    meter.appendChild(fill);
    bars[name.toLowerCase()] = fill;
    row.append(lbl, meter);
    meterPanel.appendChild(row);
  }
  view.appendChild(meterPanel);
  root.appendChild(view);

  let timer = 0;
  if (audio.enabled) {
    timer = window.setInterval(async () => {
      const r = await core.req("status");
      if (r.ok) {
        const a = r.status?.audio ?? {};
        for (const k of ["level", "bass", "mid", "treble", "beat"]) {
          if (bars[k]) bars[k].style.width = `${Math.min(100, (a[k] ?? 0) * 100)}%`;
        }
      }
    }, 300);
  }
  return () => clearInterval(timer);
}
