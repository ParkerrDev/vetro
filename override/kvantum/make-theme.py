#!/usr/bin/env python3
"""Build Vetro's Kvantum (Qt) theme: KvLibadwaita, recolored pure black and white.

KvLibadwaita (GPL-3.0, https://github.com/GabePoel/KvLibadwaita) is the Qt
twin of adw-gtk3/libadwaita, which GTK apps here use, so Qt and GTK apps get
the same shapes. This takes its dark variant at a pinned commit and:

  - maps its greys to what they'd be as white over black, so the window is
    black and buttons, fields and selections are white at low opacity, with
    a few groups set to match the GTK theme exactly (see GROUPS);
  - turns the blue accent white, and the ticks on white checkboxes black;
  - redraws menus and tooltips as black with a white hairline edge, because
    on a black window their shadows don't show;
  - sets the text colors in the .kvconfig to pure white (black on white).

Writes override/kvantum/vetro/, which `vetro on` links into ~/.config/Kvantum.
Run it again after changing anything here. It needs the network, once, for
KvLibadwaita; install.sh and the tests never run it.
"""

import pathlib
import re
import sys
import urllib.request

UPSTREAM = "https://raw.githubusercontent.com/GabePoel/KvLibadwaita"
COMMIT = "1f4e0bec44b13dabfa1fe4047aa8eeaccf2f3557"  # main, 2025-09-13
REPO = pathlib.Path(__file__).resolve().parent.parent.parent
OUT = REPO / "override/kvantum/vetro"

HAIRLINE = 0.24  # the launch card's glass edge (Style.glassEdge)
WINDOW_GREY = 0x1E  # KvLibadwaitaDark's window; greys map relative to this

# Colors that aren't greys, by hand.
ACCENTS = {
    "#3584e4": "#ffffff",  # the accent: checkboxes, sliders, progress
    "#4990e7": "#e6e6e6",  # the accent, hovered
    "#4285f4": "#ffffff",
    "#3daee9": "#ffffff",
    "#93cee9": "#d9d9d9",
    "#587392": "#8c8c8c",  # focus ring on text fields (GTK: white 50%)
    "#f04a50": "#ffffff",  # window buttons
    "#f2ffff": "#000000",  # ticks and dots on the (now white) checkboxes
    "#31363b": "#000000",
    "#eff0f1": "#ffffff",
    "#090000": "#000000",
    "#b74aff": "#b74aff",  # shadow size hints, never drawn
}

# Element groups set to match the GTK theme (libadwaita's currentColor mixes
# on black): buttons 10%, hover 15%, pressed 30%, checked 25%; fields 10%;
# selected rows 25%.
GROUPS = {
    "button-normal": "#1a1a1a",
    "button-focused": "#262626",
    "button-pressed": "#4d4d4d",
    "button-toggled": "#404040",
    "lineedit-normal": "#1a1a1a",
    "lineedit-focused": "#1a1a1a",
    "combo-normal": "#1a1a1a",
    "combo-focused": "#262626",
    "combo-pressed": "#4d4d4d",
    "combo-toggled": "#404040",
    "itemview-focused": "#1a1a1a",
    "itemview-toggled": "#404040",
    "itemview-pressed": "#4d4d4d",
    "menuitem-focused": "#262626",
    "menuitem-pressed": "#404040",
}

# Frames drawn from scratch: black, rounded, with a hairline edge.
FRAMES = {"menu-normal": (8, "#000000", 1.0), "tooltip-normal": (6, "#000000", 1.0)}

GENERAL_COLORS = """[GeneralColors]
window.color=#000000
base.color=#000000
alt.base.color=#0d0d0d
button.color=#1a1a1a
light.color=#333333
mid.light.color=#262626
dark.color=#000000
mid.color=#0d0d0d
highlight.color=#ffffff
inactive.highlight.color=#ffffff
tooltip.base.color=#000000
text.color=#ffffff
window.text.color=#ffffff
button.text.color=#ffffff
disabled.text.color=#666666
tooltip.text.color=#ffffff
highlight.text.color=#000000
inactive.highlight.text.color=#000000
link.color=#ffffff
link.visited.color=#ffffff
progress.indicator.text.color=#ffffff
"""


