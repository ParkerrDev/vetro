# Vetro identity

A V of clear glass lying on a white tile, lit from the upper left. Through the
glass the faint rings on the tile bend and swell, as content does through
Vetro's own glass; the rim catches the light, and the shadow is dark where
the edges turn the light away and light where the middle lets it through.
The palette is white, with black type.

- `render.py` makes the artwork: it works out each pixel of the glass the way
  light would (refraction, reflection, the glint and the shadow) and writes the
  three files below. It needs numpy, Pillow and fontTools.
- `vetro.png` is the mark, 512 pixels square.
- `vetro.svg` is the same picture as the app's scalable icon.
- `vetro-symbolic.svg` is the simplified silhouette, two round-ended strokes,
  for small, single-colour controls. GTK recolours it to match the foreground.
- `readme-banner.svg` combines the mark with the wordmark, set in the
  project's D-DIN Expanded, and the slogan, "Liquid glass for Omarchy", in
  D-DIN. Its lettering is outlined, so it needs no installed fonts.
- `preview.webp` is the README's moving picture: 17 seconds of the video
  (from 9.6 s), 1280 by 720 at 30 frames a second, made with
  `ffmpeg -ss 9.6 -t 17.4 -i vetro.mp4 -vf "fps=30,scale=1280:720:flags=lanczos"
  -c:v libwebp_anim -quality 70 -compression_level 6 -loop 0 preview.webp`.
  The whole video is attached to each release.

Keep the tile's clear margin when placing the logo. Use the symbolic version
for small controls, and the complete tile for the app launcher and larger uses.

To make the artwork again after changing `render.py`, run from the repository
root:

```sh
python3 assets/branding/render.py
```

The app bundles the SVGs and its CSS as GResources. `make -C apps/glass` rebuilds
them when the source changes; `vetro tune` checks those dependencies too. Both
installers put the icons in the user's `hicolor` theme as `dev.vetro.Glass`.
