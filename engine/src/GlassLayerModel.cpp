// The lens model's side of CGlassLayerSurface (docs/model.md). The classic side,
// in GlassLayerSurface.cpp, renders the layer into a framebuffer of its own and
// composites glass and layer together. This side never redirects anything: it
// reads the layer's committed buffer as the mask and draws the glass under where
// Hyprland is about to draw the layer itself.

#include "Diagnostics.hpp"
#include "GlassLayerSurface.hpp"
#include "GlassRenderer.hpp"
#include "Backdrop.hpp"
#include "Globals.hpp"
#include "LayerGeometry.hpp"
#include "Meniscus.hpp"
#include "WorkspaceAnimation.hpp"

#include <algorithm>
#include <chrono>
#include <cmath>
#include <format>
#include <hyprland/src/desktop/view/WLSurface.hpp>
#include <hyprland/src/protocols/core/Compositor.hpp>
#include <hyprland/src/render/OpenGL.hpp>
#include <hyprland/src/render/Renderer.hpp>

namespace {

// The layer's root surface, when its buffer can be sampled as it is: a plain
// 2D texture, drawn upright and whole.
SP<CWLSurfaceResource> readableSurface(const PHLLS& layerSurface) {
    const auto wlSurface = layerSurface ? layerSurface->wlSurface() : nullptr;
    const auto surface   = wlSurface ? wlSurface->resource() : nullptr;
    if (!surface)
        return nullptr;

    const auto& state   = surface->m_current;
    const auto& texture = state.texture;
    if (!texture || !texture->ok() || texture->m_type == Render::TEXTURE_EXTERNAL || texture->m_type == Render::TEXTURE_3D_LUT)
        return nullptr;
    if (state.transform != WL_OUTPUT_TRANSFORM_NORMAL || state.viewport.hasSource)
        return nullptr;
    if (state.bufferSize.x < 1 || state.bufferSize.y < 1)
        return nullptr;

    return surface;
}

// How long after a frame the model looks for the GPU's answers: long enough for the
// frame to have been drawn, short enough that a moved outline is followed at once.
constexpr auto MODEL_POLL_DELAY = std::chrono::milliseconds(3);

void onModelTimer() {
    if (!g_pGlobalState)
        return;

    bool waiting = false;
    for (const auto& [_, state] : g_pGlobalState->layerSurfaces)
        waiting = state->pollModel() || waiting;

    if (waiting && g_pGlobalState->modelTimer)
        g_pGlobalState->modelTimer->updateTimeout(MODEL_POLL_DELAY);
}

void armModelTimer() {
    auto& timer = g_pGlobalState->modelTimer;
    if (!timer) {
        timer = makeShared<CEventLoopTimer>(std::nullopt, [](SP<CEventLoopTimer>, void*) { onModelTimer(); }, nullptr);
        g_pEventLoopManager->addTimer(timer);
    }
    if (!timer->armed())
        timer->updateTimeout(MODEL_POLL_DELAY);
}

// The smallest box that holds both.
CBox boxAround(const CBox& a, const CBox& b) {
    const double x1 = std::min(a.x, b.x);
    const double y1 = std::min(a.y, b.y);
    const double x2 = std::max(a.x + a.w, b.x + b.w);
    const double y2 = std::max(a.y + a.h, b.y + b.h);
    return CBox{x1, y1, x2 - x1, y2 - y1};
}

// Pixels of this frame that hold what is under the layer: only what the render
// pass repainted. Outside it the framebuffer still shows the previous frame,
// with this layer and its glass in it.
bool repaintedThisFrame(const CBox& rect) {
    return CRegion(rect).subtract(g_pHyprRenderer->m_renderData.damage).empty();
}

} // namespace

bool CGlassLayerSurface::usesModel(PHLMONITOR monitor) const {
    const auto layerSurface = m_layerSurface.lock();
    if (!layerSurface || !monitor)
        return false;

    // The model works in framebuffer pixels and takes its mask from the layer's alpha.
    if (monitor->m_transform != WL_OUTPUT_TRANSFORM_NORMAL || resolveMaskSource() != EMaskSource::ALPHA_THRESHOLD)
        return false;

    const bool            isDark = resolveThemeIsDark();
    const std::string     preset = resolvePresetName();
    const SResolveContext ctx    = {preset, isDark, g_pGlobalState->config, g_pGlobalState->customPresets};
    if (!Meniscus::presetUsesModel(ctx))
        return false;

    return readableSurface(layerSurface) != nullptr;
}

