## Motion off as it comes

- `genie_close`, `jelly` and `wobble` are off until they are switched on (Vetro Glass, or `glass.conf`): on by default they made the desktop hard to use. The pace and the springs stay.

## Windows that move as Apple's do

- The engine's own window animations (`glass_core::motion`, `src/Motion.cpp`), run while `plugin:hyprglass:motion:enabled` is on, as Vetro's look has it:
  - the genie: a closing window pours itself into the bottom of the screen, or toward the pointer, glass, border and all, as it last looked. The engine keeps a copy of the focused window as it is drawn, taken again only where it changed, because some clients (foot) let go of their buffer before they close and Hyprland's snapshot of them is empty. A new window can pour out the same way (off as it comes);
  - the jelly: a new window lands with a squash whose sides bow in, swings a little past and settles;
  - the wobble: a window dragged bends behind the pointer and jiggles as it settles; windows the layout moves can wobble too (off as it comes);
  - the pace: every animation the config set, and every spring, at one quickness from 0.5 to 2. Vetro's shell moves its panels at it as well.
- A group of knobs of its own, Motion, in `glass.conf` and `vetro tune`: `pace`, `genie_close`, `genie_open`, `genie_pointer`, `genie_time`, `jelly`, `jelly_squash`, `wobble`, `wobble_moves`, `wobble_time`, `wobble_bounce`. A preset of the glass leaves them as they are.
- `hyprctl hyprglass motion`: what runs, and how often each part has.
- The C interface is at version 9 (`glass_genie_mesh`, `glass_wobble_hold`, `_moved`, `_squeeze`, `_step`, `_mesh`, `GlassWobble`).
- Fixed: the glass was drawn into the snapshot Hyprland takes of a closing window, where it came out black in the window's fadeout. A snapshot has no glass now.
- Fixed: Hyprland crashed when the engine was unloaded in the frame after one of its motion elements was drawn; they are taken out of the frame at unload.
- Fixed: the screen of glass behind the menu went clear for a few frames as the menu opened. A large layer's first map of its panes could be read before the layer had drawn, find nothing, and take away the glass that stood; such a map now leaves it standing until the next one.

## Body bend, and the Author's pick

- `body_bend` carries the lens's push into the middle of a large window, which was blur inside a warped rim (C interface 8).
- The preset `authors-pick`, the glass as it comes: a little darker and less frosted than iOS 27, with no white veil.
- The tag `hyprglass_solid` gives a window its outline of glass alone and says its content covers what is under it, so nothing is drawn under browsers and Electron apps.
- The lens field of a full-screen layer, as Omarchy's bar panels are, is built only around the outline that moved and the panes it knows (`changes.frag` maps where it moved), every pass scissored and the scissor put back.

## Glass the user tunes, and windows through windows

