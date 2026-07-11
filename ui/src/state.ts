import { core, Json } from "./ipc";

export interface Spec {
  key: string;
  label: string;
  kind: "slider" | "toggle" | "select" | "palette" | "mask" | "color";
  min?: number;
  max?: number;
  step?: number;
  default: any;
  options?: string[];
}

export interface EffectInfo {
  id: string;
  name: string;
  category: string;
  blurb: string;
  needs_input: boolean;
  default_palette: string;
  specs: Spec[];
}

export interface PaletteInfo {
  id: string;
  name: string;
  stops: { t: number; c: string }[];
}

export interface KeyInfo {
  led: number;
  name: string;
  row: number;
  col: number;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface LayoutInfo {
  aspect: number;
  keys: KeyInfo[];
  aux: { led: number; name: string }[];
}

export interface UiPrefs {
  sounds: boolean;
  volume: number;
  motion: boolean;
  glow: boolean;
}

type Listener = () => void;

export const store = {
  effects: [] as EffectInfo[],
  palettes: [] as PaletteInfo[],
  layout: null as LayoutInfo | null,
  status: null as Json | null,
  guardRunning: false,
  ui: { sounds: true, volume: 0.4, motion: true, glow: true } as UiPrefs,
  booted: false,

  listeners: new Set<Listener>(),
  sub(fn: Listener) {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  },
  emit() {
    this.listeners.forEach((fn) => fn());
  },
};

export function categories(): string[] {
  const seen: string[] = [];
  for (const e of store.effects) if (!seen.includes(e.category)) seen.push(e.category);
  return seen;
}

export function effectById(id: string): EffectInfo | undefined {
  return store.effects.find((e) => e.id === id);
}

/// Current param values for an effect: stored settings over spec defaults.
export function paramsFor(id: string): Json {
  const info = effectById(id);
  const out: Json = {};
  if (!info) return out;
  for (const s of info.specs) out[s.key] = s.default;
  const stored = store.status?.settings?.effect_params?.[id];
  if (stored) Object.assign(out, stored);
  // live params for the active effect are authoritative
  if (store.status?.effect === id && store.status?.params) Object.assign(out, store.status.params);
  return out;
}

export async function refreshStatus() {
  const r = await core.req("status");
  if (r.ok) {
    store.status = r.status;
    const ui = r.status?.settings?.ui;
    if (ui && typeof ui === "object") Object.assign(store.ui, ui);
    store.emit();
  }
}

export async function bootstrap() {
  const [fx, pal, lay, guard] = await Promise.all([
    core.req("effects"),
    core.req("palettes"),
    core.req("layout"),
    core.req("guard_running"),
  ]);
  if (fx.ok) store.effects = fx.effects;
  if (pal.ok) store.palettes = pal.palettes;
  if (lay.ok) store.layout = { aspect: lay.aspect, keys: lay.keys, aux: lay.aux };
  if (guard.ok) store.guardRunning = guard.running;
  await refreshStatus();
  store.booted = true;
  core.subscribePreview();
  store.emit();
}

export function saveUiPrefs() {
  void core.req("patch_settings", { patch: { ui: store.ui } });
}

let patchTimers = new Map<string, number>();

/// Debounced settings patch, keyed so rapid slider moves collapse.
export function patchSettings(key: string, patch: Json, delay = 160) {
  clearTimeout(patchTimers.get(key));
  patchTimers.set(
    key,
    window.setTimeout(() => {
      void core.req("patch_settings", { patch }).then(() => refreshStatus());
    }, delay)
  );
}

let paramTimers = new Map<string, number>();

export function sendParams(id: string, params: Json, delay = 140) {
  clearTimeout(paramTimers.get(id));
  paramTimers.set(
    id,
    window.setTimeout(() => {
      void core.req("set_params", { id, params });
    }, delay)
  );
}
