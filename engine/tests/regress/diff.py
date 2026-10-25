#!/usr/bin/env python3
"""diff.py OLD.png NEW.png SCENE.json SCALE [OUT.png]

Compares two pictures of one scene pixel by pixel, in levels of 8 bits per
channel, and says how far apart they are over the whole picture and in each
region the scene is about (the bar, a card inside a larger layer, a menu on a
dim, a notification), each with 60 pixels around it for its shadow. Exits 0
when no pixel of the whole picture differs by more than one level.
"""
import json
import sys

import numpy as np
from PIL import Image

LIMIT = 1
SHADOW = 60


def regions(scene, scale):
    """Named boxes in device pixels: x1, y1, x2, y2."""
    found = {}
    for layer in scene["layers"]:
        full = layer.get("full")
        ox, oy = (0, 0) if full else (layer["x"], layer["y"])
        for shape in layer["shapes"]:
            if shape.get("pane") is False:
                continue
            x, y = ox + shape["x"], oy + shape["y"]
            box = (x * scale, y * scale, (x + shape["w"]) * scale, (y + shape["h"]) * scale)
            ns = layer["ns"]
            if ns == "lab:bar":
                found["bar strip"] = box
            elif ns == "lab:notif":
                found["notification"] = box
            elif ns == "lab:menu" or shape.get("title") == "MENU":
                found["menu" + (" on a dim" if len(layer["shapes"]) > 1 and any(s.get("pane") is False for s in layer["shapes"]) else "")] = box
            elif ns == "lab:full" and shape.get("title") == "NOTIFICATION":
                found["card inside a larger layer"] = box
    return found


def main():
    if len(sys.argv) not in (5, 6):
        print(__doc__)
        return 2
    old = np.asarray(Image.open(sys.argv[1]).convert("RGB"), dtype=np.int16)
    new = np.asarray(Image.open(sys.argv[2]).convert("RGB"), dtype=np.int16)
    scene = json.load(open(sys.argv[3]))
    scale = float(sys.argv[4])
    if old.shape != new.shape:
        print("the two pictures differ in size:", old.shape, new.shape)
        return 1
    difference = np.abs(old - new).max(axis=2)
    height, width = difference.shape
    print("  whole picture: worst %d levels, %d of %d pixels differ at all, %d by more than %d" % (
        difference.max(), int((difference > 0).sum()), difference.size, int((difference > LIMIT).sum()), LIMIT))
    for name, (x1, y1, x2, y2) in regions(scene, scale).items():
        x1, y1 = max(0, int(x1) - SHADOW), max(0, int(y1) - SHADOW)
        x2, y2 = min(width, int(x2) + SHADOW), min(height, int(y2) + SHADOW)
        part = difference[y1:y2, x1:x2]
        print("  %-28s worst %d levels, %d pixels differ at all, %d by more than %d" % (name, part.max(), int((part > 0).sum()), int((part > LIMIT).sum()), LIMIT))
    if len(sys.argv) == 6:
        Image.fromarray(np.clip(difference * 60, 0, 255).astype(np.uint8)).save(sys.argv[5])
    return 0 if difference.max() <= LIMIT else 1


if __name__ == "__main__":
    sys.exit(main())
