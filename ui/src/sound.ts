// Synthesized interaction sounds — no audio assets, just tiny WebAudio
// envelopes. Everything routes through one master gain so the Appearance
// settings can mute or attenuate globally.
//
// Robustness notes (this is why the class is more than a few oscillators):
// a single long-lived AudioContext goes bad in WebView2/Chromium when the
// default output device changes or the machine sleeps — it keeps "running"
// but renders garbage, so the cues turn into static. We therefore let the
// context go idle (suspended) when unused, rebuild it when the device list
// changes, and watch for a wedged context whose clock has stopped advancing.

import { store } from "./state";

/** Never let more than this many cues sound at once. */
const MAX_VOICES = 10;
/** Hover fires per card; a fast mouse sweep would otherwise machine-gun it. */
const HOVER_MIN_GAP_MS = 45;
/** Release the audio device after this long with no cues. */
const IDLE_SUSPEND_MS = 20_000;

class Sfx {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private noise: AudioBuffer | null = null;
  /** Output device changed under us — rebuild before the next cue. */
  private stale = false;
  private deviceWatch = false;
  private voices = 0;
  private lastHover = 0;
  private idleTimer = 0;
  // wedge detection: a "running" context whose clock has frozen
  private lastCtxTime = -1;
  private lastWall = 0;

  private teardown() {
    const dying = this.ctx;
    this.ctx = null;
    this.master = null;
    this.noise = null;
    this.voices = 0;
    this.lastCtxTime = -1;
    this.lastWall = 0;
    this.stale = false;
    if (dying && dying.state !== "closed") void dying.close().catch(() => {});
  }

  private ensure(): boolean {
    if (!store.ui.sounds) return false;

    // A stale/closed/wedged context renders noise instead of tones — drop it.
    if (this.ctx && (this.stale || this.ctx.state === "closed")) this.teardown();
    if (this.ctx && this.ctx.state === "running") {
      const wall = performance.now();
      const frozen = this.lastWall > 0 && wall - this.lastWall > 400 && this.ctx.currentTime === this.lastCtxTime;
      if (frozen) this.teardown();
    }

    if (!this.ctx) {
      try {
        this.ctx = new AudioContext();
        this.master = this.ctx.createGain();
        // No limiter here on purpose: the cues peak around 0.03, so nothing
        // can clip, and Chromium's DynamicsCompressor is not transparent at
        // these levels (it measurably quietens a single cue).
        this.master.connect(this.ctx.destination);
      } catch {
        this.ctx = null;
        return false;
      }
      if (!this.deviceWatch) {
        // Plugging headphones / switching to another output invalidates the
        // context's device binding; rebuild rather than render into the void.
        this.deviceWatch = true;
        navigator.mediaDevices?.addEventListener?.("devicechange", () => {
          this.stale = true;
        });
      }
    }

    const ctx = this.ctx!;
    if (ctx.state === "suspended") {
      void ctx.resume().catch(() => {
        this.stale = true;
      });
    }
    this.lastCtxTime = ctx.currentTime;
    this.lastWall = performance.now();

    // Smooth the master gain: writing .value mid-cue is a step discontinuity
    // (an audible tick), and a non-finite value makes WebAudio emit garbage.
    const vol = Number(store.ui.volume);
    const target = (Number.isFinite(vol) ? Math.min(Math.max(vol, 0), 1) : 0.4) * 0.5;
    const g = this.master!.gain;
    if (Math.abs(g.value - target) > 1e-4) g.setTargetAtTime(target, ctx.currentTime, 0.01);

    this.scheduleIdleSuspend();
    return true;
  }

  /** Let the context release the audio device when nothing is playing. */
  private scheduleIdleSuspend() {
    clearTimeout(this.idleTimer);
    this.idleTimer = window.setTimeout(() => {
      if (this.ctx && this.ctx.state === "running" && this.voices === 0) {
        void this.ctx.suspend().catch(() => {});
      }
    }, IDLE_SUSPEND_MS);
  }

  /** sound theme reshapes every cue: waveform, pitch, gain, length */
  private theme(): { type: OscillatorType; f: number; g: number; d: number } {
    const themes: Record<string, { type: OscillatorType; f: number; g: number; d: number }> = {
      soft: { type: "sine", f: 1.0, g: 1.0, d: 1.0 },
      crisp: { type: "triangle", f: 1.4, g: 1.15, d: 0.6 },
      chime: { type: "sine", f: 1.8, g: 0.8, d: 1.6 },
      retro: { type: "square", f: 0.8, g: 0.5, d: 0.7 },
    };
    return themes[store.ui.soundTheme] ?? themes.soft;
  }

