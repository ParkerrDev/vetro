#!/usr/bin/env python3
"""backgrounds.py DIR: the pictures the scenes lie over, all 2560x1600.

Deterministic and made here, so that nothing depends on anyone's wallpaper:
a busy dark one (soft colour, text, fine lines and a grid, so that bent light
and blur are both visible) and a bright one with dark text.
"""
import math
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

W, H = 2560, 1600


def font(size):
    for path in ("/usr/share/fonts/TTF/DejaVuSans.ttf", "/usr/share/fonts/noto/NotoSans-Regular.ttf"):
        try:
            return ImageFont.truetype(path, size)
        except OSError:
            continue
    return ImageFont.load_default()


def base(dark):
    y, x = np.mgrid[0:H, 0:W].astype(np.float64)
    t = x / W * 0.7 + y / H * 0.3
    if dark:
        rgb = np.stack([18 + 40 * np.sin(t * 5.0) ** 2, 22 + 50 * np.cos(t * 3.0 + 1.0) ** 2, 40 + 70 * t], axis=2)
    else:
        rgb = np.stack([226 + 26 * t, 236 + 12 * t, 250 - 14 * t], axis=2)
    return Image.fromarray(np.clip(rgb, 0, 255).astype(np.uint8), "RGB").convert("RGBA")


def decorate(image, dark):
    blobs = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(blobs)
    colours = [(255, 120, 90), (90, 160, 255), (120, 220, 150), (250, 200, 70), (190, 110, 240), (255, 90, 160)]
    for k in range(14):
        cx, cy = 200 + (k * 487) % (W - 300), 120 + (k * 331) % (H - 200)
        r = 90 + (k * 53) % 200
        d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=colours[k % 6] + (200 if dark else 170,))
    image = Image.alpha_composite(image, blobs.filter(ImageFilter.GaussianBlur(5)))
    d = ImageDraw.Draw(image)
    ink = (235, 240, 250, 255) if dark else (24, 26, 32, 255)
    for i in range(16):
        d.text((60, 70 + i * 96), "The quick brown fox jumps over the lazy dog 0123456789 Liquid Glass " * 2, fill=ink, font=font(34 if i % 3 else 52))
    line = (255, 255, 255, 90) if dark else (40, 60, 90, 200)
    for i in range(0, W, 160):
        d.line((i, 0, i + 500, H), fill=line, width=2)
    for x in range(0, W, 80):
        d.line((x, 0, x, H), fill=line, width=1)
    for y in range(0, H, 80):
        d.line((0, y, W, y), fill=line, width=1)
    return image.convert("RGB")


def main():
    out = sys.argv[1]
    decorate(base(True), True).save(f"{out}/bg-busy.png")
    decorate(base(False), False).save(f"{out}/bg-light.png")


if __name__ == "__main__":
    main()
