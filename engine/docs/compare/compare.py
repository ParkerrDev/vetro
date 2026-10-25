#!/usr/bin/env python3
"""compare.py PLUGIN.png REFERENCE.png SCENE.json SCALE BEZEL OUT_PREFIX [X Y W H]

Puts the plugin's picture and the reference page's side by side with their
difference (times 4), and says how far apart they are in each part of a pane:
its face, the rim's band (BEZEL logical pixels from the outline), the two
pixels on the outline, and the shadow beside it. Differences are in levels of
8 bits per channel. X Y W H, in device pixels, crops the side by side.
"""
import json
import sys

import numpy as np
from PIL import Image


def shape_distance(px, py, x, y, w, h, r, n):
    hx, hy = w / 2, h / 2
    r = min(r, hx, hy)
    qx = np.abs(px - x - hx) - hx + r
    qy = np.abs(py - y - hy) - hy + r
    mx, my = np.maximum(qx, 0), np.maximum(qy, 0)
    corner = np.hypot(mx, my) if n == 2 else (mx ** n + my ** n) ** (1 / n)
    return np.minimum(np.maximum(qx, qy), 0) + corner - r


def main() -> int:
    if len(sys.argv) not in (7, 11):
        print(__doc__)
        return 2
    plugin = np.asarray(Image.open(sys.argv[1]).convert("RGB"), dtype=np.float64)
    reference = np.asarray(Image.open(sys.argv[2]).convert("RGB"), dtype=np.float64)
    scene = json.load(open(sys.argv[3]))
    k = float(sys.argv[4])
    bezel = float(sys.argv[5]) * k
    prefix = sys.argv[6]
    if plugin.shape != reference.shape:
        print("the two pictures differ in size:", plugin.shape, reference.shape)
        return 1

    height, width = plugin.shape[:2]
    py, px = np.mgrid[0:height, 0:width]
    px = px + 0.5
    py = py + 0.5
    distance = np.full((height, width), 1e9)
    for layer in scene["layers"]:
        # a layer of its own size is drawn at a whole device pixel
        ox, oy = (0, 0) if layer.get("full") else (round(layer["x"] * k) / k, round(layer["y"] * k) / k)
        for s in layer["shapes"]:
            if s.get("pane") is False:
                continue
            n = s.get("n", 4) if s.get("kind") == "squircle" else 2
            distance = np.minimum(distance, shape_distance(px, py, (ox + s["x"]) * k, (oy + s["y"]) * k, s["w"] * k, s["h"] * k, s.get("r", 0) * k, n))

    difference = np.abs(plugin - reference)
    worst = difference.max(axis=2)
    regions = {
        "face (deeper than the bezel)": distance < -bezel,
        "band (the bezel)": (distance >= -bezel) & (distance < -2.0),
        "outline (2 pixels)": (distance >= -2.0) & (distance < 0.5),
        "shadow (to 60 pixels out)": (distance >= 0.5) & (distance < 60.0),
    }
    lines = []
    for name, mask in regions.items():
        if not mask.any():
            continue
        values = worst[mask]
        lines.append("%-30s pixels %8d   mean %5.2f   95%% %5.1f   99%% %5.1f   max %5.1f   within 4 levels %5.1f%%" % (
            name, values.size, difference[mask].mean(), np.percentile(values, 95), np.percentile(values, 99), values.max(), 100.0 * (values <= 4).mean()))
    report = "\n".join(lines)
    print(report)
    open(prefix + ".txt", "w").write(report + "\n")

    crop = (0, 0, width, height) if len(sys.argv) == 7 else (int(sys.argv[7]), int(sys.argv[8]), int(sys.argv[7]) + int(sys.argv[9]), int(sys.argv[8]) + int(sys.argv[10]))
    parts = [Image.fromarray(a.astype(np.uint8)).crop(crop) for a in (plugin, reference, np.clip(difference * 4.0, 0, 255))]
    gap = 12
    out = Image.new("RGB", (parts[0].width * 3 + 2 * gap, parts[0].height), (255, 255, 255))
    for i, part in enumerate(parts):
        out.paste(part, (i * (part.width + gap), 0))
    out.save(prefix + ".png")
    return 0


if __name__ == "__main__":
    sys.exit(main())
