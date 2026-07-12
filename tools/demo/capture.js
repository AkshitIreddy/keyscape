// Capture frames of the LIVE Keyscape UI for the README demo animation.
//
// This renders the real web frontend in headless Chromium, pointed at the
// Vite dev server (`cd ui && npm run dev`, serving :5173). Because the UI
// holds a live WebSocket to the lighting core, the on-screen keyboard preview
// animates exactly as the installed app does — so these frames are the genuine
// app, not a mockup or a screen recording.
//
// Prereqs: `npm install` in this folder, the dev server running, and the
// lighting core running (so the preview has frames to draw).
//
// Usage:
//   node capture.js <outDir> <effectId> '<paramsJson>' [frames] [cadenceMs]
// Example (what the committed demo uses):
//   node capture.js frames magnetic_poles "{\"speed\":2.4,\"palette\":\"aurora\",\"flip_rate\":0}" 120 60

const puppeteer = require("puppeteer");
const fs = require("fs");
const path = require("path");

const OUT = process.argv[2] || "frames";
const EFFECT_ID = process.argv[3] || "magnetic_poles";
const PARAMS = JSON.parse(process.argv[4] || '{"speed":2.4,"palette":"aurora","flip_rate":0}');
const FRAMES = Number(process.argv[5] || 120);
const CADENCE = Number(process.argv[6] || 60); // ms between frames (steady)
const SETTLE_MS = Number(process.env.SETTLE_MS || 3000);
const URL = "http://localhost:5173";
const CORE_WS = "ws://127.0.0.1:53971";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

(async () => {
  const browser = await puppeteer.launch({
    headless: "new",
    args: ["--no-sandbox", "--force-color-profile=srgb", "--hide-scrollbars"],
  });
  const page = await browser.newPage();
  await page.setViewport({ width: 1160, height: 726, deviceScaleFactor: 1 });
  await page.goto(URL, { waitUntil: "networkidle2" });
  await page.waitForSelector(".card", { timeout: 20000 });
  await page.evaluate(() => document.getElementById("ob-skip")?.click()); // dismiss first-run tour

  // Set the effect and its params straight over the core's IPC. Going through
  // IPC (rather than driving the UI sliders) lets us use exact values the
  // sliders would clamp — e.g. flip_rate: 0, below the slider's 0.1 minimum,
  // which disables the effect's abrupt polarity flips.
  await page.evaluate(
    ({ ws, id, params }) =>
      new Promise((resolve) => {
        const sock = new WebSocket(ws);
        sock.onopen = () => {
          sock.send(JSON.stringify({ op: "set_effect", id, req: 1 }));
          sock.send(JSON.stringify({ op: "set_params", id, params, req: 2 }));
          setTimeout(() => {
            sock.close();
            resolve();
          }, 400);
        };
        sock.onerror = () => resolve();
      }),
    { ws: CORE_WS, id: EFFECT_ID, params: PARAMS }
  );
  await sleep(SETTLE_MS); // let the scene settle into a full, flowing state

  fs.mkdirSync(OUT, { recursive: true });
  for (let i = 0; i < FRAMES; i++) {
    const t0 = Date.now();
    await page.screenshot({
      path: path.join(OUT, `f${String(i).padStart(3, "0")}.png`),
      optimizeForSpeed: true,
    });
    const wait = CADENCE - (Date.now() - t0); // hold a steady cadence
    if (wait > 0) await sleep(wait);
  }
  console.log(`captured ${FRAMES} frames of ${EFFECT_ID} into ${OUT}`);
  await browser.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
