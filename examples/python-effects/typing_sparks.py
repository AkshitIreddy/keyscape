# Keyscape example effect: sparks fly sideways from every key you press.
# Demonstrates the taps stream, persistent state, and needs_input.

EFFECT = {
    "id": "py_typing_sparks",
    "name": "Py Typing Sparks",
    "category": "Custom",
    "blurb": "Example Python effect: keypresses shoot sideways sparks.",
    "palette": "ember",
    "needs_input": True,
    "params": [
        {"key": "sparks", "label": "Sparks per key", "kind": "slider",
         "min": 1, "max": 8, "step": 1, "default": 4},
        {"key": "fade", "label": "Fade speed", "kind": "slider",
         "min": 0.5, "max": 4.0, "step": 0.1, "default": 1.6},
    ],
}

import random


def setup(keys, state):
    state["sparks"] = []          # each: [x, y, vx, life]
    state["rng"] = random.Random(state.get("seed", 1))


def render(t, dt, keys, params, state, req):
    rng = state["rng"]
    n_per = int(params.get("sparks", 4))
    fade = params.get("fade", 1.6)
    pal = req.get("palette") or [[255, 160, 40]] * 16

    for tap in req.get("taps", []):
        _, cx, cy = tap
        for _ in range(n_per):
            state["sparks"].append(
                [cx, cy, rng.uniform(0.4, 1.6) * rng.choice((-1, 1)), 1.0])

    alive = []
    for sp in state["sparks"]:
        sp[0] += sp[2] * dt
        sp[3] -= dt * fade
        if sp[3] > 0:
            alive.append(sp)
    state["sparks"] = alive[-200:]

    out = [(0, 0, 0)] * len(keys)
    for i, k in enumerate(keys):
        # faint idle floor so the board never looks dead
        base = pal[2]
        r, g, b = base[0] * 0.04, base[1] * 0.04, base[2] * 0.04
        for sp in state["sparks"]:
            d2 = (k["cx"] - sp[0]) ** 2 + (k["cy"] - sp[1]) ** 2
            if d2 < 0.04:
                w = sp[3] * max(0.0, 1.0 - d2 / 0.04)
                c = pal[min(15, int(sp[3] * 15))]
                r += c[0] * w
                g += c[1] * w
                b += c[2] * w
        out[i] = (r, g, b)
    return out
