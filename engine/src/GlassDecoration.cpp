#include "GlassDecoration.hpp"
#include "BackgroundDamageObserver.hpp"
#include "Diagnostics.hpp"
#include "GlassPassElement.hpp"
#include "GlassRenderer.hpp"
#include "Backdrop.hpp"
#include "Globals.hpp"
#include "Hash.hpp"
#include "RenderGuards.hpp"
#include "Tuning.hpp"
#include "WindowGeometry.hpp"
#include "WorkspaceAnimation.hpp"

#include <algorithm>
#include <GLES3/gl32.h>
#include <hyprland/src/desktop/view/Window.hpp>
#include <hyprland/src/desktop/rule/windowRule/WindowRuleApplicator.hpp>
#include <hyprland/src/desktop/state/FocusState.hpp>
#include <hyprland/src/managers/fullscreen/FullscreenController.hpp>
#include <hyprland/src/render/OpenGL.hpp>
#include <hyprland/src/render/Renderer.hpp>

CGlassDecoration::CGlassDecoration(PHLWINDOW window)
    : IHyprWindowDecoration(window), m_window(window) {
}

CGlassDecoration::~CGlassDecoration() {
    withdrawNoBlur();
}

// Glass replaces Hyprland's blur for this window. Mark glassed windows with
// the noblur property so Hyprland composites their translucency against the
// live framebuffer (which contains the glass) instead of its pre-frame cached
// blur snapshot, which is captured before plugin decorations render (#46).
void CGlassDecoration::updateNoBlurProp(bool glassEnabled) {
    const auto& config = g_pGlobalState->config;
    const bool manage = config.manageWindowBlur && **config.manageWindowBlur;

    if (!manage || !glassEnabled) {
        withdrawNoBlur();
        return;
    }

    if (m_noBlurApplied)
        return;

    try {
        const auto window = m_window.lock();
        if (window && window->m_ruleApplicator) {
            window->m_ruleApplicator->noBlur().set(true, Desktop::Types::PRIORITY_SET_PROP);
            m_noBlurApplied = true;
            damageEntire();
        }
    } catch (...) {}
}

void CGlassDecoration::withdrawNoBlur() {
    if (!m_noBlurApplied)
        return;
    m_noBlurApplied = false;

    try {
        const auto window = m_window.lock();
        if (window && window->m_ruleApplicator) {
            window->m_ruleApplicator->noBlur().unset(Desktop::Types::PRIORITY_SET_PROP);
            damageEntire();
        }
    } catch (...) {}
}

// Fullscreen toggles re-apply window rules, which can drop the noblur prop
// while m_noBlurApplied still claims it's held.
void CGlassDecoration::onFullscreenStateChanged() {
    m_noBlurApplied = false;
    damageEntire();
}

static float selfSampleFor(const SResolveContext& ctx) {
    return std::clamp(ctx.f(GLASS_F_SELF_SAMPLE), 0.0f, 1.0f);
}

CGlassDecoration::EEnabledResolution CGlassDecoration::resolveEnabled() const {
    const auto& config = g_pGlobalState->config;
    const bool globalEnabled = config.enabled && **config.enabled;
    const bool skipOpaque    = config.skipOpaqueWindows && **config.skipOpaqueWindows;

    try {
        const auto window = m_window.lock();
        if (window && window->m_ruleApplicator) {
            const auto& tags = window->m_ruleApplicator->m_tagKeeper;
            // isTagged() already matches dynamic tags ("tag*") — no stripping needed here.
            // Disabled tag wins over enabled tag if both are present.
            if (tags.isTagged(std::string(TAG_DISABLED)))
                return EEnabledResolution::Disabled;
            if (tags.isTagged(std::string(TAG_ENABLED)))
                return EEnabledResolution::Enabled;
        }

        // With its border made of glass (windows:border) a window that gets no
        // pane still gets its frame: the band its border takes, and no more.
        const bool framed = window && WindowGeometry::glassBorder(window) > 0.0f;

        // A window that says its content is opaque gets no pane under it either:
        // it would be drawn again on every frame the window changes, and never seen.
        if (taggedSolid())
            return framed ? EEnabledResolution::FrameOnly : EEnabledResolution::Disabled;

        // A global disable must pre-empt the opaque-skip check below: otherwise
        // an opaque window with the plugin off entirely would still resolve to
        // DisabledBecauseOpaque and inflate the opaque-skip counter in draw().
        if (!globalEnabled)
            return framed ? EEnabledResolution::FrameOnly : EEnabledResolution::Disabled;

        // Nothing behind an opaque window is visible, unless the window
        // self-samples: then the glass shows the window's own content, which
        // does change, so the opaque-skip must yield to it.
        if (skipOpaque && window && window->opaque()) {
            const bool         isDark = resolveThemeIsDark();
            const std::string  preset = resolvePresetName();
            const SResolveContext ctx = {preset, isDark, config, g_pGlobalState->customPresets};
            if (selfSampleFor(ctx) <= 0.0f)
                return framed ? EEnabledResolution::FrameOnly : EEnabledResolution::DisabledBecauseOpaque;
        }
    } catch (...) {}

    return globalEnabled ? EEnabledResolution::Enabled : EEnabledResolution::Disabled;
}