def fetch(name):
    cache = pathlib.Path("/tmp/kvantum-vetro") / COMMIT / name
    if not cache.exists():
        cache.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(f"{UPSTREAM}/{COMMIT}/{name}", timeout=30) as r:
            cache.write_bytes(r.read())
    return cache.read_text()


def hexrgb(c):
    h = c[1:]
    if len(h) == 3:
        h = "".join(x * 2 for x in h)
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def grey(v):
    return "#%02x%02x%02x" % ((round(v),) * 3)


def remap(c):
    """One upstream color to its black and white version."""
    c = c.lower()
    if len(c) == 4:
        c = "#" + "".join(x * 2 for x in c[1:])
    if c in ACCENTS:
        return ACCENTS[c]
    r, g, b = hexrgb(c)
    if max(r, g, b) - min(r, g, b) > 24:
        sys.exit(f"unmapped color {c}: add it to ACCENTS")
    # A grey on the dark window, as white at the same contrast over black.
    alpha = max(0.0, ((r + g + b) / 3 - WINDOW_GREY) / (255 - WINDOW_GREY))
    return grey(alpha * 255)


COLOR = re.compile(r"#[0-9a-fA-F]{6}\b|#[0-9a-fA-F]{3}\b")


def recolor(text, target=None):
    """Remap every color in text. With a target, greys that don't end up
    black or white become the target instead (accents keep their mapping)."""
    def one(m):
        new = remap(m.group(0))
        if target and m.group(0).lower() not in ACCENTS and new not in ("#000000", "#ffffff"):
            return target
        return new
    return COLOR.sub(one, text)


def element_spans(svg):
    """(start, end, id) of every element with an id, outermost first."""
    spans = []
    for m in re.finditer(r'<([a-zA-Z:]+)\b[^>]*?\bid="([^"]+)"[^>]*?(/?)>', svg):
        tag, eid, selfclosing = m.group(1), m.group(2), m.group(3)
        if selfclosing:
            spans.append((m.start(), m.end(), eid))
            continue
        # Find the matching close tag, counting nested tags of the same name.
        depth, pos = 1, m.end()
        pattern = re.compile(r"<(/?)%s\b[^>]*?(/?)>" % re.escape(tag))
        while depth:
            n = pattern.search(svg, pos)
            if n.group(1):
                depth -= 1
            elif not n.group(2):
                depth += 1
            pos = n.end()
        spans.append((m.start(), pos, eid))
    return spans


def frame(prefix, size, fill, opacity):
    """Frame parts for a Kvantum element: fill, rounded, hairline outer edge."""
    s, e = size, HAIRLINE
    half = 0.5
    body = f'fill:{fill};fill-opacity:{opacity}'
    line = f'fill:#ffffff;fill-opacity:{e}'
    arc = f'fill:none;stroke:#ffffff;stroke-opacity:{e};stroke-width:1'
    x0 = 5000  # out of the way of the upstream drawing
    parts = []

    def g(name, x, y, inner):
        parts.append(f'<g id="{prefix}{name}" transform="translate({x0 + x},{y})">{inner}</g>')

    g("", 0, 0, f'<rect width="10" height="10" style="{body}"/>')
    g("-top", 20, 0, f'<rect width="10" height="{s}" style="{body}"/><rect width="10" height="1" style="{line}"/>')
    g("-bottom", 40, 0, f'<rect width="10" height="{s}" style="{body}"/><rect y="{s - 1}" width="10" height="1" style="{line}"/>')
    g("-left", 60, 0, f'<rect width="{s}" height="10" style="{body}"/><rect width="1" height="10" style="{line}"/>')
    g("-right", 80, 0, f'<rect width="{s}" height="10" style="{body}"/><rect x="{s - 1}" width="1" height="10" style="{line}"/>')
    r = s - half
    # Each corner: a quarter disc of the fill, and the hairline arc along it.
    corners = {
        "-topleft": (f"M {s},0 A {s},{s} 0 0 0 0,{s} L {s},{s} Z", f"M {s},{half} A {r},{r} 0 0 0 {half},{s}"),
        "-topright": (f"M 0,0 A {s},{s} 0 0 1 {s},{s} L 0,{s} Z", f"M 0,{half} A {r},{r} 0 0 1 {r},{s}"),
        "-bottomleft": (f"M 0,0 A {s},{s} 0 0 0 {s},{s} L {s},0 Z", f"M {half},0 A {r},{r} 0 0 0 {s},{r}"),
        "-bottomright": (f"M {s},0 A {s},{s} 0 0 1 0,{s} L 0,0 Z", f"M {r},0 A {r},{r} 0 0 1 0,{r}"),
    }
    for i, (name, (disc, edge)) in enumerate(corners.items()):
        g(name, 100 + 20 * i, 0, f'<path d="{disc}" style="{body}"/><path d="{edge}" style="{arc}"/>')
    return "\n".join(parts)


