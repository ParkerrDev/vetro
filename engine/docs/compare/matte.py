#!/usr/bin/env python3
"""matte.py BLACK.png WHITE.png OUT.png

The layer's own pixels, recovered from two photographs of the same scene with
the glass off, one over black and one over white: over black a pixel shows
a * F, over white a * F + (1 - a), so the alpha is 1 - (white - black) and the
photograph over black is the premultiplied colour. The reference page lays
this over its own glass, so that text and fills are the layer's real ones and
only the glass differs between the two pictures.
"""
import sys

import numpy as np
from PIL import Image


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__)
        return 2
    black = np.asarray(Image.open(sys.argv[1]).convert("RGB"), dtype=np.float64) / 255.0
    white = np.asarray(Image.open(sys.argv[2]).convert("RGB"), dtype=np.float64) / 255.0
    if black.shape != white.shape:
        print("the two photographs differ in size")
        return 1
    alpha = np.clip(1.0 - (white - black).mean(axis=2), 0.0, 1.0)
    # premultiplied colour stored as it is: the page is told not to multiply again
    out = np.dstack([np.clip(black, 0.0, 1.0), alpha])
    Image.fromarray(np.rint(out * 255.0).astype(np.uint8), "RGBA").save(sys.argv[3])
    return 0


if __name__ == "__main__":
    sys.exit(main())
