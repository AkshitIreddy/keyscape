// Live keyboard preview: real key geometry from the daemon's layout, painted
// with the exact frame bytes the hardware receives (inverse-gamma'd for the
// screen), plus a soft glow pass so it feels like the physical deck.

import { LayoutInfo } from "./state";
import { store } from "./state";

const GAMMA = 1.8;

export class KeyboardView {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private layout: LayoutInfo | null = null;
  private frame: Uint8Array | null = null;
  private rafPending = false;
  private lut: number[] = [];
  // quarter-res offscreen bloom layer: soft glow for ~6% of the overdraw
  private bloom: HTMLCanvasElement | null = null;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d")!;
    for (let i = 0; i < 256; i++) {
      this.lut[i] = Math.round(Math.pow(i / 255, 1 / GAMMA) * 255);
    }
    new ResizeObserver(() => this.schedule()).observe(canvas);
    document.addEventListener("visibilitychange", () => {
      if (!document.hidden) this.schedule();
    });
  }

  setLayout(layout: LayoutInfo) {
    this.layout = layout;
    this.schedule();
  }

  onFrame(bytes: Uint8Array) {
    this.frame = bytes;
    // minimized/hidden window: keep the latest frame but don't burn GPU
    if (document.hidden) return;
    this.schedule();
  }

  // rAF when the compositor is awake (vsync-aligned), but with a timeout
  // fallback so the preview still paints when the window is occluded and
  // rAF stops firing. The engine's ~30 fps stream is the real clock.
  private schedule() {
    if (this.rafPending) return;
    this.rafPending = true;
    let done = false;
    const run = () => {
      if (done) return;
      done = true;
      this.rafPending = false;
      this.draw();
    };
    const timer = window.setTimeout(run, 50);
    requestAnimationFrame(() => {
      clearTimeout(timer);
      run();
    });
  }

  private colorOf(led: number): [number, number, number] {
    if (!this.frame || led * 3 + 2 >= this.frame.length) return [0, 0, 0];
    return [
      this.lut[this.frame[led * 3]],
      this.lut[this.frame[led * 3 + 1]],
      this.lut[this.frame[led * 3 + 2]],
    ];
  }

  private draw() {
    const { canvas, ctx } = this;
    // cap dpr: glowing rounded rects don't need retina density
    const dpr = Math.min(window.devicePixelRatio || 1, 1.25);
    const cw = canvas.clientWidth;
    const ch = canvas.clientHeight;
    if (cw === 0 || ch === 0) return;
    if (canvas.width !== cw * dpr || canvas.height !== ch * dpr) {
      canvas.width = cw * dpr;
      canvas.height = ch * dpr;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cw, ch);
    const lay = this.layout;
    if (!lay) return;

    // fit keyboard rect (aspect w:h) into canvas with margins; leave a strip
    // at the bottom for the light bar.
    const mx = 26;
    const myTop = 30;
    const myBot = 34;
    const availW = cw - mx * 2;
    const availH = ch - myTop - myBot;
    const scale = Math.min(availW / lay.aspect, availH / 1.0);
    const kbW = lay.aspect * scale;
    const kbH = 1.0 * scale;
    const ox = (cw - kbW) / 2;
    const oy = myTop + (availH - kbH) / 2;

    const glow = store.ui.glow;

    // glow pass: plain circles into a quarter-res offscreen layer, then one
    // additive upscale — the bilinear stretch is the blur, no gradients, no
    // per-key full-res overdraw.
    if (glow) {
      const bw = Math.max(64, cw >> 2);
      const bh = Math.max(24, ch >> 2);
      if (!this.bloom) this.bloom = document.createElement("canvas");
      if (this.bloom.width !== bw || this.bloom.height !== bh) {
        this.bloom.width = bw;
        this.bloom.height = bh;
      }
      const bctx = this.bloom.getContext("2d")!;
      bctx.clearRect(0, 0, bw, bh);
      const sx = bw / cw;
      const sy = bh / ch;
      for (const k of lay.keys) {
        const [r, g, b] = this.colorOf(k.led);
        const luma = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
        if (luma < 0.09) continue;
        const w = k.w * lay.aspect * scale;
        const h = k.h * kbH;
        const cx = (ox + k.x * lay.aspect * scale + w / 2) * sx;
        const cy = (oy + k.y * kbH + h / 2) * sy;
        const rad = Math.max(w * sx, h * sy) * (0.9 + luma * 1.1);
        bctx.beginPath();
        bctx.arc(cx, cy, rad, 0, Math.PI * 2);
        bctx.fillStyle = `rgba(${r},${g},${b},${0.22 * luma})`;
        bctx.fill();
      }
      ctx.globalCompositeOperation = "lighter";
      ctx.drawImage(this.bloom, 0, 0, cw, ch);
      // second, slightly larger pass softens the circle edges
      ctx.globalAlpha = 0.5;
      ctx.drawImage(this.bloom, -cw * 0.02, -ch * 0.02, cw * 1.04, ch * 1.04);
      ctx.globalAlpha = 1;
      ctx.globalCompositeOperation = "source-over";
    }

    // key caps
    for (const k of lay.keys) {
      const [r, g, b] = this.colorOf(k.led);
      const x = ox + k.x * lay.aspect * scale;
      const y = oy + k.y * kbH;
      const w = k.w * lay.aspect * scale - 1.5;
      const h = k.h * kbH - 1.5;
      ctx.beginPath();
      ctx.roundRect(x, y, w, h, Math.min(4.5, w * 0.18));
      ctx.fillStyle = "#111522";
      ctx.fill();
      const luma = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
      ctx.fillStyle = `rgba(${r},${g},${b},${0.28 + 0.72 * Math.min(1, luma * 1.6)})`;
      ctx.fill();
      // subtle top bevel
      ctx.fillStyle = "rgba(255,255,255,0.05)";
      ctx.beginPath();
      ctx.roundRect(x, y, w, h * 0.42, Math.min(4.5, w * 0.18));
      ctx.fill();
    }

    // light bar: four segments below the deck (169 R1, 170 R2, 172 L2, 173 R3
    // — draw left-to-right as L2, R2, R1+R3 blend edges)
    const barY = oy + kbH + 12;
    const segs = [173, 172, 170, 169]; // left edge → right edge
    const segW = kbW / segs.length;
    for (let i = 0; i < segs.length; i++) {
      const [r, g, b] = this.colorOf(segs[i]);
      ctx.beginPath();
      ctx.roundRect(ox + i * segW + 2, barY, segW - 4, 5, 3);
      ctx.fillStyle = `rgba(${r},${g},${b},0.9)`;
      ctx.fill();
      if (glow) {
        ctx.shadowColor = `rgba(${r},${g},${b},0.8)`;
        ctx.shadowBlur = 10;
        ctx.fill();
        ctx.shadowBlur = 0;
      }
    }

    // rear light strip (chassis rear, under the lid logo) — drawn above the
    // deck; the daemon already flips it, so reverse again for the user's view
    const rearLeds = lay.aux
      .filter((a) => a.name.startsWith("Rear"))
      .map((a) => a.led)
      .sort((a, b) => b - a);
    if (rearLeds.length > 0) {
      const ry = oy - 13;
      const rw = (kbW - 26) / rearLeds.length;
      for (let i = 0; i < rearLeds.length; i++) {
        const [r, g, b] = this.colorOf(rearLeds[i]);
        ctx.beginPath();
        ctx.roundRect(ox + 26 + i * rw + 0.5, ry, rw - 1, 4, 2);
        ctx.fillStyle = `rgba(${r},${g},${b},0.9)`;
        ctx.fill();
      }
    }

    // lid logo dot, top-left beside the rear strip
    const [lr, lg, lb] = this.colorOf(168);
    ctx.beginPath();
    ctx.arc(ox + 9, oy - 11, 5, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(${lr},${lg},${lb},0.95)`;
    if (glow) {
      ctx.shadowColor = `rgba(${lr},${lg},${lb},0.9)`;
      ctx.shadowBlur = 12;
    }
    ctx.fill();
    ctx.shadowBlur = 0;
  }
}
