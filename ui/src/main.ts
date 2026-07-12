import { core } from "./ipc";
import { KeyboardView } from "./keyboard";
import { showOnboarding } from "./onboarding";
import { applyUiPrefs, bootstrap, refreshStatus, store } from "./state";
import { sfx } from "./sound";
import { renderAudio } from "./views/audio";
import { renderCustom } from "./views/custom";
import { renderEffects } from "./views/effects";
import { renderGuide } from "./views/guide";
import { renderPlaylist } from "./views/playlist";
import { renderSettings } from "./views/settings";

type ViewFn = (root: HTMLElement) => (() => void) | void;

const views: Record<string, ViewFn> = {
  effects: renderEffects,
  playlist: renderPlaylist,
  custom: renderCustom,
  audio: renderAudio,
  settings: renderSettings,
  guide: renderGuide,
};

const viewRoot = document.getElementById("view-root")!;
const pill = document.getElementById("core-pill")!;
const pillText = pill.querySelector(".pill-text")!;
let currentView = "effects";
let cleanup: (() => void) | null = null;
let lastEffect = "";

function show(view: string, silent = false) {
  currentView = view;
  cleanup?.();
  cleanup = null;
  document.querySelectorAll<HTMLElement>(".nav-btn").forEach((b) => {
    b.classList.toggle("active", b.dataset.view === view);
  });
  if (!silent) sfx.whoosh();
  if (!core.online) {
    renderOffline();
    return;
  }
  cleanup = views[view]?.(viewRoot) ?? null;
  // each section starts at its own top, not the previous section's scroll
  viewRoot.scrollTop = 0;
}

function renderOffline() {
  viewRoot.innerHTML = `
    <div class="view"><div class="panel" style="max-width:520px;margin:40px auto;text-align:center">
      <h3>Lighting core is offline</h3>
      <div class="sub" style="margin-top:6px">The Keyscape core runs as a tiny background process and
      keeps your lighting alive even when this window is closed. Waiting for it on
      <b>ws://127.0.0.1:53971</b>…</div>
      <button class="btn primary" id="start-core">Start core</button>
    </div></div>`;
  document.getElementById("start-core")?.addEventListener("click", () => {
    sfx.click();
    const tauri = (window as any).__TAURI__;
    if (tauri?.core?.invoke) void tauri.core.invoke("start_core");
  });
}

function updateMeta() {
  const name = document.getElementById("active-effect-name")!;
  const cat = document.getElementById("active-effect-cat")!;
  name.textContent = store.status?.effect_name || "—";
  cat.textContent = store.status?.category || "";
  if (store.status?.settings?.playlist?.enabled) cat.textContent += " · playlist";
  if (store.status?.audio?.active) cat.textContent += " · ♫";
}

// ---------- boot ----------
const kb = new KeyboardView(document.getElementById("kb-canvas") as HTMLCanvasElement);
core.onFrame((bytes) => kb.onFrame(bytes));

core.onState(async (online) => {
  pill.classList.toggle("online", online);
  pill.classList.toggle("offline", !online);
  pillText.textContent = online ? "core connected" : "core offline";
  if (online) {
    await bootstrap();
    if (store.layout) kb.setLayout(store.layout);
    applyUiPrefs();
    show(currentView, true);
    showOnboarding();
  } else {
    renderOffline();
  }
});

store.sub(() => {
  updateMeta();
  const eff = store.status?.effect ?? "";
  if (eff !== lastEffect) {
    // playlist advanced (or another client changed it): refresh the gallery highlight
    if (lastEffect && currentView === "effects" && core.online) show("effects", true);
    lastEffect = eff;
  }
});

document.querySelectorAll<HTMLElement>(".nav-btn").forEach((btn) => {
  btn.addEventListener("click", () => show(btn.dataset.view!));
});

setInterval(() => {
  if (core.online && !document.hidden) void refreshStatus();
}, 2500);

// Background mesh drift at ~7 fps from JS — a CSS animation on this layer
// keeps the compositor running full-tilt every vsync (see styles.css).
const mesh = document.getElementById("bg-mesh");
let driftT = Math.random() * 1000;
setInterval(() => {
  if (!mesh || document.hidden || !store.ui.motion) return;
  driftT += 0.15;
  const x = Math.sin(driftT * 0.021) * 2.2;
  const y = Math.cos(driftT * 0.017) * 1.8;
  mesh.style.transform = `translate3d(${x}%, ${y}%, 0)`;
}, 150);

renderOffline();
core.connect();