  private tone(freq: number, dur: number, opts: { type?: OscillatorType; gain?: number; sweep?: number; delay?: number } = {}) {
    if (!this.ensure()) return;
    if (this.voices >= MAX_VOICES) return; // pile-up guard
    const th = this.theme();
    freq *= th.f;
    dur *= th.d;
    const ctx = this.ctx!;
    const t0 = ctx.currentTime + (opts.delay ?? 0);
    const osc = ctx.createOscillator();
    const g = ctx.createGain();
    osc.type = opts.type ?? th.type;
    osc.frequency.setValueAtTime(freq, t0);
    if (opts.sweep) osc.frequency.exponentialRampToValueAtTime(Math.max(40, freq * opts.sweep), t0 + dur);
    const peak = (opts.gain ?? 0.08) * th.g;
    g.gain.setValueAtTime(0, t0);
    g.gain.linearRampToValueAtTime(peak, t0 + 0.008);
    g.gain.exponentialRampToValueAtTime(0.0004, t0 + dur);
    osc.connect(g).connect(this.master!);
    this.voices++;
    osc.onended = () => {
      this.voices--;
      // drop the finished nodes out of the graph instead of leaving them
      // connected to master for the GC to notice later
      osc.disconnect();
      g.disconnect();
    };
    osc.start(t0);
    osc.stop(t0 + dur + 0.05);
  }

  /** feather-quiet tick for hovering cards */
  hover() {
    const now = performance.now();
    if (now - this.lastHover < HOVER_MIN_GAP_MS) return;
    this.lastHover = now;
    this.tone(1900, 0.035, { gain: 0.012 });
  }

  /** soft two-note confirm when an effect is applied */
  select() {
    this.tone(620, 0.1, { gain: 0.05 });
    this.tone(930, 0.16, { gain: 0.04, delay: 0.07 });
  }

  click() {
    this.tone(500, 0.055, { gain: 0.045, sweep: 1.35 });
  }

  toggleOn() {
    this.tone(420, 0.11, { gain: 0.05, sweep: 1.8 });
  }

  toggleOff() {
    this.tone(560, 0.11, { gain: 0.045, sweep: 0.55 });
  }

  /** airy view-change swish (filtered noise) */
  whoosh() {
    if (!this.ensure()) return;
    if (this.voices >= MAX_VOICES) return;
    const ctx = this.ctx!;
    const th = this.theme();
    const dur = 0.26 * th.d;
    if (!this.noise) {
      // Flat noise, built once per context; the filters and the gain envelope
      // below do all the shaping.
      const buf = ctx.createBuffer(1, Math.floor(ctx.sampleRate * 0.5), ctx.sampleRate);
      const d = buf.getChannelData(0);
      for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
      this.noise = buf;
    }
    const t0 = ctx.currentTime;
    const src = ctx.createBufferSource();
    src.buffer = this.noise;
    // A *narrow* sweeping band is what reads as a swish. The old Q of 1.1 was
    // so wide it passed the noise through almost unfiltered, which just sounds
    // like a burst of static.
    const band = ctx.createBiquadFilter();
    band.type = "bandpass";
    band.Q.value = 4;
    band.frequency.setValueAtTime(520, t0);
    band.frequency.exponentialRampToValueAtTime(2600, t0 + dur);
    // tame the brittle top end that makes filtered noise hiss
    const tame = ctx.createBiquadFilter();
    tame.type = "lowpass";
    tame.frequency.value = 5200;
    const g = ctx.createGain();
    // fade in: starting noise at full amplitude cracks
    g.gain.setValueAtTime(0, t0);
    // gain is high vs the tones because the narrow band throws away most of
    // the noise energy; this lands at the same peak level as the old cue
    g.gain.linearRampToValueAtTime(0.13 * th.g, t0 + 0.03);
    g.gain.exponentialRampToValueAtTime(0.0004, t0 + dur);
    src.connect(band).connect(tame).connect(g).connect(this.master!);
    this.voices++;
    src.onended = () => {
      this.voices--;
      src.disconnect();
      band.disconnect();
      tame.disconnect();
      g.disconnect();
    };
    src.start(t0);
    src.stop(t0 + dur + 0.02);
  }
}

export const sfx = new Sfx();