bool CGlassDecoration::taggedSolid() const {
    const auto window = m_window.lock();
    return window && window->m_ruleApplicator && window->m_ruleApplicator->m_tagKeeper.isTagged(std::string(TAG_SOLID));
}

bool CGlassDecoration::resolveThemeIsDark() const {
    try {
        const auto window = m_window.lock();
        if (window && window->m_ruleApplicator) {
            const std::string lightTag = std::string(TAG_THEME_PREFIX) + "light";
            const std::string darkTag  = std::string(TAG_THEME_PREFIX) + "dark";
            if (window->m_ruleApplicator->m_tagKeeper.isTagged(lightTag))
                return false;
            if (window->m_ruleApplicator->m_tagKeeper.isTagged(darkTag))
                return true;
        }

        const auto& config = g_pGlobalState->config;
        const auto theme = readStringConfig(config.defaultTheme);
        if (!theme.empty())
            return theme != "light";
    } catch (...) {}

    return true;
}

bool CGlassDecoration::resolveXray() const {
    const auto window = m_window.lock();
    if (!window || !Backdrop::possible(window->m_monitor.lock()))
        return false;

    const bool            isDark = resolveThemeIsDark();
    const std::string     preset = resolvePresetName();
    const SResolveContext ctx    = {preset, isDark, g_pGlobalState->config, g_pGlobalState->customPresets};
    return ctx.i(GLASS_I_XRAY) > 0;
}

std::string CGlassDecoration::resolvePresetName() const {
    try {
        const auto window = m_window.lock();
        if (window && window->m_ruleApplicator) {
            for (const auto& tag : window->m_ruleApplicator->m_tagKeeper.getTags()) {
                if (tag.starts_with(TAG_PRESET_PREFIX))
                    return std::string(Glass::stripDynamicTagMarker(tag.substr(TAG_PRESET_PREFIX.size())));
            }
        }

        const auto& config = g_pGlobalState->config;
        const auto preset = readStringConfig(config.defaultPreset);
        if (!preset.empty())
            return std::string(preset);
    } catch (...) {}

    return "default";
}

SDecorationPositioningInfo CGlassDecoration::getPositioningInfo() {
    SDecorationPositioningInfo info;
    info.priority       = 10000;
    info.policy         = DECORATION_POSITION_ABSOLUTE;
    info.desiredExtents = {{0, 0}, {0, 0}};
    return info;
}

void CGlassDecoration::onPositioningReply(const SDecorationPositioningReply& reply) {}

void CGlassDecoration::queueGlassPass(float alpha, bool xray, std::vector<Beneath::SRect> under) {
    // A duplicate copy is redirected into the dedupe sink and dropped whole: it
    // needs no glass, and stamping it would leave the surviving element stale.
    if (g_pGlobalState->dedupe.guard)
        return;

    CGlassPassElement::SGlassPassData data{m_self, alpha};
    data.xray  = xray;
    data.under = std::move(under);

    // Only the real monitor pass is de-duplicated: snapshots, screencopy and
    // overview framebuffers render the window once, out of frame order.
    const bool managed = g_pGlobalState->frameSerial != 0 && !g_pHyprRenderer->m_bRenderingSnapshot &&
        g_pHyprRenderer->m_renderData.projectionType == Render::RPT_MONITOR;

    if (managed) {
        if (m_glassFrameSerial != g_pGlobalState->frameSerial) {
            m_glassFrameSerial = g_pGlobalState->frameSerial;
            m_glassQueueIndex  = 0;
        } else
            ++m_glassQueueIndex;

        data.frameSerial = m_glassFrameSerial;
        data.queueIndex  = m_glassQueueIndex;
    }

    // m_renderPass, never addPassElement: draw() runs inside Hyprland's own
    // per-window redirect for transformed windows (motion blur), whose pass
    // renders into a work buffer cleared to transparent — nothing to sample.
    g_pHyprRenderer->m_renderPass.add(makeUnique<CGlassPassElement>(data));
}