def build_svg():
    svg = fetch("src/KvLibadwaita/KvLibadwaitaDark.svg")
    spans = element_spans(svg)

    # Drop the frames we redraw.
    drop = [(a, b) for a, b, eid in spans
            if any(eid == p or re.fullmatch(re.escape(p) + r"-(top|bottom|left|right)(left|right)?", eid) for p in FRAMES)]
    for a, b in sorted(drop, reverse=True):
        svg = svg[:a] + svg[b:]
    spans = element_spans(svg)

    # Recolor: grouped elements to their GTK-matching grey, the rest by rule.
    marks = []
    for a, b, eid in spans:
        for group, target in GROUPS.items():
            if eid == group or eid.startswith(group + "-"):
                marks.append((a, b, target))
    out, pos = [], 0
    for a, b, target in sorted(marks):
        if a < pos:
            continue  # nested inside a group already done
        out.append(recolor(svg[pos:a]))
        out.append(recolor(svg[a:b], target))
        pos = b
    out.append(recolor(svg[pos:]))
    svg = "".join(out)

    frames = "\n".join(frame(p, *spec) for p, spec in FRAMES.items())
    svg = svg.replace("</svg>", frames + "\n</svg>")
    svg = svg.replace('sodipodi:docname="KvLibadwaitaDark.svg"', 'sodipodi:docname="vetro.svg"')
    return svg


def build_kvconfig():
    kv = fetch("src/KvLibadwaita/KvLibadwaitaDark.kvconfig")
    kv = re.sub(r"\[GeneralColors\]\n(?:[^\[\n][^\n]*\n|\n)*", GENERAL_COLORS + "\n", kv, count=1)
    kv = re.sub(r"^author=.*$", "author=Gabriel Perko-Engel (KvLibadwaita), recolored for Vetro", kv, flags=re.M)
    kv = re.sub(r"^comment=.*$", "comment=KvLibadwaita in pure black and white", kv, flags=re.M)
    # Shadows don't show on black; the hairline does their job.
    kv = re.sub(r"^menu_shadow_depth=.*$", "menu_shadow_depth=0", kv, flags=re.M)
    kv = re.sub(r"^tooltip_shadow_depth=.*$", "tooltip_shadow_depth=0", kv, flags=re.M)
    kv = re.sub(r"(\[ToolTip\][^\[]*?)frame\.expansion=\d+", r"\1frame.expansion=0", kv)
    # Text: dim greys to white, white stays white.
    kv = re.sub(r"^(text\.[a-z]+\.color)=(#dfdfdf|#dedede|white)$", r"\1=#ffffff", kv, flags=re.M)
    kv = re.sub(r"^(text\.normal\.color)=#787878$", r"\1=#8c8c8c", kv, flags=re.M)
    left = [line for line in kv.splitlines() if re.search(r"color=#(?!ffffff|000000|0d0d0d|1a1a1a|262626|333333|666666|8c8c8c)", line)]
    if left:
        sys.exit("unmapped kvconfig colors:\n" + "\n".join(left))
    return kv


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "vetro.svg").write_text(build_svg())
    (OUT / "vetro.kvconfig").write_text(build_kvconfig())
    (OUT / "COPYING").write_text(fetch("LICENSE"))
    (OUT / "README").write_text(
        "Vetro's Kvantum theme: KvLibadwaita by Gabriel Perko-Engel\n"
        f"(https://github.com/GabePoel/KvLibadwaita, commit {COMMIT[:12]}),\n"
        "itself based on Colloid-kde by Vince Liuice, recolored pure black and\n"
        "white by override/kvantum/make-theme.py. GPL-3.0, see COPYING.\n")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
