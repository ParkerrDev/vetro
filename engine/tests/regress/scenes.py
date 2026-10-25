#!/usr/bin/env python3
"""scenes.py DIR: the scenes the regression script draws, as the JSON lab.qml reads.

A 1600x1000 logical canvas at scale 1.6, the owner's panel. The same scenes the
model's comparison with its reference page uses (docs/compare): the budget
scene (a bar, two cards, a notification and a menu), cards inside one larger
layer, and a menu on a dim. Every length is in logical pixels.
"""
import json
import os
import sys

DARK, LIGHT = [0, 0, 0, 0.12], [1, 1, 1, 0.12]
ROWS = ["Apps", "Learn", "Trigger", "Style", "Setup", "Install", "Remove", "Update", "About", "System"]


def one(ns, x, y, w, h, r, fill, **kw):
    shape = {"kind": "rrect", "x": 0, "y": 0, "w": w, "h": h, "r": r, "fill": fill}
    shape.update(kw)
    return {"ns": ns, "x": x, "y": y, "w": w, "h": h, "shapes": [shape]}


def desk(bg, text, fill, pan=0):
    """The budget scene of docs/model.md: a bar, two cards, a notification and a menu."""
    menu = {"kind": "rrect", "x": 520, "y": 250, "w": 560, "h": 470, "r": 18, "fill": fill, "title": "MENU", "rows": ROWS, "selected": 2, "shadow": 0.02}
    return {"bg": bg, "text": text, "pan": pan, "threshold": 0.04, "layers": [
        one("lab:bar", 5, 5, 1590, 30, 15, fill),
        one("lab:launch", 16, 56, 384, 274, 14, fill, title="NEXT LAUNCH", big="T- 01:16:31"),
        one("lab:weather", 1266, 56, 318, 238, 14, fill, title="CAPE CANAVERAL", big="24 Clear"),
        one("lab:notif", 1200, 320, 384, 92, 18, fill, title="NOTIFICATION", big=""),
        {"ns": "lab:menu", "full": True, "overlay": True, "x": 0, "y": 0, "w": 1600, "h": 1000, "shapes": [menu]},
    ]}


def full(bg, text, fill):
    """Several cards, a menu, a capsule, circles, a squircle and a dock inside one see-through layer."""
    shapes = [
        {"kind": "rrect", "x": 70, "y": 90, "w": 420, "h": 260, "r": 18, "fill": fill, "title": "NOTIFICATION", "big": "Liftoff"},
        {"kind": "rrect", "x": 560, "y": 90, "w": 300, "h": 420, "r": 14, "fill": fill, "title": "MENU", "rows": ROWS[:8], "selected": 1, "shadow": 0.02},
        {"kind": "rrect", "x": 930, "y": 110, "w": 360, "h": 60, "r": 30, "fill": fill, "label": "VOLUME"},
        {"kind": "rrect", "x": 930, "y": 200, "w": 64, "h": 64, "r": 32, "fill": fill},
        {"kind": "rrect", "x": 1010, "y": 200, "w": 64, "h": 64, "r": 32, "fill": fill},
        {"kind": "squircle", "x": 1110, "y": 200, "w": 180, "h": 180, "r": 70, "n": 4, "fill": fill, "label": "SQUIRCLE"},
        {"kind": "rrect", "x": 1340, "y": 90, "w": 200, "h": 300, "r": 0, "fill": fill, "label": "SQUARE"},
        {"kind": "rrect", "x": 70, "y": 420, "w": 420, "h": 200, "r": 40, "fill": fill, "title": "WEATHER", "big": "24"},
        {"kind": "rrect", "x": 300, "y": 560, "w": 320, "h": 180, "r": 40, "fill": fill},
        {"kind": "rrect", "x": 930, "y": 440, "w": 610, "h": 300, "r": 24, "fill": fill, "title": "PANEL", "big": "Keyboard"},
        {"kind": "rrect", "x": 70, "y": 800, "w": 1470, "h": 120, "r": 60, "fill": fill, "label": "DOCK"},
    ]
    return {"bg": bg, "text": text, "pan": 0, "threshold": 0.04, "layers": [
        one("lab:bar", 5, 5, 1590, 30, 15, fill),
        {"ns": "lab:full", "full": True, "x": 0, "y": 0, "w": 1600, "h": 1000, "shapes": shapes},
    ]}


def dimmed(bg, text, dim, card):
    """What Omarchy's menu is: one full-screen layer, a dim over all of it, and cards on the dim."""
    shapes = [
        {"kind": "rrect", "x": 0, "y": 0, "w": 1600, "h": 1000, "r": 0, "fill": [0, 0, 0, dim], "pane": False},
        {"kind": "rrect", "x": 500, "y": 220, "w": 600, "h": 420, "r": 18, "fill": [0, 0, 0, card], "title": "MENU", "rows": ROWS[:8], "selected": 1},
        {"kind": "rrect", "x": 500, "y": 680, "w": 600, "h": 60, "r": 30, "fill": [0, 0, 0, card], "label": "SEARCH"},
        {"kind": "squircle", "x": 1180, "y": 220, "w": 200, "h": 200, "r": 70, "n": 4, "fill": [0, 0, 0, card]},
    ]
    # the threshold goes midway between the dim and the card: 1 - (1 - d)(1 - c) is what a card holds
    midway = round((dim + (1 - (1 - dim) * (1 - card))) / 2, 3)
    return {"bg": bg, "text": text, "pan": 0, "threshold": midway, "layers": [{"ns": "lab:full", "full": True, "x": 0, "y": 0, "w": 1600, "h": 1000, "shapes": shapes}]}


SCENES = {
    "desk": desk("bg-busy.png", "#ffffff", DARK),
    "desk-light": desk("bg-light.png", "#101216", LIGHT),
    "desk-live": desk("bg-busy.png", "#ffffff", DARK, pan=30),
    "full": full("bg-busy.png", "#ffffff", DARK),
    "dimmed": dimmed("bg-busy.png", "#ffffff", 0.4, 0.6),
}


def main():
    out = sys.argv[1]
    for name, scene in SCENES.items():
        with open(os.path.join(out, name + ".json"), "w") as f:
            json.dump(scene, f, indent=1)


if __name__ == "__main__":
    main()
