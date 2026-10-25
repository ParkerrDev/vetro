#include "Beneath.hpp"
#include "Globals.hpp"
#include "Hash.hpp"
#include "RenderGuards.hpp"
#include "WindowGeometry.hpp"

#include <algorithm>
#include <hyprland/src/desktop/view/Window.hpp>
#include <hyprland/src/managers/fullscreen/FullscreenController.hpp>
#include <hyprland/src/render/Renderer.hpp>

namespace {
    // A frame draws a window more than once at most a few times (a floating
    // window over a fullscreen one): far more than this is not a frame's worth.
    constexpr size_t MAX_DRAWN = 128;

    CBox unite(const CBox& a, const CBox& b) {
        const double x1 = std::min(a.x, b.x);
        const double y1 = std::min(a.y, b.y);
        const double x2 = std::max(a.x + a.w, b.x + b.w);
        const double y2 = std::max(a.y + a.h, b.y + b.h);
        return CBox{x1, y1, x2 - x1, y2 - y1};
    }

    bool meets(const CBox& a, const CBox& b) {
        return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
    }

    // Keeps at most MAX_RECTS: the last one becomes the box round itself and
    // every one after it, square cornered, which covers them all.
    void cap(std::vector<Beneath::SRect>& rects) {
        if (rects.size() <= Beneath::MAX_RECTS)
            return;
        auto& last = rects[Beneath::MAX_RECTS - 1];
        for (size_t i = Beneath::MAX_RECTS; i < rects.size(); ++i)
            last.box = unite(last.box, rects[i].box);
        last.radius = 0.0f;
        last.window.reset();
        rects.resize(Beneath::MAX_RECTS);
    }
} // namespace

namespace Beneath {

    void beginFrame() {
        const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
        if (!g_pGlobalState || !monitor)
            return;
        g_pGlobalState->drawnWindows[monitor->m_id].clear();
    }

    void windowDrawn() {
        auto&      renderData = g_pHyprRenderer->m_renderData;
        const auto window     = renderData.currentWindow.lock();
        const auto monitor    = renderData.pMonitor.lock();
        if (!g_pGlobalState || !window || !monitor || g_pHyprRenderer->m_bRenderingSnapshot || RenderGuards::isForeignRender())
            return;
        auto& drawn = g_pGlobalState->drawnWindows[monitor->m_id];
        if (drawn.size() >= MAX_DRAWN)
            return;
        const auto box = WindowGeometry::computeWindowBox(window, monitor);
        if (!box || box->w <= 0.0 || box->h <= 0.0)
            return;

        // the corners the glass draws a window with (CGlassDecoration::renderPass)
        const bool  unrounded = Fullscreen::controller()->getFullscreenModes(window).internal == Fullscreen::FSMODE_FULLSCREEN;
        const float border    = WindowGeometry::glassBorder(window) * monitor->m_scale;
        float       radius    = unrounded ? 0.0f : window->rounding() * monitor->m_scale;
        if (radius > 0.0f)
            radius += border;
        drawn.push_back(SRect{
            .box    = WindowGeometry::applyMonitorTransform(*box, monitor),
            .radius = radius,
            .power  = unrounded ? 2.0f : window->roundingPower(),
            .window = window,
        });
    }

    std::vector<SRect> meeting(const std::vector<SRect>& rects, const CBox& area) {
        std::vector<SRect> out;
        for (const auto& rect : rects) {
            if (meets(rect.box, area))
                out.push_back(rect);
        }
        cap(out);
        return out;
    }

    std::vector<SRect> under(const PHLMONITOR& monitor, const CBox& area, const PHLWINDOW& self) {
        if (!g_pGlobalState || !monitor)
            return {};
        const auto found = g_pGlobalState->drawnWindows.find(monitor->m_id);
        if (found == g_pGlobalState->drawnWindows.end())
            return {};
        std::vector<SRect> others;
        for (const auto& rect : found->second) {
            if (!self || rect.window.lock() != self)
                others.push_back(rect);
        }
        return meeting(others, area);
    }

    uint64_t signature(const std::vector<SRect>& rects) {
        size_t hash = rects.size();
        for (const auto& rect : rects)
            Hash::hashCombine(hash, rect.box.x, rect.box.y, rect.box.w, rect.box.h, rect.radius, rect.power, rect.window.get());
        return hash;
    }

    bool contains(const std::vector<SRect>& rects, const PHLWINDOW& window) {
        return window && std::ranges::any_of(rects, [&](const SRect& rect) { return rect.window.lock() == window; });
    }

} // namespace Beneath
