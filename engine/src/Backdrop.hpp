#pragma once

#include <hyprland/src/helpers/memory/Memory.hpp>
#include <hyprland/src/render/Renderer.hpp>

// X-ray glass reads the backdrop: the monitor's wallpaper and nothing else
// (its background layers), drawn by the engine itself into a framebuffer of
// the monitor's size, with a mip chain for the blur. It is drawn again only
// when a background layer commits, moves or fades, or the monitor's frame
// changes size or format. So the glass of windows, of the bar and of the
// desktop's cards costs no copy of the screen on any other frame, nothing
// above the wallpaper (the desktop's widgets, other windows) ever shows in
// it, and one texture serves every pane on the monitor.
namespace Backdrop {

    // Whether x-ray glass can be drawn on this monitor at all: not under
    // render:xp_mode, which draws no wallpaper, nor on a rotated output, where
    // the wallpaper's boxes would land unrotated. Cheap, and safe while the
    // pass is built.
    [[nodiscard]] bool possible(const PHLMONITOR& monitor);

    // Marks the monitor's backdrop as wanted this frame. Called while the pass
    // is built, never from inside it.
    void request(const PHLMONITOR& monitor);

    // The wallpaper as it stands now, as a number that changes whenever a
    // background layer commits, moves or fades: a copy that has the backdrop
    // in it is out of date once this differs from what it was when the copy
    // was made. Zero before the monitor's backdrop was first asked for.
    // Cheap, and safe while the pass is built.
    [[nodiscard]] uint64_t stamp(const PHLMONITOR& monitor);

    // RENDER_BEGIN: ages the rendered monitor's backdrop and drops one nothing
    // asked for lately, which frees its texture.
    void beginFrame();

    // The monitor's backdrop, drawn first when it is missing or out of date.
    // Only from inside a pass element's draw; `frame` is the target being
    // drawn, whose size and format the backdrop takes. Null when it cannot be
    // made.
    [[nodiscard]] SP<Render::IFramebuffer> get(const PHLMONITOR& monitor, const SP<Render::IFramebuffer>& frame);

    // The backdrop blurred to `radius` device pixels with a true Gaussian, at a
    // half, a quarter or an eighth of its size (the blur hides the difference),
    // made again only when the backdrop or the radius changes. Read with one
    // bilinear tap, it is as smooth as the radius, where a mip level read
    // instead shows its blocks. Null when it cannot be made.
    [[nodiscard]] SP<Render::IFramebuffer> blurred(const PHLMONITOR& monitor, const SP<Render::IFramebuffer>& frame, float radius);

    // `source`, which has a mip chain, blurred to `radius` device pixels with a
    // true Gaussian into `out`, at a half, a quarter or an eighth of its size.
    // Only from inside a pass element's draw; `frame` is bound again after.
    // False when it could not be made.
    [[nodiscard]] bool blurInto(const SP<Render::IFramebuffer>& source, SP<Render::IFramebuffer>& out, float radius, const SP<Render::IFramebuffer>& frame);

    // Whether `framebuffer` is some monitor's backdrop: shared, so never to be
    // resized or written by a pane that holds it.
    [[nodiscard]] bool isBackdrop(const SP<Render::IFramebuffer>& framebuffer);
    [[nodiscard]] bool isBackdropOrBlur(const SP<Render::IFramebuffer>& framebuffer);

    void forget(MONITORID monitor);
    void clear();

} // namespace Backdrop
