#!/usr/bin/env python3
"""corners.py PLUGIN.png REFERENCE.png OUT.png X,Y[,SIZE] ...

Enlarges small squares of the plugin's picture and of the reference's eight
times, nearest neighbour, and sets them side by side: the plugin's on the left
of each pair. X,Y is the top left of a square of SIZE device pixels (40 when
not given). At this size every pixel of a corner's rim can be counted.
"""
import sys

from PIL import Image

ZOOM = 8


def main() -> int:
    if len(sys.argv) < 5:
        print(__doc__)
        return 2
    plugin = Image.open(sys.argv[1]).convert("RGB")
    reference = Image.open(sys.argv[2]).convert("RGB")
    squares = []
    for spec in sys.argv[4:]:
        parts = [int(v) for v in spec.split(",")]
        squares.append((parts[0], parts[1], parts[2] if len(parts) > 2 else 40))
    gap = 8
    width = sum(2 * s * ZOOM + gap for _, _, s in squares) + gap * (len(squares) - 1)
    height = max(s for _, _, s in squares) * ZOOM
    out = Image.new("RGB", (width, height), (255, 255, 255))
    x = 0
    for left, top, size in squares:
        for image in (plugin, reference):
            crop = image.crop((left, top, left + size, top + size)).resize((size * ZOOM, size * ZOOM), Image.NEAREST)
            out.paste(crop, (x, 0))
            x += size * ZOOM + (gap if image is plugin else 0)
        x += gap
    out.save(sys.argv[3])
    return 0


if __name__ == "__main__":
    sys.exit(main())
