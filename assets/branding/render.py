#!/usr/bin/env python3
"""Renders Vetro's artwork: the V of clear glass and the README banner.

The V is a rod of clear glass lying on a white tile, lit from the upper left,
and each pixel is worked out the way light would make it. A ray from the eye
bends where it enters the glass (Snell's law) and lands on the tile some way
off, so what the glass shows is the tile moved and magnified; the surface
reflects a little of a bright room, more at a grazing angle (Fresnel); the
light glints off it; and the rod's shadow is dark where its edges turn the
light away and light where its middle lets the light through. Faint rings on
the tile, ripples round the V, are there to be bent: through the glass they
warp as they do through Vetro's own.

Writes, beside this file:
  vetro.png          the mark, 512 pixels
  vetro.svg          the same picture as the app's scalable icon
  readme-banner.svg  the mark, the wordmark and the slogan; the lettering is
                     outlined from shell/fonts, so it needs no fonts installed

Needs numpy, Pillow and fontTools. Run from anywhere:
  python3 assets/branding/render.py
"""

import base64
import io
from pathlib import Path

import numpy as np
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from PIL import Image

HERE = Path(__file__).resolve().parent
FONTS = HERE.parent.parent / "shell" / "fonts"

# The V, in the tile's 256-unit square: its arms' tops, its tip, its width.
ARMS = ((80.0, 74.0), (176.0, 74.0))
TIP = (128.0, 168.0)
RADIUS = 22.0
JOIN = 5.0          # how far the inner join is rounded
THICK = 30.0        # the rod's height over the tile
FLAT = 4.0          # its profile: 2 a round rod, higher a flatter top
IOR = 1.5           # glass
LIGHT = np.array([-0.38, -0.62, 1.0]) / np.linalg.norm([-0.38, -0.62, 1.0])
SHINE = 90.0
ROOM = (1.0, 0.68)  # what the glass reflects: above, below
SHADOW = dict(opacity=0.32, dx=5.0, dy=12.0, blur=6.0, through=0.06, caustic=0.9)
RINGS = dict(centre=(128.0, 124.0), step=12.0, opacity=0.09)

SLOGAN = "Liquid glass for Omarchy"
INK = "#1d1d1f"
SOFT_INK = "#424245"


