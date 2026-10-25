#pragma once

#include <hyprland/src/desktop/DesktopTypes.hpp>
#include <hyprland/src/helpers/math/Math.hpp>
#include <hyprland/src/helpers/memory/Memory.hpp>
#include <hyprutils/math/Box.hpp>

#include <cstddef>
#include <vector>

// What lies under a piece of glass besides the wallpaper and the desktop's
// widgets: the windows Hyprland has drawn so far in this frame, in the order it
// drew them. Glass with none of them under the page it reads takes the backdrop
// (the wallpaper alone, Backdrop.hpp), as it always did. Glass over one of them
// reads a clean copy of the frame (Meniscus::copyClean): the frame where those
// windows lie and the wallpaper everywhere else, so other windows show through
// glass and the widgets, drawn between the wallpaper and the windows, never do
// (2026-10-18: "you should be able to see other windows through other windows
// when they overlap, just not for the widgets in the wallpaper").
namespace Beneath {

    struct SRect {
        CBox         box;           // device pixels of the monitor's frame, as the glass reads it
        float        radius = 0.0f; // its corners, device pixels
        float        power  = 2.0f; // Hyprland's rounding_power
        PHLWINDOWREF window;
    };

    // The most windows one clean copy tells apart; any more are merged into one box.
    inline constexpr size_t MAX_RECTS = 16;

    // RENDER_BEGIN: the rendered monitor's list starts empty.
    void beginFrame();

    // RENDER_POST_WINDOW: the window Hyprland just drew joins the list.
    void windowDrawn();

    // The windows drawn so far this frame on `monitor` whose box meets `area`
    // (device pixels of its frame), `self` left out: at most MAX_RECTS of them.
    [[nodiscard]] std::vector<SRect> under(const PHLMONITOR& monitor, const CBox& area, const PHLWINDOW& self);

    // Of `rects`, those that meet `area`, at most MAX_RECTS.
    [[nodiscard]] std::vector<SRect> meeting(const std::vector<SRect>& rects, const CBox& area);

    // A number that changes when the list does: a page copied with other windows
    // under it is stale once they moved.
    [[nodiscard]] uint64_t signature(const std::vector<SRect>& rects);

    // Whether `window` is one of `rects`.
    [[nodiscard]] bool contains(const std::vector<SRect>& rects, const PHLWINDOW& window);

} // namespace Beneath
