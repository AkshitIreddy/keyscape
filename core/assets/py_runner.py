# Keyscape Python effect runner. Spawned by keyscape-core; speaks one JSON
# object per line over stdin/stdout. Not meant to be run by hand.
#
# Effect scripts must define:
#   EFFECT = { "id": "...", "name": "...", ... }        (manifest, JSON-style)
#   def render(t, dt, keys, params, state, req): ...    (returns colors)
# and may define:
#   def setup(keys, state): ...                         (called once)
#
# render() returns a list of (r, g, b) 0-255 tuples, one per key (in `keys`
# order), or a dict {key_index: (r, g, b)} for sparse updates over black.

import importlib.util
import json
import sys

path = sys.argv[1]
spec = importlib.util.spec_from_file_location("keyscape_effect", path)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

if len(sys.argv) > 2 and sys.argv[2] == "--manifest":
    print(json.dumps(mod.EFFECT), flush=True)
    sys.exit(0)

init = json.loads(sys.stdin.readline())
keys = init["keys"]
state = {"seed": init.get("seed", 0)}
if hasattr(mod, "setup"):
    mod.setup(keys, state)
print(json.dumps({"ready": True}), flush=True)

n = len(keys)
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    req = json.loads(line)
    colors = mod.render(req["t"], req["dt"], keys, req.get("params", {}), state, req)
    if isinstance(colors, dict):
        arr = [[0, 0, 0]] * n
        for k, v in colors.items():
            i = int(k)
            if 0 <= i < n:
                arr[i] = [int(v[0]), int(v[1]), int(v[2])]
        colors = arr
    out = []
    for c in colors[:n]:
        out.append([max(0, min(255, int(c[0]))), max(0, min(255, int(c[1]))), max(0, min(255, int(c[2])))])
    while len(out) < n:
        out.append([0, 0, 0])
    sys.stdout.write(json.dumps(out))
    sys.stdout.write("\n")
    sys.stdout.flush()
