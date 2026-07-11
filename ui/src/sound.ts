// Synthesized interaction sounds — no audio assets, just tiny WebAudio
// envelopes. Everything routes through one master gain so the Appearance
// settings can mute or attenuate globally.

import { store } from "./state";

class Sfx {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;

  private ensure(): boolean {
    if (!store.ui.sounds) return false;
    if (!this.ctx) {
      try {
        this.ctx = new AudioContext();
        this.master = this.ctx.createGain();
        this.master.connect(this.ctx.destination);
      } catch {
        return false;
      }
    }
    if (this.ctx.state === "suspended") void this.ctx.resume();
    this.master!.gain.value = store.ui.volume * 0.5;
    return true;
  }

  private tone(freq: number, dur: number, opts: { type?: OscillatorType; gain?: number; sweep?: number; delay?: number } = {}) {
    if (!this.ensure()) return;
    const ctx = this.ctx!;
    const t0 = ctx.currentTime + (opts.delay ?? 0);
    const osc = ctx.createOscillator();
    const g = ctx.createGain();
    osc.type = opts.type ?? "sine";
    osc.frequency.setValueAtTime(freq, t0);
    if (opts.sweep) osc.frequency.exponentialRampToValueAtTime(Math.max(40, freq * opts.sweep), t0 + dur);
    const peak = opts.gain ?? 0.08;
    g.gain.setValueAtTime(0, t0);
    g.gain.linearRampToValueAtTime(peak, t0 + 0.008);
    g.gain.exponentialRampToValueAtTime(0.0004, t0 + dur);
    osc.connect(g).connect(this.master!);
    osc.start(t0);
    osc.stop(t0 + dur + 0.05);
  }

  /** feather-quiet tick for hovering cards */
  hover() {
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
    const ctx = this.ctx!;
    const dur = 0.22;
    const buf = ctx.createBuffer(1, ctx.sampleRate * dur, ctx.sampleRate);
    const d = buf.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / d.length);
    const src = ctx.createBufferSource();
    src.buffer = buf;
    const f = ctx.createBiquadFilter();
    f.type = "bandpass";
    f.frequency.setValueAtTime(700, ctx.currentTime);
    f.frequency.exponentialRampToValueAtTime(2400, ctx.currentTime + dur);
    f.Q.value = 1.1;
    const g = ctx.createGain();
    g.gain.value = 0.05;
    src.connect(f).connect(g).connect(this.master!);
    src.start();
  }
}

export const sfx = new Sfx();