void CGlassLayerSurface::resolveMaskLimits(float& threshold, float& feather) const {
    const auto& cfg = g_pGlobalState->config;
    threshold       = cfg.layersMaskThreshold ? static_cast<float>(**cfg.layersMaskThreshold) : 0.001f;
    feather         = cfg.layersMaskFeather ? static_cast<float>(**cfg.layersMaskFeather) : 0.0f;

    if (const auto layerSurface = m_layerSurface.lock()) {
        const auto& thresholds = g_pGlobalState->layerNamespaceMaskThresholds;
        if (const auto it = thresholds.find(layerSurface->m_namespace); it != thresholds.end())
            threshold = it->second;
        const auto& feathers = g_pGlobalState->layerNamespaceMaskFeathers;
        if (const auto it = feathers.find(layerSurface->m_namespace); it != feathers.end())
            feather = it->second;
    }

    // Capped so full-alpha content still gets full-strength glass.
    feather = std::clamp(feather, 0.0f, std::max(0.0f, 1.0f - threshold));
}

void CGlassLayerSurface::setModelActive(bool active) {
    if (active == m_modelActive)
        return;

    m_modelActive = active;
    if (active) {
        m_model = std::make_unique<Meniscus::CLayer>();
    } else {
        // back to the classic path: its own cache starts over, and the model's textures go
        m_model.reset();
        m_hasCachedSample = false;
    }
    m_backgroundDirty = true;
}