def segment_distance(px, py, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    t = np.clip(((px - a[0]) * dx + (py - a[1]) * dy) / (dx * dx + dy * dy), 0, 1)
    return np.hypot(px - a[0] - t * dx, py - a[1] - t * dy)


def v_distance(px, py):
    """Distance to the V's centre line; the two arms meet in a smooth minimum."""
    a = segment_distance(px, py, ARMS[0], TIP)
    b = segment_distance(px, py, ARMS[1], TIP)
    h = np.clip(0.5 + 0.5 * (b - a) / JOIN, 0, 1)
    return b + (a - b) * h - JOIN * h * (1 - h)


def blur(img, sd):
    """A separable Gaussian blur, sd in pixels."""
    n = int(sd * 3) + 1
    k = np.exp(-0.5 * (np.arange(-n, n + 1) / sd) ** 2)
    k /= k.sum()
    img = np.apply_along_axis(lambda m: np.convolve(m, k, mode="same"), 0, img)
    return np.apply_along_axis(lambda m: np.convolve(m, k, mode="same"), 1, img)


def shift(img, dx, dy):
    """Moves an image by whole pixels; what comes in from the edge is empty."""
    out = np.zeros_like(img)
    h, w = img.shape
    out[max(dy, 0):h + min(dy, 0), max(dx, 0):w + min(dx, 0)] = \
        img[max(-dy, 0):h + min(-dy, 0), max(-dx, 0):w + min(-dx, 0)]
    return out


def tile(px, py, s):
    """The bare tile: white going a little grey toward the bottom, with rings."""
    base = 1.0 - 0.065 * np.clip((py - 8) / 240, 0, 1)
    d = np.hypot(px - RINGS["centre"][0], py - RINGS["centre"][1])
    step = RINGS["step"]
    f = np.abs(((d + step / 2) % step) - step / 2)
    line = np.clip(1 - (f - 0.55) * s / 1.2, 0, 1) * np.clip(1.15 - d / 125, 0, 1)
    return base - RINGS["opacity"] * line


def surface(px, py, s):
    """The rod's height, its footprint and its normal."""
    d = v_distance(px, py)
    height = THICK * np.power(np.clip(1 - np.power(np.clip(d / RADIUS, 0, 1), FLAT), 0, 1), 1 / FLAT)
    gy, gx = np.gradient(height, 1 / s)
    nz = 1 / np.sqrt(1 + gx * gx + gy * gy)
    return d, height, (-gx * nz, -gy * nz, nz), np.hypot(gx, gy)


def lit_tile(inside, slope, s):
    """The tile as the light leaves it: its shadow, darker at the rod's edges,
    a caustic where the rod gathers the light, and a contact shadow."""
    o = SHADOW
    lost = np.where(inside, np.clip(slope * 0.9, 0, 1), 0.0)
    gather = np.where(inside, np.clip(1 - slope * 2.2, 0, 1), 0.0)
    block = np.where(inside, o["through"] + (1 - o["through"]) * lost, 0.0)
    shade = blur(shift(block, round(o["dx"] * s), round(o["dy"] * s)), o["blur"] * s) * o["opacity"]
    caustic = blur(shift(gather, round((o["dx"] + 2) * s), round((o["dy"] + 4) * s)), 3.0 * s) * o["caustic"] * 0.6
    contact = shift(blur(np.where(inside, 1.0, 0.0), 2.0 * s), 0, round(3 * s)) * 0.10
    return shade, caustic, contact


def render_mark(size, supersample=2):
    """The mark as an RGBA image, size pixels square."""
    n = size * supersample
    s = n / 256.0
    yy, xx = np.mgrid[0:n, 0:n].astype(np.float64)
    x, y = (xx + 0.5) / s, (yy + 0.5) / s
    d, height, (nx, ny, nz), slope = surface(x, y, s)
    inside = d < RADIUS
    shade, caustic, contact = lit_tile(inside, slope, s)

    def ground(px, py):
        ix = np.clip((px * s).astype(int), 0, n - 1)
        iy = np.clip((py * s).astype(int), 0, n - 1)
        sh = shade[iy, ix]
        return np.clip(tile(px, py, s) * (1 - sh - contact[iy, ix]) + caustic[iy, ix] * sh * 2.2, 0, 1)

    # Refraction of a ray straight down: T = eta I + (eta cos - sqrt(k)) N.
    eta = 1 / IOR
    c = eta * nz - np.sqrt(np.clip(1 - eta * eta * (1 - nz * nz), 0, 1))
    travel = height / np.maximum(eta - c * nz, 1e-3)
    seen = ground(x + c * nx * travel, y + c * ny * travel)
    f0 = ((1 - IOR) / (1 + IOR)) ** 2
    fresnel = np.clip(f0 + (1 - f0) * np.power(1 - np.clip(nz, 0, 1), 5), 0, 1)
    room = ROOM[1] + (ROOM[0] - ROOM[1]) * np.clip(0.5 - 2 * nz * ny * 0.9, 0, 1)
    half = (LIGHT + [0, 0, 1]) / np.linalg.norm(LIGHT + [0, 0, 1])
    glint = np.power(np.clip(nx * half[0] + ny * half[1] + nz * half[2], 0, 1), SHINE)
    glass = seen * (1 - fresnel) + room * fresnel + glint

    cover = np.clip((RADIUS - d) * s + 0.5, 0, 1)
    out = glass * cover + ground(x, y) * (1 - cover)
    rgb, alpha = tile_frame(out, x, y, s)
    img = np.concatenate([rgb * 255, alpha[..., None] * 255], -1).round().astype(np.uint8)
    return Image.fromarray(img, "RGBA").resize((size, size), Image.LANCZOS)


def tile_frame(grey, x, y, s):
    """Cuts the tile's rounded square out and draws its hairline."""
    def covered(inset):
        cx = np.clip(x, 8 + 56, 248 - 56)
        cy = np.clip(y, 8 + 56, 248 - 56)
        return np.clip(0.5 - (np.hypot(x - cx, y - cy) - 56 + inset) * s, 0, 1)
    alpha = covered(0)
    hair = alpha - covered(1)
    grey = np.clip(grey, 0, 1) * (1 - hair * 0.08)
    return np.stack([grey] * 3, -1), alpha


def png_data(image):
    buf = io.BytesIO()
    image.save(buf, "PNG", optimize=True)
    return buf.getvalue()


def data_uri(image):
    return "data:image/png;base64," + base64.b64encode(png_data(image)).decode()


def kerning(font):
    """Pair kerning from the font's GPOS, as {(left, right): units}."""
    pairs = {}
    for lookup in font["GPOS"].table.LookupList.Lookup:
        for sub in lookup.SubTable:
            if lookup.LookupType == 9:
                sub = sub.ExtSubTable
            if getattr(sub, "LookupType", lookup.LookupType) != 2:
                continue
            firsts = sub.Coverage.glyphs
            if sub.Format == 1:
                for first, pset in zip(firsts, sub.PairSet):
                    for rec in pset.PairValueRecord:
                        v = getattr(rec.Value1, "XAdvance", 0) or 0
                        pairs.setdefault((first, rec.SecondGlyph), v)
            elif sub.Format == 2:
                c1, c2 = sub.ClassDef1.classDefs, sub.ClassDef2.classDefs
                for first in firsts:
                    row = sub.Class1Record[c1.get(first, 0)]
                    for second, k2 in c2.items():
                        v = getattr(row.Class2Record[k2].Value1, "XAdvance", 0) or 0
                        if v:
                            pairs.setdefault((first, second), v)
    return pairs


def outline(font_file, text, size, x, baseline, tracking=0.0):
    """Text as SVG paths; returns the paths and the width they take."""
    font = TTFont(FONTS / font_file)
    glyphs, cmap, hmtx = font.getGlyphSet(), font.getBestCmap(), font["hmtx"]
    kern = kerning(font)
    scale = size / font["head"].unitsPerEm
    pen_x, paths, last = x, [], None
    for ch in text:
        name = cmap[ord(ch)]
        if last is not None:
            pen_x += kern.get((last, name), 0) * scale
        pen = SVGPathPen(glyphs, ntos=lambda v: f"{v:.2f}".rstrip("0").rstrip("."))
        glyphs[name].draw(TransformPen(pen, (scale, 0, 0, -scale, pen_x, baseline)))
        if pen.getCommands():
            paths.append(f'<path d="{pen.getCommands()}"/>')
        pen_x += hmtx[name][0] * scale + tracking
        last = name
    return "".join(paths), pen_x - tracking - x


def icon_svg(mark):
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">
  <title>Vetro</title>
  <desc>A V of clear glass on a white tile, bending the rings beneath it. Made by render.py.</desc>
  <image width="256" height="256" href="{data_uri(mark)}"/>
</svg>
'''


def banner_svg(mark):
    word, _ = outline("D-DINExp.otf", "VETRO", 100, 350, 206, tracking=16)
    slogan, _ = outline("D-DIN.otf", SLOGAN, 30, 354, 262)
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="400" viewBox="0 0 1200 400" fill="none">
  <title>Vetro. {SLOGAN}.</title>
  <desc>A V of clear glass beside the Vetro wordmark, on white. Made by render.py.</desc>
  <defs>
    <linearGradient id="paper" x1="600" y1="0" x2="600" y2="400" gradientUnits="userSpaceOnUse">
      <stop stop-color="#ffffff"/><stop offset="1" stop-color="#f5f5f7"/>
    </linearGradient>
  </defs>
  <rect x=".5" y=".5" width="1199" height="399" rx="28" fill="url(#paper)" stroke="#000" stroke-opacity=".08"/>
  <g transform="translate(72 88) scale(.875)"><image width="256" height="256" href="{data_uri(mark)}"/></g>
  <g fill="{INK}">{word}</g>
  <g fill="{SOFT_INK}">{slogan}</g>
</svg>
'''


def main():
    mark = render_mark(512)
    (HERE / "vetro.png").write_bytes(png_data(mark))
    (HERE / "vetro.svg").write_text(icon_svg(mark))
    (HERE / "readme-banner.svg").write_text(banner_svg(render_mark(448)))


if __name__ == "__main__":
    main()
