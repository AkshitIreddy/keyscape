# Demo renderer

Regenerates the looping preview in the top-level README
(`docs/assets/keyscape.webp`) from the **live** app UI — see
[capture.js](capture.js) and [make-demo.ps1](make-demo.ps1).

It is a maintainer tool, not part of the shipped app. The top-level README's
["How the demo animation is made"](../../README.md#how-the-demo-animation-is-made)
section explains the technique.

## Run it

```powershell
# 1. Start the app in dev mode (two terminals):
cd ../../ui ; npm run dev          # frontend on :5173
keyscape-core run                  # core daemon (feeds the preview)

# 2. Install puppeteer here (first time only):
npm install

# 3. Render (needs ImageMagick on PATH):
powershell -ExecutionPolicy Bypass -File make-demo.ps1
```

Tweak the effect/palette/speed with parameters, e.g.:

```powershell
powershell -File make-demo.ps1 -Effect nebula_drift -Params '{"speed":1.8,"palette":"candy"}'
```
