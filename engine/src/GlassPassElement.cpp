#include "GlassPassElement.hpp"
#include "Diagnostics.hpp"
#include "GlassDecoration.hpp"
#include "Backdrop.hpp"
#include "Globals.hpp"
#include "Motion.hpp"
#include "PluginConfig.hpp"
#include "WindowGeometry.hpp"

#include <cmath>

CGlassPassElement::CGlassPassElement(const SGlassPassData& data)
    : m_data(data) {}

std::vector<UP<IPassElement>> CGlassPassElement::draw() {
    // debug:mode = hints_only: keep every hint below (damage, live blur,
    // simplification bypass) but do none of the GL work, to isolate the
    // render pass's own cost from the glass pipeline's.
    if (currentDebugMode() == EDebugMode::HINTS_ONLY)
        return {};

    if (!m_data.decoration.valid())
        return {};

    // Hyprland renders a floating window over fullscreen more than once per
    // frame; every copy but the last queued is a no-op, so the glass is applied
    // exactly once and samples the framebuffer as it is under the last copy.
    if (!m_data.decoration->isCurrentGlassPass(m_data.frameSerial, m_data.queueIndex))
        return {};

    const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
    // a window being bent takes its glass along: drawn here, lifted off the frame
    const Motion::CLift lift(m_data.decoration->getOwner(), monitor);
    m_data.decoration->renderPass(monitor, m_data.alpha, m_data.xray, m_data.under);

    return {};
}

std::optional<CBox> CGlassPassElement::paddedLogicalBox() const {
    if (!m_data.decoration.valid())
        return std::nullopt;

    auto window = m_data.decoration->getOwner();
    if (!window)
        return std::nullopt;

    const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
    auto box = WindowGeometry::computeWindowBox(window, monitor);
    if (!box)
        return std::nullopt;

    // IPassElement::boundingBox() is a monitor-local LOGICAL coordinate
    // contract; computeWindowBox() returns physical pixels for renderPass()'s
    // own use, so convert back and expand by our sampling padding here.
    const float scale = monitor->m_scale > 0.0f ? monitor->m_scale : 1.0f;
    box->scale(1.0 / scale).expand(GlassRenderer::SAMPLE_PADDING_PX / scale).noNegativeSize().round();
    if (!std::isfinite(box->x) || !std::isfinite(box->y) || !std::isfinite(box->w) || !std::isfinite(box->h) || box->w <= 0.0 || box->h <= 0.0)
        return std::nullopt;

    return box;
}

std::optional<CBox> CGlassPassElement::boundingBox() {
    return paddedLogicalBox();
}

CRegion CGlassPassElement::opaqueRegion() {
    // debug:mode keeps the render pass as it was without the glass's own work
    if (currentDebugMode() != EDebugMode::OFF || !m_data.decoration.valid() || Motion::bends(m_data.decoration->getOwner()))
        return {};
    return m_data.decoration->opaqueRegion(g_pHyprRenderer->m_renderData.pMonitor.lock(), m_data.xray, !m_data.under.empty());
}

bool CGlassPassElement::needsLiveBlur() {
    // debug:mode = gl_work_only: run the GL pipeline but withhold these
    // hints, isolating their render-pass cost (full re-render) from the
    // pipeline's own GL cost.
    if (currentDebugMode() == EDebugMode::GL_WORK_ONLY)
        return false;

    // Must agree with boundingBox() on whether a box exists: Hyprland's
    // CRenderPass::render() asserts a bounding box for any element reporting
    // live blur ("No bounding box for an element with live blur is illegal",
    // Pass.cpp) and aborts the compositor if it's absent. A truthy result
    // here also guarantees the decoration, its window and its monitor are
    // all valid, since paddedLogicalBox() checks each of them.
    if (!paddedLogicalBox().has_value())
        return false;

    const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
    const auto window   = m_data.decoration->getOwner();
    if (!monitor || !window)
        return false;

    const auto box = WindowGeometry::computeWindowBox(window, monitor);
    if (!box)
        return false;

    // Only expand damage/exempt occlusion when the cached background actually
    // needs a fresh sample this frame — a cache hit needs neither (see the
    // "Cache hit" case in renderPass()). Transformed like renderPass()'s own
    // box so the two calls agree on 90/270-degree-rotated monitors too.
    const CBox transformBox = WindowGeometry::applyMonitorTransform(*box, monitor);
    if (!m_data.decoration->wantsBackgroundResample(monitor, transformBox, m_data.xray, m_data.under))
        return false;

    // X-ray reads the backdrop, never the frame: nothing under it need be drawn,
    // unless windows lie under it, which its clean page reads from the frame.
    return !(m_data.xray && m_data.under.empty() && Backdrop::possible(monitor));
}

bool CGlassPassElement::needsPrecomputeBlur() {
    return false;
}

bool CGlassPassElement::disableSimplification() {
    // Left enabled, including under debug:mode = gl_work_only: an element
    // whose padded box misses the render pass's damage is safely discarded.
    // One that survives discard still draws its whole box, so needsLiveBlur
    // above is what keeps the background beneath that box correct, not this.
    return false;
}

void CGlassPassElement::discard() {
    // CRenderPass::render() calls this in place of draw() for an element
    // simplify() dropped — renderPass() never runs for it, so it's otherwise
    // invisible to every other counter this file records. The only way
    // `hyprctl hyprglass stats` can show how many glass windows exist versus
    // how many are actually being kept current.
    if (const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock())
        Diagnostics::recordWindowPassDiscarded(monitor->m_id);

    IPassElement::discard();
}