void CGlassDecoration::draw(PHLMONITOR monitor, float const& alpha) {
    if (!g_pGlobalState)
        return;

    // Render-order fingerprint: fold this window's identity/geometry/alpha into
    // the monitor's running hash in z-order, before resolveEnabled() so background
    // windows still count. Guarded by isForeignRender(), not a bare mainFB check,
    // since a foreign replay's z-order isn't this frame's real one. Folded at most
    // once per frameSerial: draw() itself runs 2-3x per monitor frame for a floating
    // window over fullscreen, and folding every copy would make the hash depend on
    // how many of those copies happened to render this frame rather than on the scene.
    if (monitor && !RenderGuards::isForeignRender() &&
        (g_pGlobalState->frameSerial == 0 || m_lastFoldedFrameSerial != g_pGlobalState->frameSerial)) {
        if (const auto window = m_window.lock()) {
            const auto workspace = window->m_workspace;
            const Vector2D workspaceRenderOffset =
                (workspace && !window->m_pinned) ? workspace->m_renderOffset->value() : Vector2D();
            const auto fullscreenMode = Fullscreen::controller()->getFullscreenModes(window).internal;

            auto& fingerprint = g_pGlobalState->renderFingerprints[monitor->m_id];
            Hash::hashCombine(fingerprint.runningHash, window.get(), window->positionAnimation()->value(),
                               window->sizeAnimation()->value(), alpha, fullscreenMode, workspaceRenderOffset);

            m_lastFoldedFrameSerial = g_pGlobalState->frameSerial;
        }
    }

    const auto enabledResolution = resolveEnabled();
    const bool enabled   = enabledResolution == EEnabledResolution::Enabled;
    // A frame is drawn by the same pass as a pane, and is none of a pane's
    // business otherwise: the window keeps its blur, and its shadow is its own.
    const bool frameOnly = enabledResolution == EEnabledResolution::FrameOnly;
    updateNoBlurProp(enabled);
    if (enabled && !m_glassWasEnabled)
        scheduleShadowsForWindowGlass();
    m_glassWasEnabled = enabled;
    if (frameOnly)
        m_lastSelfSample = 0.0f;
    if (!enabled && !frameOnly) {
        m_lastSelfSample = 0.0f;

        // Only count a skip that resolveEnabled() itself attributes to
        // skip_opaque_windows — a tag or global-disable skip doesn't have a
        // counter of its own (see EEnabledResolution) and isn't counted here.
        if (monitor && enabledResolution == EEnabledResolution::DisabledBecauseOpaque)
            Diagnostics::recordWindowOpaqueSkipped(monitor->m_id);
        return;
    }

    // A foreign render must not queue a pass element at all: it would sample its own
    // framebuffer into this window's real, persistent background cache. Nor a
    // snapshot (the picture of a closing window that its fadeout shows): it is
    // drawn into a cleared framebuffer, so its glass would be black, and the
    // fadeout would show the window black wherever it lets the glass through.
    if (RenderGuards::isForeignRender() || g_pHyprRenderer->m_bRenderingSnapshot)
        return;

    const bool xray = resolveXray();
    if (xray)
        Backdrop::request(monitor);

    // The windows Hyprland drew before this one, where its glass reads them.
    auto under  = xray ? windowsUnder(monitor) : std::vector<Beneath::SRect>{};
    m_lastXray  = xray;
    m_lastUnder = under;
    queueGlassPass(alpha, xray, std::move(under));

    // A slide translates the scene under us without any geometry change, and
    // Hyprland's per-tick window damage carries none of our sampling padding.
    // Damage only: no cache state may be touched from a render.
    const auto window = m_window.lock();
    if (window && !window->m_pinned) {
        const auto workspace = window->m_workspace;
        if (workspace && workspace->m_renderOffset->isBeingAnimated())
            damageEntire();
    }
}

PHLWINDOW CGlassDecoration::getOwner() {
    return m_window.lock();
}

