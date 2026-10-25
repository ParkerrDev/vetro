# The glass model

This branch (`meniscus`) draws layer surfaces with a model of Apple's Liquid
Glass instead of the stock shader's free parameters. This file says what the
model is, where every number in the presets comes from, how the plugin
computes it, how close it comes to an independent reference, and what it
costs. Lengths are in points (logical pixels) unless device pixels are named;
the plugin multiplies them by the monitor's scale.

## The model in short

1. A pane is a slab of glass with a rounded rim: flat face, and a bezel along
   the outline where the surface curves down to the page.
2. The rim refracts: a ray that reaches the eye through the bezel left the
   page further inside the pane, by an amount Snell's law gives from the
   surface's slope. Red and blue bend a little less and more (dispersion).
3. The glass is frosted: what lies behind is blurred by a small radius, then
   toned by the face (more saturation, a tint that depends on how bright the
   page is).
4. A light from one side draws a thin line on the rim walls that face it and
   on the opposite walls, with a faint glow and a faint inner shadow.
5. The pane throws a soft shadow downward.

## Outline

The outline of a pane is taken from the layer's own alpha: every connected
part of the surface whose alpha reaches `mask_threshold` is one pane. A card,
a menu, a capsule or a popup inside one large transparent surface therefore
each has its own rim, with rounded or squircle corners exactly as the client
drew them, and no shape has to be declared.

Rule for the threshold: above the alpha of anything the layer draws that is
not pane (a dim over the whole screen, its own soft shadow), and at or below
the alpha of the pane's fill as it ends up in the surface. Midway is the safe
choice. For a card of alpha `c` on a dim of alpha `d` the surface holds
`1 - (1 - d)(1 - c)` under the card and `d` beside it: with `d = 0.4` and
`c = 0.6` that is 0.76 against 0.4, so 0.58. For a card of alpha 0.55 on a
transparent layer, 0.27. Keep at least 0.05 between the threshold and each
side.

Distance to the outline comes from blurring the coverage (1 inside, 0 outside)
with a Gaussian of deviation `s = bezel / 3`: near a straight edge the blurred
coverage is `v = Phi(d / s)`, with `Phi` the normal distribution function and
`d` the depth inside the outline, so `d = s * quantile(v)`. The gradient of
`v` is the inward normal. The plugin stores `v` and the normal in a field
texture, built once per change of the outline.

## Lens

