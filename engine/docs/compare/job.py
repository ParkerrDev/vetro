#!/usr/bin/env python3
"""job.py SCENE.json PARAMS.json BACKGROUND LAYER WIDTH HEIGHT SCALE > job.js

Writes the job the reference page reads: the scene's panes as exact shapes,
the preset as `hyprctl hyprglass model` printed it, and the two images.
LAYER may be "-" for none. Image paths are given relative to the page.
"""
import json
import sys


def main() -> int:
    if len(sys.argv) != 8:
        print(__doc__, file=sys.stderr)
        return 2
    scene = json.load(open(sys.argv[1]))
    params = json.load(open(sys.argv[2]))
    scale = float(sys.argv[7])
    shapes = []
    for layer in scene["layers"]:
        # Hyprland draws a layer at a whole device pixel: its box is rounded, and what
        # the client drew in it keeps its own size.
        ox, oy = (0, 0) if layer.get("full") else (round(layer["x"] * scale) / scale, round(layer["y"] * scale) / scale)
        for s in layer["shapes"]:
            # a dim the layer draws behind its cards is no pane
            if s.get("pane") is False:
                continue
            radius = min(s.get("r", 0), s["w"] / 2, s["h"] / 2)
            shapes.append({"x": ox + s["x"], "y": oy + s["y"], "w": s["w"], "h": s["h"], "r": radius,
                           "n": s.get("n", 4) if s.get("kind") == "squircle" else 2})
    job = {"width": int(sys.argv[5]), "height": int(sys.argv[6]), "scale": scale,
           "background": sys.argv[3], "layer": None if sys.argv[4] == "-" else sys.argv[4],
           "shapes": shapes, "params": params}
    print("window.JOB = " + json.dumps(job, indent=1) + ";")
    return 0


if __name__ == "__main__":
    sys.exit(main())