void CGlassDecoration::markBackgroundDirty() {
    if (m_backgroundDirty)
        return;

    const auto& config = g_pGlobalState->config;
    const int64_t fps = config.windowsLiveResampleFps ? **config.windowsLiveResampleFps : 0;
    const auto now = std::chrono::steady_clock::now();
    if (fps > 0 && now - m_lastDirtyMark < std::chrono::nanoseconds(1'000'000'000 / fps))
        return;
    m_lastDirtyMark = now;

    m_backgroundDirty = true;
    // damage the full sample region: outside the committed area the framebuffer
    // still holds our previous glass output, which must not be re-sampled
    damageEntire();
}

bool CGlassDecoration::wantsBackgroundResample(PHLMONITOR monitor, const CBox& transformBox, bool xray, const std::vector<Beneath::SRect>& under) const {
    if (sceneChangedSinceSample(monitor, xray))
        return true;

    // Apple's shader keeps its own page: stale once the windows under it moved.
    {
        // the context keeps a reference to the name: it must outlive it
        const std::string     quartzPreset  = resolvePresetName();
        const SResolveContext quartzContext = {quartzPreset, resolveThemeIsDark(), g_pGlobalState->config, g_pGlobalState->customPresets};
        if (Meniscus::resolveParams(quartzContext, 1.0f).quartz != 0)
            return !m_quartzPage || m_quartzClean != !under.empty() ||
                (!under.empty() && (Beneath::signature(under) != m_quartzUnder || Backdrop::stamp(monitor) != m_quartzBackdrop));
    }

    const auto& config = g_pGlobalState->config;
    const auto  source = g_pHyprRenderer->m_renderData.currentFB;
    if (!m_sampleFramebuffer || !source)
        return true;

    // Monitor/scale/DPI/HDR change: recompute the size sampleBackground()
    // would allocate this frame (never cached) and compare against the FBO's
    // actual size/format from the last real sample.
    const bool isDark          = resolveThemeIsDark();
    const std::string preset   = resolvePresetName();
    const SResolveContext ctx  = {preset, isDark, config, g_pGlobalState->customPresets};
    const float blurStrength   = ctx.f(GLASS_F_BLUR_STRENGTH);
    const bool  layered        = Meniscus::resolveParams(ctx, 1.0f).layered.frost_wide > 0.0f;
    const int   downscale     = GlassRenderer::downscaleFor(blurStrength, resolveEnabled() == EEnabledResolution::FrameOnly, layered);

    const int fullWidth  = static_cast<int>(transformBox.w) + 2 * GlassRenderer::SAMPLE_PADDING_PX;
    const int fullHeight = static_cast<int>(transformBox.h) + 2 * GlassRenderer::SAMPLE_PADDING_PX;
    const int requiredWidth  = std::max(1, fullWidth / downscale);
    const int requiredHeight = std::max(1, fullHeight / downscale);

    return m_sampleFramebuffer->m_size.x != requiredWidth || m_sampleFramebuffer->m_size.y != requiredHeight ||
        m_sampleFramebuffer->m_drmFormat != source->m_drmFormat;
}

// Whether what is behind the window may have changed since the last real
// sample, whatever kind of sample that was.
bool CGlassDecoration::sceneChangedSinceSample(PHLMONITOR monitor, bool xray) const {
    const auto& config = g_pGlobalState->config;
    if (!config.windowsBackgroundCache || !**config.windowsBackgroundCache)
        return true; // kill switch off: exact pre-cache behavior, no other state consulted

    if (!m_hasCachedSample)
        return true; // first frame: nothing to reuse yet

    // Per-monitor generation (see m_lastGenerationMonitor's declaration):
    // covers future commit/close-behind-cache invalidation as well as
    // existing event bumps, and forces a resample the instant a window
    // lands on a different monitor even if that monitor's counter happens
    // to numerically match the cached value.
    if (!monitor || monitor->m_id != m_lastGenerationMonitor || g_pGlobalState->getSceneGeneration(monitor) != m_lastSceneGeneration)
        return true;

    if (m_backgroundDirty)
        return true; // markBackgroundDirty() mark not yet escalated into a scene-generation bump

    if (m_cachedFromSnapshot != xray)
        return true; // x-ray switched on or off

    const auto window = m_window.lock();
    if (!window)
        return true;

    // Own move/resize animation, re-checked every call rather than latched
    // once: updateWindow() bumps scene generation only at the animation's
    // start (no per-tick decoration callback exists), so every later frame of
    // the same still-interpolating animation needs this live poll to keep
    // resampling.
    if (window->positionAnimation()->isBeingAnimated() || window->sizeAnimation()->isBeingAnimated())
        return true;

    // A workspace slide or fade moves the whole scene behind us without any
    // geometry change of our own.
    if (WorkspaceAnimation::anyWorkspaceAnimating(monitor))
        return true;

    return false;
}

void CGlassDecoration::renderPass(PHLMONITOR monitor, const float& alpha, bool xray, const std::vector<Beneath::SRect>& under) {
    // Belt and braces: draw() already refuses to queue a pass element for a foreign
    // render, but a caller that reaches renderPass() during one anyway must not sample
    // the foreign framebuffer into this decoration's persistent background cache.
    if (RenderGuards::isForeignRender())
        return;

    auto& shaderManager = g_pGlobalState->shaderManager;
    shaderManager.initializeIfNeeded();

    if (!shaderManager.isInitialized())
        return;

    const auto window = m_window.lock();
    if (!window)
        return;

    const auto source = g_pHyprRenderer->m_renderData.currentFB;
    if (!source)
        return;

    auto optBox = WindowGeometry::computeWindowBox(window, monitor);
    if (!optBox)
        return;

    if (monitor)
        Diagnostics::recordWindowGlassDraw(monitor->m_id);

    CBox windowBox    = *optBox;
    CBox transformBox = WindowGeometry::applyMonitorTransform(windowBox, monitor);

    // Every non-discarded frame needs this regardless of cache hit/miss: a
    // cache-hit frame has no sampleBackground()/blurBackground() call for it
    // to sit after, and reusing a stale value here (e.g. glassAlpha after a
    // fade, or cornerRadius after a fullscreen toggle) would silently
    // mis-render the composite even though the cached sample itself is fine.
    const bool isDark          = resolveThemeIsDark();
    const std::string preset   = resolvePresetName();
    const SResolveContext ctx  = {preset, isDark, g_pGlobalState->config, g_pGlobalState->customPresets};

    float monitorScale = monitor->m_scale;

    // Hyprland renders internal-fullscreen windows unrounded (dontRound), we need to
    // match, or the glass would show rounded gaps at the screen corners
    const bool fsUnrounded = Fullscreen::controller()->getFullscreenModes(window).internal == Fullscreen::FSMODE_FULLSCREEN;
    float cornerRadius  = fsUnrounded ? 0.0f : window->rounding() * monitorScale;
    float roundingPower = window->roundingPower();

    // The pane reaches beyond the window by its border (windows:border), and a
    // band round a rounded window is rounded by that much more, as Hyprland's
    // own border is. Square windows keep square frames.
    const float borderPx  = WindowGeometry::glassBorder(window) * monitorScale;
    if (cornerRadius > 0.0f)
        cornerRadius += borderPx;
    const bool frameOnly  = resolveEnabled() == EEnabledResolution::FrameOnly;

    // A window without the focus has its rim lit less (windows:border_dim),
    // which is how the focused one is told from the others once the border is
    // glass and no longer a colour.
    float rimGain = 1.0f;
    if (borderPx > 0.0f && Desktop::focusState()->window() != window) {
        const auto& dim = g_pGlobalState->config.windowsBorderDim;
        rimGain = dim ? std::clamp(static_cast<float>(**dim), 0.0f, 1.0f) : 1.0f;
    }

    // Resolved here, not inside the cache branch below, so it stays fresh on a
    // cache-hit frame too, when neither sampleBackground() nor blendOwnContent() run.
    m_lastSelfSample = frameOnly ? 0.0f : selfSampleFor(ctx);
    if (m_lastSelfSample > 0.0f && !g_pGlobalState->selfSampleConfigured) {
        // hyprctl keyword emits no config.reloaded, and a setup without layer
        // surfaces has no other place that would notice self_sample turning on
        g_pGlobalState->selfSampleConfigured = true;
        BackgroundDamageObserver::refreshEnabled();
    }

    // The render alpha Hyprland hands decorations is activeInactive * fade.
    // Glass must follow fades (open/close, fullscreen, workspace moves) but
    // not the active/inactive dimming or opacity rules: those make the surface
    // more translucent — revealing more glass — and shouldn't wash out the
    // glass pane itself. Rebuild the fade-only alpha from its components.
    float glassAlpha = window->alphaTotalWithout(Desktop::View::WINDOW_ALPHA_ACTIVE);
    if (const auto workspace = window->m_workspace; workspace && !window->m_pinned)
        glassAlpha *= workspace->m_alpha->value();

    const MONITORID monitorId = monitor ? monitor->m_id : -1; // -1 mirrors Hyprland's own MONITOR_INVALID

    // The current material, where the window's preset asks for it: the sample
    // stays the page as it is and a cloud is made beside it, in place of the blur.
    GlassRenderer::SLayeredPane layered{Meniscus::resolveParams(ctx, monitorScale), m_cloudFramebuffer};
    const bool                  isLayered = layered.params.layered.frost_wide > 0.0f;

    // Apple's own shader, where the window's preset asks for it.
    if (layered.params.quartz != 0) {
        renderQuartz(monitor, source, windowBox, transformBox, glassAlpha, cornerRadius, roundingPower, frameOnly ? borderPx : -1.0f, rimGain, xray,
                     layered.params.quartz, under);
        return;
    }

    if (!wantsBackgroundResample(monitor, transformBox, xray, under)) {
        // Background unchanged since the last real sample — reuse it, skip
        // the most expensive GPU work (blit + blur passes) entirely.
        Diagnostics::recordWindowCacheHit(monitorId);
    } else {
        // X-ray never samples the live frame: it holds the windows it hides.
        const auto snapshot = xray ? Backdrop::get(monitor, source) : nullptr;
        const bool covered  = xray ? static_cast<bool>(snapshot) :
                                     GlassRenderer::sampleRegionCovered(transformBox, source, g_pHyprRenderer->m_renderData.damage, monitor);

        if (covered) {
            float blurStrength   = ctx.f(GLASS_F_BLUR_STRENGTH);
            int downscale        = GlassRenderer::downscaleFor(blurStrength, frameOnly, isLayered);

            GlassRenderer::sampleBackground(m_sampleFramebuffer, xray ? snapshot : source, transformBox, m_samplePaddingRatio, downscale);

            // Only here, on a real (non-cached) sample: blending onto a cache-hit
            // frame would double-composite our own content over an already-blurred FBO.
            if (m_lastSelfSample > 0.0f)
                GlassRenderer::blendOwnContent(m_sampleFramebuffer, window, monitor, transformBox, downscale,
                                               m_lastSelfSample, cornerRadius, roundingPower);

            float blurRadius     = blurStrength * 12.0f / downscale;
            int blurIterations   = std::clamp(static_cast<int>(ctx.i(GLASS_I_BLUR_ITERATIONS)), 1, 5);

            if (ctx.config.blurFold && **ctx.config.blurFold) {
                const GlassRenderer::SFoldedBlur folded = GlassRenderer::foldBlurPasses(blurRadius, blurIterations);
                blurRadius     = folded.radius;
                blurIterations = folded.iterations;
            }

            if (isLayered)
                (void)Meniscus::makeCloud(m_sampleFramebuffer, m_cloudFramebuffer, layered.params, downscale, source);
            else
                GlassRenderer::blurBackground(m_sampleFramebuffer, blurRadius, blurIterations, source);

            m_hasCachedSample       = true;
            m_cachedFromSnapshot    = xray;
            m_lastSceneGeneration   = g_pGlobalState->getSceneGeneration(monitor);
            m_lastGenerationMonitor = monitorId;
            m_backgroundDirty       = false;
            Diagnostics::recordWindowCacheMiss(monitorId);
        } else if (m_hasCachedSample && (!xray || m_cachedFromSnapshot)) {
            // Not enough of the padded box is damaged yet to safely re-sample
            // (would pick up stale pixels outside this frame's damage), or not
            // yet in the x-ray snapshot. Force it into next frame's damage and
            // draw the stale cache for now — a future pass scissors the draw
            // to what's actually damaged.
            damageEntire();
            Diagnostics::recordWindowDeferredResample(monitorId);
            if (xray)
                Diagnostics::recordXrayDeferredResample(monitorId);
        } else {
            // No cache yet (or only a live one under x-ray) and not enough
            // damage to sample cleanly: nothing valid to draw this frame.
            damageEntire();
            if (xray)
                Diagnostics::recordXrayDeferredResample(monitorId);
            return;
        }
    }

    // the cloud this frame made, or the one kept from the last frame that made one
    layered.cloud = m_cloudFramebuffer;

    // A frame is drawn as its strips alone (the rule for them is in the model).
    GlassFrame frame{};
    const bool framed = frameOnly && glass_frame_strips(&static_cast<const GlassRect&>(Glass::toRect(windowBox)), borderPx, cornerRadius, &frame) == GLASS_OK;
    if (frameOnly && !framed)
        return;

    GlassRenderer::applyGlassEffect(m_sampleFramebuffer, source,
                                     windowBox, transformBox, glassAlpha,
                                     std::array<float, 4>{cornerRadius, cornerRadius, cornerRadius, cornerRadius},
                                     roundingPower, m_samplePaddingRatio, ctx, nullptr, framed ? &frame : nullptr, rimGain,
                                     isLayered && layered.cloud ? &layered : nullptr);
}

// With x-ray glass from the backdrop over the whole of a see-through window,
// at full fade, nothing under the window can show: the glass paints every
// pixel of it but the antialiased edge and what the rounding takes off the
// corners. Telling Hyprland so lets it skip the wallpaper, the desktop's cards
// and the windows beneath on every frame the window changes, which is most of
// what a see-through window costs. Only Apple's shader is claimed for: its
// pane is opaque inside its outline, as the classic shader's is not always.
CRegion CGlassDecoration::opaqueRegion(PHLMONITOR monitor, bool xray, bool windowsUnder) const {
    const auto window = m_window.lock();
    // A solid window's frame is drawn alone, and its middle is its own content,
    // opaque: nothing under that can show either, as long as the window is
    // drawn whole (its opacity rule as well as its fade at one).
    const auto resolved = resolveEnabled();
    const bool solid    = resolved == EEnabledResolution::FrameOnly && taggedSolid();
    // Over other windows the glass reads them from the frame: they must be drawn.
    if (!xray || windowsUnder || !window || !monitor || monitor->m_scale <= 0.0f || !Backdrop::possible(monitor) ||
        (resolved != EEnabledResolution::Enabled && !solid))
        return {};

    float alpha = solid ? window->alphaTotal() : window->alphaTotalWithout(Desktop::View::WINDOW_ALPHA_ACTIVE);
    if (const auto workspace = window->m_workspace; workspace && !window->m_pinned)
        alpha *= workspace->m_alpha->value();
    if (alpha < 1.0f)
        return {};

    // the context keeps a reference to the name: it must outlive it
    const std::string     preset = resolvePresetName();
    const SResolveContext ctx    = {preset, resolveThemeIsDark(), g_pGlobalState->config, g_pGlobalState->customPresets};
    if (Meniscus::resolveParams(ctx, monitor->m_scale).quartz == 0)
        return {};

    auto box = WindowGeometry::computeWindowBox(window, monitor);
    if (!box)
        return {};
    // a solid window's own box, inside the frame; a pane's, frame and all
    const double frame = solid ? WindowGeometry::glassBorder(window) : 0.0;
    box->scale(1.0 / monitor->m_scale).expand(-1.0 - frame);

    const bool   fsUnrounded = Fullscreen::controller()->getFullscreenModes(window).internal == Fullscreen::FSMODE_FULLSCREEN;
    const double radius      = fsUnrounded || window->rounding() <= 0.0f ? 0.0 :
                                                                          std::ceil(window->rounding() + WindowGeometry::glassBorder(window) - frame);
    if (box->w <= 2.0 * radius || box->h <= 2.0 * radius)
        return {};

    CRegion region;
    region.add(CBox{box->x + radius, box->y, box->w - 2.0 * radius, box->h});
    region.add(CBox{box->x, box->y + radius, box->w, box->h - 2.0 * radius});
    return region;
}

// A window's pane with Apple's own shader: the page around it, as far as the
// shader reads it, copied with a mip chain when what is behind may have
// changed, and the pane drawn over it with the window's own shape. `band` is
// the frame's width for a solid window (only the frame is drawn), or negative.
void CGlassDecoration::renderQuartz(PHLMONITOR monitor, const SP<Render::IFramebuffer>& source, CBox windowBox, CBox transformBox, float alpha,
                                    float cornerRadius, float roundingPower, float band, float rimGain, bool xray, uint32_t code,
                                    const std::vector<Beneath::SRect>& under) {
    GlassQuartz quartz{};
    if (!Tuning::quartz(code, quartz))
        return;

    Meniscus::SQuartzWindow pane{
        .rawBox        = windowBox,
        .transformBox  = transformBox,
        .cornerRadius  = cornerRadius,
        .roundingPower = roundingPower,
        .band          = band,
        .alpha         = alpha,
        .lightGain     = rimGain,
        .quartz        = quartz,
    };

    // X-ray reads the backdrop as it is: one texture and its mip chain for every
    // window on the monitor, drawn again only when the wallpaper changes. No
    // copy is made for the window, nothing above the wallpaper shows in it, and
    // a window that moves needs nothing new.
    if (xray) {
        if (const auto backdrop = Backdrop::get(monitor, source)) {
            if (!under.empty()) {
                renderQuartzClean(monitor, source, backdrop, pane, under);
                return;
            }
            m_quartzPage.reset();
            m_hasCachedSample = false;
            pane.page         = backdrop;
            pane.pageRect     = CBox{{}, backdrop->m_size};
            pane.blurred      = Backdrop::blurred(monitor, source, quartz.blur * static_cast<float>(monitor->m_scale));
            Meniscus::drawQuartzWindow(source, pane);
            return;
        }
        xray = false;
    }

    const MONITORID monitorId = monitor ? monitor->m_id : -1;
    // the shader's lengths are points
    const double    scale     = monitor && monitor->m_scale > 0.0f ? monitor->m_scale : 1.0;
    const CBox      wanted    = transformBox.copy().expand(std::ceil(quartz.reach * scale) + 2.0).intersection(CBox{{}, source->m_size});
    const double    x         = std::floor(wanted.x);
    const double    y         = std::floor(wanted.y);
    const CBox      copyRect  = {x, y, std::ceil(wanted.x + wanted.w) - x, std::ceil(wanted.y + wanted.h) - y};

    const bool stale = !m_quartzPage || m_quartzClean || copyRect != m_quartzRect || sceneChangedSinceSample(monitor, xray);
    if (!stale) {
        Diagnostics::recordWindowCacheHit(monitorId);
    } else {
        const bool covered = GlassRenderer::sampleRegionCovered(copyRect, source, g_pHyprRenderer->m_renderData.damage, monitor);
        if (covered && Meniscus::copyWithMips(m_quartzPage, source, copyRect, source)) {
            m_quartzRect            = copyRect;
            m_quartzClean           = false;
            m_hasCachedSample       = true;
            m_cachedFromSnapshot    = false;
            m_lastSceneGeneration   = g_pGlobalState->getSceneGeneration(monitor);
            m_lastGenerationMonitor = monitorId;
            m_backgroundDirty       = false;
            Diagnostics::recordWindowCacheMiss(monitorId);
        } else {
            // what is behind is not all repainted yet: ask for it, and draw the last copy meanwhile
            damageEntire();
            Diagnostics::recordWindowDeferredResample(monitorId);
            if (!m_quartzPage)
                return;
        }
    }

    pane.page     = m_quartzPage;
    pane.pageRect = m_quartzRect;
    Meniscus::drawQuartzWindow(source, pane);
}

// The windows drawn before this one whose boxes meet what its glass reads: the
// window and, round it, the furthest Apple's shader displaces or blurs. None
// when the user turned windows through glass off, or the glass is not Apple's
// shader (the classic one keeps reading the backdrop alone).
std::vector<Beneath::SRect> CGlassDecoration::windowsUnder(PHLMONITOR monitor) const {
    const auto window = m_window.lock();
    if (!window || !monitor || !Tuning::seeWindows())
        return {};
    // the context keeps a reference to the name: it must outlive it
    const std::string     preset = resolvePresetName();
    const SResolveContext ctx    = {preset, resolveThemeIsDark(), g_pGlobalState->config, g_pGlobalState->customPresets};
    const uint32_t        code = Meniscus::resolveParams(ctx, monitor->m_scale).quartz;
    GlassQuartz           quartz{};
    if (code == 0 || !Tuning::quartz(code, quartz))
        return {};
    const auto box = WindowGeometry::computeWindowBox(window, monitor);
    if (!box)
        return {};
    const CBox area = WindowGeometry::applyMonitorTransform(*box, monitor).expand(std::ceil(quartz.reach * monitor->m_scale) + 2.0);
    return Beneath::under(monitor, area, window);
}

// The pane over other windows. Its page is copied from the frame, as live glass
// is, but with the backdrop wherever none of those windows lies (Beneath.hpp),
// and blurred for the face as the backdrop is. It is made again only when what
// is under it moved or changed, and it waits for the frame to be repainted
// across the whole of it: meanwhile the last page is drawn, or, before the
// first, the backdrop alone.
void CGlassDecoration::renderQuartzClean(PHLMONITOR monitor, const SP<Render::IFramebuffer>& source, const SP<Render::IFramebuffer>& backdrop,
                                         Meniscus::SQuartzWindow& pane, const std::vector<Beneath::SRect>& under) {
    const MONITORID monitorId = monitor->m_id;
    const double    scale     = monitor->m_scale > 0.0f ? monitor->m_scale : 1.0;
    const CBox      wanted    = pane.transformBox.copy().expand(std::ceil(pane.quartz.reach * scale) + 2.0).intersection(CBox{{}, source->m_size});
    const double    x         = std::floor(wanted.x);
    const double    y         = std::floor(wanted.y);
    const CBox      copyRect  = {x, y, std::ceil(wanted.x + wanted.w) - x, std::ceil(wanted.y + wanted.h) - y};
    const uint64_t  signature = Beneath::signature(under);
    const float     blur      = pane.quartz.blur * static_cast<float>(scale);
    // The page has the wallpaper in it wherever no window lies under it: a new
    // wallpaper makes it stale too (2026-10-23: the window under the pointer
    // kept the old one until a change of focus made the page again).
    const uint64_t  wallpaper = Backdrop::stamp(monitor);

    const bool stale = !m_quartzPage || !m_quartzClean || copyRect != m_quartzRect || signature != m_quartzUnder || wallpaper != m_quartzBackdrop ||
        sceneChangedSinceSample(monitor, true);
    if (!stale) {
        Diagnostics::recordWindowCacheHit(monitorId);
    } else if (CRegion(copyRect).subtract(g_pHyprRenderer->m_renderData.damage).empty() &&
               Meniscus::copyClean(m_quartzPage, Meniscus::SCleanSource{source, backdrop, under}, copyRect, source)) {
        if (!(blur >= 1.0f) || !Backdrop::blurInto(m_quartzPage, m_quartzBlur, blur, source))
            m_quartzBlur.reset();
        m_quartzRect            = copyRect;
        m_quartzClean           = true;
        m_quartzUnder           = signature;
        m_quartzBackdrop        = wallpaper;
        m_hasCachedSample       = true;
        m_cachedFromSnapshot    = true;
        m_lastSceneGeneration   = g_pGlobalState->getSceneGeneration(monitor);
        m_lastGenerationMonitor = monitorId;
        m_backgroundDirty       = false;
        Diagnostics::recordWindowCacheMiss(monitorId);
    } else {
        // not all of it is drawn this frame: ask for all of it
        g_pHyprRenderer->damageBox(copyRect.copy().scale(1.0 / scale).translate(monitor->m_position));
        damageEntire();
        Diagnostics::recordWindowDeferredResample(monitorId);
        Diagnostics::recordXrayDeferredResample(monitorId);
    }

    if (m_quartzPage && m_quartzClean) {
        pane.page     = m_quartzPage;
        pane.pageRect = m_quartzRect;
        pane.blurred  = blur >= 1.0f ? m_quartzBlur : nullptr;
    } else {
        pane.page     = backdrop;
        pane.pageRect = CBox{{}, backdrop->m_size};
        pane.blurred  = Backdrop::blurred(monitor, source, blur);
    }
    Meniscus::drawQuartzWindow(source, pane);
}

bool CGlassDecoration::commitMatters(const PHLWINDOW& committing) const {
    if (!m_lastXray)
        return true;
    return Beneath::contains(m_lastUnder, committing);
}

eDecorationType CGlassDecoration::getDecorationType() {
    return DECORATION_CUSTOM;
}

// Driven by the real position/size variables and by map/layout/rule changes,
// unlike draw(), which sees whatever geometry the caller substituted. The
// PHLWINDOW argument is ignored on purpose: a third-party replay passes its own
// handle, while our state is keyed on the owner we were constructed with.
void CGlassDecoration::updateWindow(PHLWINDOW) {
    // A plugin may replay this mid-render under substituted geometry. mainFB is
    // set exactly between begin() and end(); currentFB also follows binds taken
    // outside a pass. Above the m_last* store, so the next real update bumps.
    if (g_pHyprRenderer->m_renderData.mainFB)
        return;

    damageEntire();

    if (!g_pGlobalState)
        return;
    if (const auto resolution = resolveEnabled(); resolution != EEnabledResolution::Enabled && resolution != EEnabledResolution::FrameOnly)
        return;

    const auto ownWindow = m_window.lock();
    if (!ownWindow)
        return;

    const auto monitor = ownWindow->m_monitor.lock();
    if (!monitor)
        return;

    const auto currentPosition = ownWindow->position(Desktop::View::IGeometric::GEOMETRIC_CURRENT);
    const auto currentSize     = ownWindow->size(Desktop::View::IGeometric::GEOMETRIC_CURRENT);
    if (currentPosition == m_lastPosition && currentSize == m_lastSize)
        return;

    m_lastPosition = currentPosition;
    m_lastSize     = currentSize;

    // A workspace the monitor is not rendering changes nothing behind a glassed
    // layer; switching to it bumps on its own. Same predicate Hyprland renders
    // by: a slide or fade keeps drawing an already-invisible workspace.
    const auto workspace = ownWindow->m_workspace;
    if (workspace && !workspace->m_visible && !workspace->m_forceRendering && !workspace->m_renderOffset->isBeingAnimated() &&
        !workspace->m_alpha->isBeingAnimated() && !ownWindow->m_pinned)
        return;

    g_pGlobalState->bumpSceneGeneration(monitor);
}

void CGlassDecoration::damageEntire() {
    const auto window = m_window.lock();
    if (!window)
        return;

    // Padded so the render pass re-renders background content (wallpaper,
    // other windows) in the sampling margin too. Without this, the scissored
    // render pass leaves stale previous-frame content in the padding area,
    // causing noise artifacts.
    const auto box = WindowGeometry::computePaddedGlobalBox(window, GlassRenderer::SAMPLE_PADDING_PX);
    if (!box)
        return;

    g_pHyprRenderer->damageBox(*box);
}

eDecorationLayer CGlassDecoration::getDecorationLayer() {
    return DECORATION_LAYER_BOTTOM;
}

uint64_t CGlassDecoration::getDecorationFlags() {
    return DECORATION_NON_SOLID;
}

std::string CGlassDecoration::getDisplayName() {
    return "HyprGlass";
}
