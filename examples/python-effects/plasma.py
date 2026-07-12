# Keyscape example effect: classic plasma, tinted by the user's palette.
# Copy into  %APPDATA%\Keyscape\effects\  and restart the core (tray > Quit,
# then relaunch) to see it appear under the "Custom" category.

EFFECT = {
    "id": "py_plasma",
    "name": "Py Plasma",
    "category": "Custom",
    "blurb": "Example Python effect: three interfering sine fields.",
    "palette": "synthwave",
    "params": [
        {"key": "scale", "label": "Scale", "kind": "slider",
         "min": 0.5, "max": 3.0, "step": 0.1, "default": 1.4},
        {"key": "warp", "label": "Warp", "kind": "slider",
         "min": 0.0, "max": 1.0, "step": 0.05, "default": 0.5},
    ],
}

import math


def render(t, dt, keys, params, state, req):
    s = params.get("scale", 1.4) * 3.0
    warp = params.get("warp", 0.5)
    pal = req.get("palette") or [[255, 255, 255]] * 16

    out = []
    for k in keys:
        x, y = k["cx"], k["cy"]
        v = (
            math.sin(x * s + t)
            + math.sin((y * s + t * 1.31) * 1.7)
            + math.sin((x + y) * s * 0.8 + t * 0.7 + warp * math.sin(t * 0.4) * 4)
        )
        f = (v + 3.0) / 6.0  # 0..1
        c = pal[min(15, int(f * 16))]
        bright = 0.35 + 0.65 * f
        out.append((c[0] * bright, c[1] * bright, c[2] * bright))
    return out