- `glass_core::tuning`: twenty knobs (clear to tinted, frost, colour, inner light, darker over bright, the tint colour, Apple's colour, the lens, its bend and fringe, the dark edge, where the light comes from, the rim light, side light, hairline and glow, and whether windows show through), seven presets, and the `key = value` file `~/.config/vetro/glass.conf`. The preset `tuned` (`quartz::Kind::Tuned`, material 8) is iOS 27's glass until the user tunes it: lit from straight above, the darkened edge macOS 27 draws (a point outside, darker on the sides), no shadow, and the bright layer's matrix over the whole face as light of its own over a dark backdrop. The block of Apple's shader grows to 116 floats for the light's direction, the side light and that inner light (C interface 7).
- `hyprctl hyprglass tune [reload | key=value ...]`; the file is read at load and on every config reload.
- X-ray glass shows the windows under it and never the desktop's widgets: the windows of the frame drawn so far are kept (`Beneath`), and glass over one of them reads a clean copy of the frame (`quartzclean.frag`), the backdrop wherever none of them lies, blurred for the face as the backdrop is. Glass over no window reads the backdrop alone, as before. A commit of a window above, or of the widgets, no longer marks x-ray glass stale.
- Fixed: a resolve context built from a temporary preset name read freed memory (it keeps a reference to the name).

## The lens, and a blur without blocks

- The preset `lens`: Apple's dark colour and bright layer with the edge of iOS 26's glass as Kyant0's AndroidLiquidGlass and kube.io's bezel draw it, a quarter-circle bezel 28 points deep that bends what is behind by up to 28 points at the edge (kept within the pane's own size on a thin pane), a 7-point blur in place of the 45-point frost, no bleed band and no blur fill, a dark tint that goes further over a bright backdrop so that text on the glass reads, the colours behind saturated 1.3 times (1.7 at first; the calibrated iOS 26 and 27 materials saturate 1.4 to 1.5), a soft light inside the lit edges, and a hairline rim lit by how squarely the edge faces a light from the top left.
- X-ray glass reads a blurred copy of the backdrop made with true Gaussian passes at a half, a quarter or an eighth of its size, once per change of the wallpaper or the radius, with one bilinear tap: a mip level read instead showed its blocks, worst at a light blur. Glass read from a mip chain takes four reads a level finer on a rotated grid.
- The C interface is at version 6 (`GlassQuartz.blur`; the block is 108 floats, with the hairline, the tint, the saturation and the glow).

## Apple's own glass shader

- New presets that draw a pane with Apple's own glass shader as transcribed from macOS 27's `QuartzCore` (`glass_background_all_lpf`, by liquid-glass-react-native; see NOTICE): its 63 uniforms in Apple's order and its stages in Apple's order, in one pass over the pane. The page behind the pane is copied with a mip chain, and the shader blurs by reading a level of it, as macOS does.
  - `apple`: the numbers that project fitted against a real `NSGlassEffectView`.
  - `apple-recipe`: Apple's own `platformContentGlass` recipe (blur 45 and its colour matrix), with the dark appearance's measured bias.
  - `apple-clear`: the clear style, its matrix solved against macOS 27.
  - `apple27`: the recipe with macOS 27's light as gpui-glass-demo decoded it from `QuartzCore`: a dark rim a point outside the face, a bright layer of a sharp and a soft lobe keyed from above and filled from below through `vibrantColorMatrix`, and the blur fill that darkens.
- The shape is whatever the layer drew: a distance field of its outline, one texel a pixel, made by a jump flood when the outline changes, stands in for the SDF texture Apple makes of a view's shape.
- The C interface is at version 5 (`GlassParams.quartz`, `glass_quartz`, `GlassQuartz`).

## Nothing brighter behind comes out darker

- The classic shader's `adaptive_dim` made its curve fall as it went once it was above about 0.36: at 0.9, a quarter grey stayed at 0.25 and white went to 0.10, so white letters behind a window, blurred, came out dark inside a light rim. The core now finds where the curve of the dim, the boost and the brightness first stops rising (`tone::peak`), and the shader holds whatever is brighter behind the glass at that height, keeping its colour. Below that point, and for any gentler dim, every pixel is as it was.
- The C interface is at version 4 (`glass_tone_peak`, `GlassTonePeak`).


## Apple's current material

- The preset `regular27`: the material of the current system, where `regular` is the first generation. A frost of two copies of the page (a nearly sharp one under a wide soft one), a far lighter tone with a shade over bright content, a rim of a dark hairline and two lobes of light, and the thin-prism lens.
- New options: `material`, `frost_wide`, `frost_cloud`, `body_shade`, `rim_light`, `rim_shade`.
- The numbers were already in the core as data (`material.rs`); they now reach the passes. With none of the new options set, every pixel is as it was.
- The C interface is at version 3 (`GlassLayered`, `GlassLens.paraxial`).
- Windows take it too, through the shader that draws a window's pane: a preset with `frost_wide` set gives a window the page as it is (at half resolution) under a cloud made at an eighth, the same face and the same rim, and its frame the hairline and the light. It costs less than the blur it replaces (60 against 73 ms of GPU a second for two windows over a playing video on an integrated card). `glass_rim_constants` hands that shader the rim's measured numbers.

## Windows with a border of glass

- `windows:border`: a window's pane reaches out over the band Hyprland keeps for its border, and a solid window gets that band alone as a frame. The frame's four strips are worked out in the model (`geometry::frame_strips`), sampled at a quarter of the resolution and drawn without touching the middle the window hides.
- `windows:border_dim`: the rim of a window without the focus is lit less.
- The C interface is at version 2 (`glass_frame_strips`, `GlassFrame`).

## [v0.10.0](https://github.com/Hyprnux/hyprglass/compare/v0.9.1...v0.10.0) - 2026-10-09

### Bug Fixes

* faint parts of bars and panels, like shadows, no longer disappear
* steady, even grain on every window and theme ([#67](https://github.com/Hyprnux/hyprglass/issues/67))
* window glass works even with shadows turned off
* faster Hyprland startup with hyprglass loaded ([#88](https://github.com/Hyprnux/hyprglass/issues/88))
* loading hyprglass twice no longer makes the two copies clash
* glass works on rotated and flipped screens

### CI/CD

* releases are only built against a released Hyprland version

### Documentation

* what to do when Hyprland hangs at startup ([#88](https://github.com/Hyprnux/hyprglass/issues/88))

### Features

* set the layer mask threshold once for all layers ([#52](https://github.com/Hyprnux/hyprglass/issues/52))
* optional grain on the glass for a frosted look ([#67](https://github.com/Hyprnux/hyprglass/issues/67))
* soft glass edges can be set per layer ([#58](https://github.com/Hyprnux/hyprglass/issues/58))
* glass follows rounded shapes requested by apps more closely
* layer glass follows the soft edges of rounded panels ([#58](https://github.com/Hyprnux/hyprglass/issues/58))
* x-ray can be set per preset and hides every window behind the glass ([#68](https://github.com/Hyprnux/hyprglass/issues/68))
* add xray option to hide windows under the glass ([#68](https://github.com/Hyprnux/hyprglass/issues/68))
* hyprglass pauses instead of crashing on a Hyprland it was not built for


## [v0.9.1](https://github.com/Hyprnux/hyprglass/compare/v0.9.0...v0.9.1) - 2026-10-03

### Bug Fixes

* hyprglass loads on glibc 2.41+ instead of failing with "cannot enable executable stack" ([#86](https://github.com/Hyprnux/hyprglass/issues/86))
* hyprctl hyprglass status tells whether glass is actually drawn, not just loaded
* hyprctl plugin list shows the installed hyprglass version instead of 1.0.0

### Chores

* **release:** v0.9.1 [skip ci]

### Documentation

* give option ranges as typical values and describe vibrancy_darkness as the shader applies it


## [v0.9.0](https://github.com/Hyprnux/hyprglass/compare/v0.8.1...v0.9.0) - 2026-10-01

### Bug Fixes

* no compositor crash when an app closes a glass item after hyprglass was unloaded or reloaded
* remove the corner seam in the edge bevel and follow capsule and squircle outlines
* few artifacts speckles in the glass and fix glass when near monitor borders
* glass bounding box in logical coordinates so damage and culling match on scaled monitors
* glass darkened and rendered twice on floating windows over fullscreen windows
* glass artifacts + useless re-renders for overview plugins ([#69](https://github.com/Hyprnux/hyprglass/issues/69))
* replace damageSurface hook in favor of a commit listener per surface ([#78](https://github.com/Hyprnux/hyprglass/issues/78))

### CI/CD

* release from hyprland-X.Y branches, merge them into main and build main against hyprland-git

### Chores

* drop blur margin warning (not always right, and hard to maintain)
* **hyprpm:** install v0.8.1 on Hyprland 0.56.1/0.56.2 and v0.6.3 on 0.55.3
* **release:** v0.9.0 [skip ci]

### Documentation

* explain stable and hyprland-git installs and which branch to target

### Features

* hyprctl hyprglass items lists subsurface glass items with their resolved preset and shape
* apps can set a per-surface glass preset and shape (hyprglass_item_v1 protocol)
* glass behind window subsurfaces that set a background-effect region ([#61](https://github.com/Hyprnux/hyprglass/issues/61))
* hyprctl hyprglass stats, debug mode and GPU stage timers for performance tuning
* self-sampling ourself
* add refraction and bevel settings + add pomme (apple style)  preset

### Performance Improvements

* fewer blur passes when the same blur is reachable with less work (blur_fold, default on)
* cheaper glass shader math for standard rounded corners, same look
* sample and clear only what a protocol-masked layer can show as glass
* reuse the blurred background of a window until its backdrop changes (windows:background_cache)
* warn at config reload when Hyprland blur size/passes leave too little damage margin for glass
* skip the glass effect under fully opaque windows (skip_opaque_windows, default on)
* let Hyprland skip glass elements outside the damaged area on windows and layers


## [v0.8.1](https://github.com/Hyprnux/hyprglass/compare/v0.8.0...v0.8.1) - 2026-09-03

### Bug Fixes

* issue provoking black artifacts in some monitors configurations ([#56](https://github.com/Hyprnux/hyprglass/issues/56))

### Chores

* **release:** v0.8.1 [skip ci]


## [v0.8.0](https://github.com/Hyprnux/hyprglass/compare/v0.7.0...v0.8.0) - 2026-09-03

### Bug Fixes

* validate hg.config keys and report errors at the caller ([#71](https://github.com/Hyprnux/hyprglass/issues/71))
* keep glass on internal-fullscreen windows, recover on fullscreen exit ([#54](https://github.com/Hyprnux/hyprglass/issues/54))
* size layer FBOs from the framebuffer, not the monitor transform
* own event listeners in plugin state so unload unregisters them

### Build System

* rebuild objects when headers change
* update hyprland compatibility to v0.56.2
* update hyprland compatibility to v0.56.1

### Chores

* **release:** v0.8.0 [skip ci]

### Features

* add support for ext-background-effect-v1 (wayland) with alpha fallback ([#61](https://github.com/Hyprnux/hyprglass/issues/61))
* per-layer live_resample override for layer glass ([#59](https://github.com/Hyprnux/hyprglass/issues/59))
* re-rendering layer background when window content behind it change ([#59](https://github.com/Hyprnux/hyprglass/issues/59))

### Performance Improvements

* mark layer glass dirty from actual commit damage only ([#59](https://github.com/Hyprnux/hyprglass/issues/59))


## [v0.7.0](https://github.com/Hyprnux/hyprglass/compare/v0.6.4...v0.7.0) - 2026-07-20

### Bug Fixes

* don't dim glass by inactive alpha and opacity rules
* use source framebuffer format for layer temp FBO in HDR mode
* set noblur on glassed windows, glass replaces hyprland blur ([#46](https://github.com/Hyprnux/hyprglass/issues/46))
* use MONITORID instead of Monitor::CMonitor, unavailable before hyprland 0.56 ([#44](https://github.com/Hyprnux/hyprglass/issues/44))
* relax version check to compare ABI suffix only
* qualify CMonitor with Monitor:: namespace
* wrong viewport restore after blur on transformed monitors ([#41](https://github.com/Hyprnux/hyprglass/issues/41))
* match dynamic window tags for preset selection ([#45](https://github.com/Hyprnux/hyprglass/issues/45))

### Build System

* update hyprland compatibility to v0.56.0

### Chores

* **release:** v0.7.0 [skip ci]

### Documentation

* add troubleshooting section

### Features

* compatibility hyprland 0.56
* add HYPRGLASS_SKIP_VERSION_CHECK escape hatch ([#44](https://github.com/Hyprnux/hyprglass/issues/44))


## [v0.6.4](https://github.com/Hyprnux/hyprglass/compare/v0.6.3...v0.6.4) - 2026-06-12

### Build System

* update hyprland compatibility to v0.55.4

### Chores

* **release:** v0.6.4 [skip ci]


## [v0.6.3](https://github.com/Hyprnux/hyprglass/compare/v0.6.2...v0.6.3) - 2026-06-08

### Build System

* update hyprland compatibility to v0.55.3

### CI/CD

* **hyprpm:** correctly pin to hyprland versions ([#38](https://github.com/Hyprnux/hyprglass/issues/38))

### Chores

* **release:** v0.6.3 [skip ci]


## [v0.6.2](https://github.com/Hyprnux/hyprglass/compare/v0.6.1...v0.6.2) - 2026-05-21

### Bug Fixes

* keep layer mask threshold stable during fade

### Chores

* **release:** v0.6.2 [skip ci]


## [v0.6.1](https://github.com/Hyprnux/hyprglass/compare/v0.6.0...v0.6.1) - 2026-05-20

### Bug Fixes

* initialize presets before validation
* support legacy string config values

### Chores

* **release:** v0.6.1 [skip ci]


## [v0.6.0](https://github.com/Hyprnux/hyprglass/compare/v0.5.0...v0.6.0) - 2026-05-19

### Bug Fixes

* clean up decoration lifetime on unload
* stabilize Hyprland 0.55 plugin integration

### Build System

* update hyprland compatibility to v0.55.2

### Chores

* **release:** v0.6.0 [skip ci]

### Documentation

* add Lua config example
* **configuration:** reorganize docs + enhance lua/conf doc ([#16](https://github.com/Hyprnux/hyprglass/issues/16))

### Features

* **configuration:** improve lua configuration using lua functions ([#16](https://github.com/Hyprnux/hyprglass/issues/16))


## [v0.5.0](https://github.com/Hyprnux/hyprglass/compare/v0.4.1...v0.5.0) - 2026-05-06

### Bug Fixes

* fading workspace  animations were not fading the glass layer ([#24](https://github.com/Hyprnux/hyprglass/issues/24))

### Chores

* **release:** v0.5.0 [skip ci]

### Features

* add ability to enable/disable hyprglass effect per-window ([#23](https://github.com/Hyprnux/hyprglass/issues/23))


## [v0.4.1](https://github.com/Hyprnux/hyprglass/compare/v0.4.0...v0.4.1) - 2026-03-31

### Bug Fixes

* when workspace animation occurs and no windows were on workspace, effect was not redrawn (useful when background animation occur on workspace changes)

### Chores

* **release:** v0.4.1 [skip ci]


## [v0.4.0](https://github.com/Hyprnux/hyprglass/compare/v0.3.1...v0.4.0) - 2026-03-31

### Bug Fixes

* correct redraw artifact on multi-monitor setup
* some low opacity layers on XRGB monitors where shown without the effect

### Chores

* **release:** v0.4.0 [skip ci]

### Features

* **layers:** add namespace_mask_thresholds for better shadow handling + fix some config parsing issues


## [v0.3.1](https://github.com/Hyprnux/hyprglass/compare/v0.3.0...v0.3.1) - 2026-03-30

### Build System

* update hyprland compatibility to v0.54.3

### Chores

* **release:** v0.3.1 [skip ci]


## [v0.3.0](https://github.com/Hyprnux/hyprglass/compare/v0.2.7...v0.3.0) - 2026-03-25

### Chores

* **release:** v0.3.0 [skip ci]

### Features

* handle layer decoration - BETA ([#6](https://github.com/Hyprnux/hyprglass/issues/6))

### Performance Improvements

* conditionaly make half-res blur if blur_strenght is sufficient enough
* reduce GPU overhead for layer glass effect, still not perfect ([#6](https://github.com/Hyprnux/hyprglass/issues/6))


## [v0.2.7](https://github.com/Hyprnux/hyprglass/compare/v0.2.6...v0.2.7) - 2026-03-11

### Build System

* improved the makefile to allow for parallel builds + fix linker flag

### Chores

* **release:** v0.2.7 [skip ci]


## [v0.2.6](https://github.com/Hyprnux/hyprglass/compare/v0.2.5...v0.2.6) - 2026-03-11

### Build System

* update hyprland compatibility to v0.54.2

### Chores

* **release:** v0.2.6 [skip ci]


## [v0.2.5](https://github.com/Hyprnux/hyprglass/compare/v0.2.4...v0.2.5) - 2026-03-09

### Bug Fixes

* preset configuration was not correctly picked up ([#11](https://github.com/Hyprnux/hyprglass/issues/11))

### Chores

* **release:** v0.2.5 [skip ci]


## [v0.2.4](https://github.com/Hyprnux/hyprglass/compare/v0.2.3...v0.2.4) - 2026-03-04

### Build System

* update hyprland compatibility to v0.54.1

### Chores

* **release:** v0.2.4 [skip ci]


## [v0.2.3](https://github.com/Hyprnux/hyprglass/compare/v0.2.2...v0.2.3) - 2026-03-02

### Bug Fixes

* need to always resample to catch real time background change, cheap GPU overhead, but no way to do otherwise as of now

### Chores

* add BSD 3-Clause license
* remove some standard uniform names (already resolved internally by CShader since Hyprland 0.54)
* **release:** v0.2.3 [skip ci]


## [v0.2.2](https://github.com/Hyprnux/hyprglass/compare/v0.2.1...v0.2.2) - 2026-03-02

### Bug Fixes

* refactor shader creation for 0.54 refactored shader logic
* refactor callbacks for new event bus listeners

### Build System

* update hyprland compatibility to v0.54.0

### Chores

* **release:** v0.2.2 [skip ci]


## [v0.2.1](https://github.com/Hyprnux/hyprglass/compare/v0.2.0...v0.2.1) - 2026-02-18

### Bug Fixes

* **ci:** build tag from hyprland version bump was not picked up for release

### Build System

* update hyprland compatibility to v0.53.3

### Chores

* **release:** v0.2.1 [skip ci]


## [v0.2.0](https://github.com/Hyprnux/hyprglass/compare/v0.1.0...v0.2.0) - 2026-02-18

### Chores

* **release:** v0.2.0 [skip ci]

### Features

* settings improvments, with presets and built-in presets


## v0.1.0 - 2026-02-16

### Bug Fixes

* area was not damaged correctly  when moving while tiled layout
* remove some optimization causing noise when rendering blur and moving window (not worth it)
* gpu infinite render
* border radius
* working with shadow enable, not yet with shadow disabled

### CI/CD

* pipelines + docs

### Chores

* **release:** v0.1.0 [skip ci]

### Code Refactoring

* better file separation, renaming
* remove unused code and fix bounding box
* attempt with glass shader using SDF bezel refraction and poisson blur

### Features

* improve settings
* almost perfect
* more like apple variant, magnifying variant, seemd to be more beautiful
* add meniscus dispersion, color, and border rim to glasss shader (attempt to make it more apple-like, cool effects but not a big usable success)"
* tweaking little bit different approach
* add color_tint effect
* split corner/bezel SDF, add multi-pass blur and inner shadow (in order to avoid weird corners, but not really a success yet)
* add UV padding and remove wave distorsion
* draft working version of shader for transparent windows

### Performance Improvements

* massive gpu usage improvments, resample only when really needed
* shared blur framebuffers, remove raw texture sampler
* half-res blur pipeline, linear sampling, shadows