bool CGlassLayerSurface::pollModel() {
    if (!m_modelActive || !m_model || !g_pHyprOpenGL) {
        m_pendingBackgroundMark.reset();
        return false;
    }

    // A change behind the layer that pacing held back is acted on once its time has come.
    bool waiting = false;
    if (m_pendingBackgroundMark) {
        const auto&   config = g_pGlobalState->config;
        const int64_t fps    = config.layersLiveResampleFps ? **config.layersLiveResampleFps : 0;
        if (fps <= 0 || std::chrono::steady_clock::now() - m_lastDirtyMark >= std::chrono::nanoseconds(1'000'000'000 / fps)) {
            const CBox held = *m_pendingBackgroundMark;
            m_pendingBackgroundMark.reset();
            (void)markBackgroundDirtyWithin(held);
        }
        waiting = m_pendingBackgroundMark.has_value();
    }

    g_pHyprOpenGL->makeEGLCurrent();
    const auto state = m_model->poll();

    // One pixel of damage inside the layer brings its element back on the next frame.
    const auto wake = [this] {
        if (const auto layerSurface = m_layerSurface.lock()) {
            const auto position = layerSurface->position(Desktop::View::IGeometric::GEOMETRIC_CURRENT);
            if (std::isfinite(position.x) && std::isfinite(position.y))
                g_pHyprRenderer->damageBox(CBox{position.x, position.y, 1.0, 1.0});
        }
    };

    // A new map of the panes: where glass stood and where it will stand is drawn
    // again, because the panes and the lens fitted to them have changed.
    if (m_model->takeMapAdopted()) {
        damageModelPanes();
        damagePanes(m_model->knownPanes());
        wake();
    }

    switch (state) {
        case Meniscus::CLayer::EWaiting::NOTHING: return waiting;
        case Meniscus::CLayer::EWaiting::ANSWER: return true;
        case Meniscus::CLayer::EWaiting::FRAME: break;
    }

    // The outline moved: the next frame rebuilds the field and then asks for what changed.
    wake();
    return waiting;
}

std::string CGlassLayerSurface::describeModel() const {
    const auto layerSurface = m_layerSurface.lock();
    if (!layerSurface)
        return {};

    std::string out = std::format("  {}: {}\n", layerSurface->m_namespace, m_modelActive && m_model ? "lens model" : "classic shader");
    if (!m_modelActive || !m_model)
        return out;

    out += std::format("      {}{}{}\n", m_model->describeState(), layerSurface->m_mapped ? "" : ", not mapped", m_pendingBackgroundMark ? ", a change behind it held back" : "");

    for (const auto& pane : m_model->panes()) {
        out += std::format("      pane {:.0f},{:.0f} {:.0f}x{:.0f}px  short side {:.0f}px  frost {}{}\n", pane.box.x, pane.box.y, pane.box.w, pane.box.h, pane.shortSide,
                           pane.hasSample ? std::format("{:.0f}x{:.0f} texels, 1/{} of the screen's resolution", pane.frost ? pane.frost->m_size.x : 0.0,
                                                        pane.frost ? pane.frost->m_size.y : 0.0, pane.reduction) :
                                            std::string("not sampled yet"),
                           pane.dirty ? ", to be resampled" : "");
    }
    return out;
}

int CGlassLayerSurface::paddingPx() const {
    if (!m_modelActive || !m_model)
        return GlassRenderer::SAMPLE_PADDING_PX;
    // what the frost reads beyond a pane (plus one texel of its coarsest grid) and what the shadow covers
    return m_model->paddingReach();
}

CBox CGlassLayerSurface::globalLogicalBox(const CBox& monitorPixels, const PHLMONITOR& monitor) const {
    const float scale = monitor && monitor->m_scale > 0.0f ? monitor->m_scale : 1.0f;
    CBox        box   = monitorPixels;
    box.scale(1.0 / scale);
    if (monitor)
        box.translate(monitor->m_position);
    return box;
}

void CGlassLayerSurface::markContentChanged(const std::optional<CBox>& surfaceBox) {
    if (!m_modelActive || !m_model)
        return;

    const auto layerSurface = m_layerSurface.lock();
    const auto monitor      = layerSurface ? layerSurface->m_monitor.lock() : nullptr;
    if (!monitor)
        return;

    if (!surfaceBox) {
        m_model->contentChanged(std::nullopt);
        return;
    }

    // Surface-local logical pixels to box-local device pixels, with room for rounding.
    const float scale   = monitor->m_scale > 0.0f ? monitor->m_scale : 1.0f;
    CBox        changed = *surfaceBox;
    changed.scale(scale).expand(2.0);
    m_model->contentChanged(changed);
}

bool CGlassLayerSurface::markBackgroundDirtyWithin(const CBox& damagedGlobalBox) {
    if (!m_modelActive || !m_model)
        return false;

    const auto layerSurface = m_layerSurface.lock();
    const auto monitor      = layerSurface ? layerSurface->m_monitor.lock() : nullptr;
    if (!monitor)
        return true;

    auto layerBox = LayerGeometry::computeLayerBox(layerSurface, monitor);
    if (!layerBox)
        return true;

    // Same pacing as the classic path: at most layers:live_resample_fps marks a second.
    const auto&   config = g_pGlobalState->config;
    const int64_t fps    = config.layersLiveResampleFps ? **config.layersLiveResampleFps : 0;
    const auto    now    = std::chrono::steady_clock::now();
    const bool    paced  = fps > 0 && now - m_lastDirtyMark < std::chrono::nanoseconds(1'000'000'000 / fps);

    bool marked = false;
    for (auto& pane : m_model->panes()) {
        if (pane.dirty)
            continue;

        const CBox rect = m_model->sampleRect(pane, layerBox->pos(), monitor->m_pixelSize);
        if (rect.w < 1.0 || rect.h < 1.0)
            continue;

        const CBox area = globalLogicalBox(rect, monitor);
        if (!area.overlaps(damagedGlobalBox))
            continue;
        if (paced) {
            // Too soon after the last one. It is kept, not dropped: the last change
            // of a burst is the one the glass has to end up showing.
            m_pendingBackgroundMark = m_pendingBackgroundMark ? boxAround(*m_pendingBackgroundMark, damagedGlobalBox) : damagedGlobalBox;
            armModelTimer();
            return true;
        }

        // The whole of what this pane reads must be repainted before it is read again:
        // beside the committed area the framebuffer still holds last frame's glass.
        pane.dirty = true;
        g_pHyprRenderer->damageBox(area);
        marked = true;
    }

    if (marked)
        m_lastDirtyMark = now;
    return true;
}

void CGlassLayerSurface::damageModelPanes() {
    if (m_model)
        damagePanes(m_model->panes());
}

void CGlassLayerSurface::damagePanes(const std::vector<Meniscus::SPane>& panes) {
    if (!m_model)
        return;

    const auto layerSurface = m_layerSurface.lock();
    const auto monitor      = layerSurface ? layerSurface->m_monitor.lock() : nullptr;
    if (!monitor)
        return;

    auto layerBox = LayerGeometry::computeLayerBox(layerSurface, monitor);
    if (!layerBox)
        return;

    // as far as the shadow falls and as far as the frost reads, on its coarsest grid
    const int reach = m_model->damageReach();
    for (const auto& pane : panes) {
        CBox       rect = pane.box.copy().translate(layerBox->pos()).expand(reach);
        const CBox area = globalLogicalBox(rect, monitor);
        if (area.w > 0.0 && area.h > 0.0 && std::isfinite(area.x) && std::isfinite(area.y) && std::isfinite(area.w) && std::isfinite(area.h))
            g_pHyprRenderer->damageBox(area);
    }
}

void CGlassLayerSurface::sampleModelPanes(PHLMONITOR monitor, const SP<Render::IFramebuffer>& source, const CBox& layerBox, bool xray,
                                          const std::vector<Beneath::SRect>& under) {
    const auto layerSurface = m_layerSurface.lock();
    if (!layerSurface || !m_model)
        return;

    // What invalidates every pane at once: the same triggers as the classic path.
    const uint64_t generation  = g_pGlobalState->getSceneGeneration(monitor);
    const auto&    config      = g_pGlobalState->config;
    const bool     forceLive   = config.layersForceLiveResample && **config.layersForceLiveResample;
    // The layer's own fade is left out: it changes nothing behind the layer.
    const bool     isMoving    = layerSurface->positionAnimation()->isBeingAnimated() || layerSurface->sizeAnimation()->isBeingAnimated();
    const bool     isAnimating = isMoving || WorkspaceAnimation::anyWorkspaceAnimating(monitor);
    const bool sourceSwitched = xray != m_cachedFromSnapshot;

    if (generation != m_lastSceneGeneration || isAnimating || forceLive || sourceSwitched || m_backgroundDirty) {
        for (auto& pane : m_model->panes()) {
            pane.dirty = true;
            // a sample of the live frame must never stand in for the x-ray one, nor the reverse
            if (sourceSwitched)
                pane.hasSample = false;
        }
        m_lastSceneGeneration = generation;
        m_cachedFromSnapshot  = xray;
        m_backgroundDirty     = false;
    }

    for (auto& pane : m_model->panes()) {
        const CBox rect = m_model->sampleRect(pane, layerBox.pos(), source->m_size);
        if (rect.w < 1.0 || rect.h < 1.0) {
            pane.hasSample = false;
            continue;
        }

        // A pane that moved reads a different part of the screen.
        const bool needed = !pane.hasSample || pane.dirty || rect != pane.sampled;
        if (!needed || !layerSurface->m_mapped) {
            // unchanged behind it, or fading out over pixels that are no longer its background
            Diagnostics::recordLayerCacheHit(monitor->m_id);
            continue;
        }

        const auto snapshot = xray ? Backdrop::get(monitor, source) : nullptr;
        // windows under this pane: the clean page, read from the frame as the
        // live glass is, so it waits for the frame to be repainted there
        Meniscus::SCleanSource cleanSource{source, snapshot, snapshot ? Beneath::meeting(under, rect) : std::vector<Beneath::SRect>{}};
        const bool clean    = !cleanSource.under.empty();
        const bool fresh    = clean ? repaintedThisFrame(rect) : xray ? static_cast<bool>(snapshot) : repaintedThisFrame(rect);
        // A sliding layer outruns the damage asked for a frame earlier. Its pane
        // itself is repainted, only the margin the frost reads around it lags:
        // sample now, and again next frame, rather than lose the glass for the slide.
        const bool ahead = !fresh && (!xray || clean) && isMoving && repaintedThisFrame(pane.box.copy().translate(layerBox.pos()).intersection(CBox{{}, source->m_size}));
        if (!fresh) {
            // Not all of it is fresh this frame: ask for all of it, and until then draw what was there.
            pane.dirty = true;
            g_pHyprRenderer->damageBox(globalLogicalBox(rect, monitor));
            if (!ahead) {
                Diagnostics::recordLayerDeferredResample(monitor->m_id);
                if (xray)
                    Diagnostics::recordXrayDeferredResample(monitor->m_id);
                if (rect != pane.sampled)
                    pane.hasSample = false;
                continue;
            }
        }

        m_model->sample(pane, xray && !clean ? snapshot : source, layerBox.pos(), source, clean ? &cleanSource : nullptr);
        pane.dirty = ahead;
        Diagnostics::recordLayerCacheMiss(monitor->m_id);
    }
}

void CGlassLayerSurface::drawModel(PHLMONITOR monitor, float alpha, bool xray, const std::vector<Beneath::SRect>& under) {
    auto& programs = g_pGlobalState->meniscusPrograms;
    programs.initializeIfNeeded();
    if (!programs.ready())
        return;

    const auto layerSurface = m_layerSurface.lock();
    if (!layerSurface || !monitor || !m_model)
        return;

    const auto source = g_pHyprRenderer->m_renderData.currentFB;
    if (!source)
        return;

    const auto layerBox = LayerGeometry::computeLayerBox(layerSurface, monitor);
    if (!layerBox)
        return;

    const auto surface = readableSurface(layerSurface);
    if (!surface)
        return;

    // Nothing shows through a buffer without alpha: no glass to draw under it.
    const auto& texture = surface->m_current.texture;
    if (texture->m_opaque || texture->m_type != Render::TEXTURE_RGBA)
        return;

    Meniscus::SMaskSource mask;
    mask.texture = texture;
    resolveMaskLimits(mask.threshold, mask.feather);

    // Hyprland draws a buffer that is a pixel or two off the box of a fractionally
    // scaled layer one texel to one pixel, cutting the excess off: read it the same way.
    const auto& bufferSize = surface->m_current.bufferSize;
    const bool  fractional = std::floor(monitor->m_scale) != monitor->m_scale;
    if (fractional && surface->m_current.scale == 1 && layerBox->size() != bufferSize && std::abs(layerBox->w - bufferSize.x) < 3.0 && std::abs(layerBox->h - bufferSize.y) < 3.0 &&
        !layerSurface->sizeAnimation()->isBeingAnimated())
        mask.uvScale = layerBox->size() / bufferSize;

    const bool            isDark = resolveThemeIsDark();
    const std::string     preset = resolvePresetName();
    const SResolveContext ctx    = {preset, isDark, g_pGlobalState->config, g_pGlobalState->customPresets};
    m_modelParams                = Meniscus::resolveParams(ctx, monitor->m_scale);
    m_modelActive                = true;

    Diagnostics::recordLayerGlassDraw(monitor->m_id);

    const bool hasPanes = m_model->update(mask, layerBox->size(), m_modelParams, source);

    // The model is waiting for an answer from the GPU, or pacing itself. A timer reads
    // the answer between frames, so that an outline that did not move costs no frame.
    if (m_model->takePollRequest())
        armModelTimer();

    // The outline moved and the field was rebuilt after this frame's damage was fixed:
    // what is on screen around the change shows the old rim and shadow.
    if (const auto repaint = m_model->takeRepaint()) {
        const double reach = m_model->repaintReach();
        const CBox   area  = globalLogicalBox(repaint->intersection(CBox{{}, layerBox->size()}).translate(layerBox->pos()).expand(reach), monitor);
        if (area.w > 0.0 && area.h > 0.0 && std::isfinite(area.x) && std::isfinite(area.y) && std::isfinite(area.w) && std::isfinite(area.h))
            g_pHyprRenderer->damageBox(area);
    }

    if (!hasPanes)
        return;

    sampleModelPanes(monitor, source, *layerBox, xray, under);

    m_model->draw(source, *layerBox, mask, alpha);
}