With `t` the depth in bezel widths (0 on the outline, 1 where the bezel ends),
`B` the bezel width, `T` the slab's thickness, `n` the refractive index and
`p` the profile's exponent:

    h(t)  = (1 - (1 - t)^p)^(1/p)            height of the surface, in units of B
    a1(t) = atan(h'(t))                      angle of incidence
    a2(t) = asin(sin(a1) / n)                Snell's law
    D(t)  = refraction * (B h(t) + T) * tan(a1 - a2)

`D` is how far inside the pane the page is read. On the outline the surface
is vertical and `tan(a1 - a2) = sqrt(n^2 - 1)`; at `t = 1` it is flat and
`D = 0`. Dispersion `k` gives red and blue the indices
`n -/+ k (n - 1) / 2`.

A pane too small for the whole lens gets a smaller one: the bezel at most a
quarter of the pane's short side, and the bend on the outline at most to the
pane's centre line (`T <= shortSide / (2 refraction sqrt(n^2 - 1))`).

## Frost and face

The page behind a pane is blurred with a Gaussian of deviation `frost`, then

    L   = luminance(c)
    c   = mix(L, c, saturation)
    c   = (c - 0.5) * contrast + 0.5
    c   = c * brightness
    c   = mix(c, tint, mix(tint_over_dark, tint_over_bright, L))

## Light and shadow

With `N` the outward normal of the outline and `Ld` the direction to the
light (`light_angle`, 0 to the right, 90 down, so 315 is the upper left):

    f      = dot(N, Ld)
    lit    = f^2 * (1 if f > 0 else counter_light)
    edge   = clamp(2 (1 - v), 0, 1)
    colour = colour * (1 - inner_shadow * edge^3)
           + light_intensity * lit * (line + rim_glow * edge^3)

`line` is 1 within `rim_width` of the outline and 0 further in. The shadow is
the pane's coverage blurred by `shadow_radius / 2`, moved down by
`shadow_offset` and drawn in black at `shadow_opacity`.

## The numbers and where they come from

No source publishes Apple's own constants. The numbers below are measurements
and reconstructions by others, read at first hand; where two disagree, the
one measured on a device was taken.

| Quantity | Value in `regular` | Source |
|---|---|---|
| Refraction curve | `B = 20`, `T = 53`, `n = 1.5`, `p = 2.6`: within 1.5 points of `60 (1 - sqrt(1 - x^2))` over 20 points, `x = 1 - t` | flutter_liquid_glass, measured on iOS: refraction height 20, amount 60, quarter-circle falloff. ShatteredGlass reads height 12.5 and amount 40 for the inner zone of macOS 26's SwiftUI glass. |
| Bezel against size | at most a quarter of the short side; the rim reads no deeper than the centre line | flutter_liquid_glass, measured. No source ties the bezel to the corner radius: a corner tighter than the bezel only rounds the field. |
| Frost (blur) | deviation 6 points at 1x, times the scale (9.6 device pixels at 1.6) | ShatteredGlass: backdrop at scale 0.25 with `inputBlurRadius` 1.5, which is 6 points. flutter_liquid_glass: single blur equivalent 2, 6.1 and 16.6 points at slider 0, 0.5 and 1. A third-party reading of Apple's Figma kit gives 14, 12 and 7 by size. |
| Saturation | 2.5 dark, 1.95 light | CodenameOne `GlassRecipe.java`, measured on iOS 26 (`UIGlassEffect` panel). |
| Tint | dark `#252525` at 0.76, light `#fafafa` at 0.70, the same over dark and bright pages | The same measurement: output = input * 0.238 + 28.4 (dark) and * 0.303 + 174.3 (light), which is a mix toward those colours by 1 minus the factor. The kit reading agrees: `#fafafa` at 0.7. |
| Light | from the upper left (315), both walls along its axis lit (`counter_light` 1), `light_intensity` 0.5, `rim_width` 1 | ShatteredGlass: two highlight effects at pi/4 and -3pi/4, amount 0.5, height 1. flutter_liquid_glass: "1.2 pt glint on both walls along the light axis". Figma's kit: light angle -45. |
| Shadow | opacity 0.12, blur 40, offset 8 downward | ShatteredGlass: offset (0, 8), radius 40, SDR opacity 0.12. |
| Clear over bright | tint 0.35 of black over a bright page, 0.18 over a dark one; frost 2; saturation 1.2; dispersion 0.1 | Apple's guidelines: clear needs "a dark dimming layer of 35% opacity" over bright media. The rest is this branch's choice. |

`rim_glow` 0.1 and `inner_shadow` 0.04 are fitted by eye to Apple's published
pictures and have no measured source.

Figma's Glass effect names the same things: Light angle and intensity
(`light_angle`, `light_intensity`), Refraction (`refraction`), Depth
(`bezel_width` and `glass_thickness`), Dispersion (`dispersion`), Frost
(`frost`). Splay has no counterpart here.

## How the plugin computes it

The arithmetic of every step below, and every decision about how the passes are
laid out, is the Rust crate `crates/glass-core` (`lens.rs`, `panes.rs`,
`schedule.rs`, `options/`); the plugin's C++ asks it and draws what it says.

For a layer whose preset sets `model = 1` the layer is no longer redirected
into a buffer. Its glass is drawn under it, and the layer's committed buffer
is read as the mask.

1. Field, once per change of the outline: coverage from the alpha, reduced by
   1, 2 or 4, blurred, differentiated. A commit that does not move the outline
   is found by an occlusion query and costs no rebuild.
2. Pane map: the field in cells of 8 pixels is read back without waiting, and
   connected cells become panes, each with its box, short side and the largest
   rectangle that is face only.
3. Frost per pane, only when something behind that pane changed: one reduction
   by 1, 2, 4 or 8 and a separable Gaussian whose last pass applies the face.
4. Drawing, cut to the frame's damage: the shadow, then the face rectangles
   with one texture read and no blending, then the rest with the lens, the
   rim and the edge.

Edge. Within a few pixels of the outline the share of a pixel that is pane is
`(own - beside) / (inside - beside)`: the layer's alpha at the pixel, the
alpha two pixels further in, and the alpha beyond the rim line when that is
under the threshold. This is the coverage the client's own antialiasing
gave the pixel, whatever the fill's alpha, at any fractional scale, and for
any shape: the glass, the rim line and the client's edge end on the same
pixels with the same softness. Nothing is drawn where the layer draws only a
dim or a shadow.

Windows, subsurface items, rotated outputs and layers masked by a blur region
still go through upstream's shader.

## Agreement with the reference

`docs/compare/reference.html` is the same model written independently: exact
shapes, a full-resolution Gaussian, the lens computed per pixel. It is
rendered by headless Chromium over the same background and shapes, and
`compare.py` measures the difference in 8-bit levels by region.

| Scene (2560x1600 at 1.6) | Picture | Face | Bezel | Outline, 2 px | Shadow |
|---|---|---|---|---|---|
| regular dark, text on the panes | `regular-dark-text.jpg` | 0.26 | 1.03 | 6.0 | 0.08 |
| regular light, text | `regular-light-text.jpg` | 0.21 | 0.95 | 0.84 | 0.25 |
| regular dark over a busy colourful page | `regular-dark-busy.jpg` | 0.25 | 1.33 | 8.9 | 0.17 |
| regular light over the busy page | `regular-light-busy.jpg` | 0.22 | 1.27 | 3.9 | 0.17 |
| clear over a grid | `clear-grid.jpg` | 0.28 | 1.92 | 6.2 | 0.10 |
| cards on a dim, busy page | `regular-dark-dimmed.jpg` | 0.06 | 0.28 | 1.8 | 0.16 |
| corners on mid grey, dark | `corners-dark.png` | 0.00 | 0.15 | 8.2 | 0.29 |
| corners on mid grey, light | `corners-light.png` | 0.00 | 0.16 | 5.8 | 0.29 |

Mean absolute difference. Straight edges are identical; what remains sits in
the outermost 2 to 3 pixels of corners, where the lens compresses the page
about 13 to 1 and the plugin's depth comes from a field at reduced resolution.
`results.txt` has the percentiles.

The corner pictures enlarge corners eight times, nearest neighbour, plugin on
the left of each pair: the bar's pill, a card that is its own layer at a
position that is not a whole device pixel, the same with the 1 pixel outline
a client may draw, a card and an outlined card inside a full-screen layer,
and in the large pictures a squircle and an outlined capsule. The rim is one
continuous line through the arc, and it does not double against the client's
outline, because both are cut by the same alpha.

## What refreshes the glass

A pane is sampled again when a surface behind it commits new pixels (kept, not
dropped, when it comes sooner than `layers:live_resample_fps` allows), when a
layer or a window opens or closes, when the layer moves, and during workspace
animations. A window that moves under a pane without committing, such as a
floating window dragged under a bar, is not seen until it commits; upstream
has the same limit.

## Cost

Measured in a nested compositor on the Intel integrated GPU, 2560x1600 at
scale 1.6, as GPU time of the whole compositor from the kernel's accounting.
The nested compositor gets about 17 frames a second; GPU frequency scaling
moves single results by about a quarter of a millisecond.

Scene: a bar 1590x30, two cards 384x274 and 318x238, a notification 384x92
and a menu 560x470 inside a full-screen layer, over a wallpaper that changes
every frame.

| | GPU time per frame over no glass |
|---|---|
| upstream v0.10.0, its clear preset | 6.1 to 10.3 ms |
| this branch, `regular` | 0.86 to 1.06 ms |

With nothing changing behind the glass no frame is drawn and nothing is
sampled. A counter redrawn five times a second inside a pane of a full-screen
layer costs about 1.6 ms per tick.

## Not modelled

Apple's blur that grows toward the rim, the outer refraction zone, the flip
of small elements between light and dark, a shadow that adapts to the
content, highlights that react to motion, and frost and opacity that depend
on the pane's size.

## Sources

- Apple, Human Interface Guidelines, Materials: https://developer.apple.com/design/human-interface-guidelines/materials
- Apple, WWDC25 session 219, Meet Liquid Glass: https://developer.apple.com/videos/play/wwdc2025/219/
- Figma, Apply effects to layers (Glass): https://help.figma.com/hc/en-us/articles/360041488473-Apply-effects-to-layers
- Figma, plugin API, Effect: https://developers.figma.com/docs/plugins/api/Effect/
- kube.io, Liquid Glass in the browser, and its code: https://kube.io/blog/liquid-glass-css-svg/ and https://github.com/kube/kube.io
- ShatteredGlass (the layer tree of macOS 26's glass): https://github.com/AlexStrNik/ShatteredGlass
- CodenameOne, `GlassRecipe.java` (measured on iOS 26): https://github.com/codenameone/CodenameOne
- flutter_liquid_glass (measured on iOS): https://github.com/whynotmake-it/flutter_liquid_glass
- ios26-design-system, `materials.json` (a reading of Apple's Figma kit, approximate by its own account): https://github.com/seunghan91/ios26-design-system
- AndroidLiquidGlass: https://github.com/Kyant0/AndroidLiquidGlass
- liquid-glass-studio: https://github.com/iyinchao/liquid-glass-studio
- liquid-glass-react: https://github.com/rdev/liquid-glass-react
- liquidGL: https://github.com/naughtyduk/liquidGL
