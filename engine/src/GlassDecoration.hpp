#pragma once

#include "Beneath.hpp"
#include "GlassRenderer.hpp"
#include "Meniscus.hpp"
#include "PluginConfig.hpp"

#include <chrono>
#include <hyprland/src/desktop/view/Window.hpp>
#include <hyprland/src/render/decorations/IHyprWindowDecoration.hpp>
#include <hyprland/src/render/Framebuffer.hpp>
#include <hyprland/src/SharedDefs.hpp>

class CGlassDecoration : public IHyprWindowDecoration {
  public:
    explicit CGlassDecoration(PHLWINDOW window);
    ~CGlassDecoration() override;

    [[nodiscard]] SDecorationPositioningInfo getPositioningInfo() override;
    void                                     onPositioningReply(const SDecorationPositioningReply& reply) override;
    void                                     draw(PHLMONITOR monitor, float const& alpha) override;
    [[nodiscard]] eDecorationType            getDecorationType() override;
    void                                     updateWindow(PHLWINDOW window) override;
    void                                     damageEntire() override;
    [[nodiscard]] eDecorationLayer           getDecorationLayer() override;
    [[nodiscard]] uint64_t                   getDecorationFlags() override;
    [[nodiscard]] std::string                getDisplayName() override;

    [[nodiscard]] PHLWINDOW getOwner();
    // `under`: the windows drawn before this one that its glass reads (Beneath.hpp).
    void                    renderPass(PHLMONITOR monitor, const float& alpha, bool xray, const std::vector<Beneath::SRect>& under);
    void                    onFullscreenStateChanged();

    // Content below committed damage in our sample region — resample next frame.
    // Public: a future commit-driven invalidation pass calls this on any
    // decoration whose padded box overlaps a commit from outside the
    // window's own surface tree.
    void markBackgroundDirty();

    // Pure predicate: true when the cached sample must not be reused as-is
    // this frame. Called identically from CGlassPassElement::needsLiveBlur()
    // (draw()-time) and from renderPass() itself — safe to call twice, no GL
    // calls. transformBox is monitor-local physical pixels (see callers).
    [[nodiscard]] bool wantsBackgroundResample(PHLMONITOR monitor, const CBox& transformBox, bool xray, const std::vector<Beneath::SRect>& under) const;
    [[nodiscard]] bool sceneChangedSinceSample(PHLMONITOR monitor, bool xray) const;
    void renderQuartz(PHLMONITOR monitor, const SP<Render::IFramebuffer>& source, CBox windowBox, CBox transformBox, float alpha, float cornerRadius,
                      float roundingPower, float band, float rimGain, bool xray, uint32_t code, const std::vector<Beneath::SRect>& under);
    // The pane over other windows: the clean page of the frame (Beneath.hpp).
    void renderQuartzClean(PHLMONITOR monitor, const SP<Render::IFramebuffer>& source, const SP<Render::IFramebuffer>& backdrop, Meniscus::SQuartzWindow& pane,
                           const std::vector<Beneath::SRect>& under);

    // Whether a commit of `committing` (null for anything but a window) can
    // change what this window's glass shows: x-ray glass reads the backdrop and,
    // over other windows, those windows alone.
    [[nodiscard]] bool commitMatters(const PHLWINDOW& committing) const;
    // How many windows the last frame's glass read under this one, for `hyprctl hyprglass status`.
    [[nodiscard]] size_t lastUnderCount() const { return m_lastUnder.size(); }

    // What this window's glass paints whole, in monitor-local logical
    // coordinates, for Hyprland to skip drawing under it (IPassElement::
    // opaqueRegion). Empty unless that is certain this frame.
    [[nodiscard]] CRegion opaqueRegion(PHLMONITOR monitor, bool xray, bool windowsUnder) const;
    // Tagged hyprglass_solid: its own content is opaque (PluginConfig.hpp).
    [[nodiscard]] bool taggedSolid() const;

    // Owner test without the shared_ptr copy getOwner() hands out.
    [[nodiscard]] bool  ownsWindow(const PHLWINDOW& window) const { return m_window == window; }
    // self_sample as of the last rendered frame, 0 when the glass is off. Read on
    // every watched surface commit, so it must not walk the preset chain.
    [[nodiscard]] float lastSelfSample() const { return m_lastSelfSample; }

    // Public wrappers around the private tag/config resolution below, so other
    // glass consumers that key off this window (subsurface item glass — see
    // GlassSubsurfaceState.cpp) apply exactly the same enable/disable rules and
    // theme/preset tags as the window's own glass, without duplicating them.
    [[nodiscard]] bool        isGlassEnabled() const { return resolveEnabled() == EEnabledResolution::Enabled; }
    [[nodiscard]] bool        isThemeDark() const { return resolveThemeIsDark(); }
    [[nodiscard]] std::string presetName() const { return resolvePresetName(); }

    // Weak over the UP Hyprland owns: use .get()/-> only, never .lock().
    WP<CGlassDecoration> m_self;

  private:
    PHLWINDOWREF m_window;
    SP<Render::IFramebuffer> m_sampleFramebuffer;
    // the current material: the wide soft copy that goes with the sample
    SP<Render::IFramebuffer> m_cloudFramebuffer;
    // Apple's own shader: the page around the window with its mip chain, and
    // the monitor pixels it holds
    SP<Render::IFramebuffer> m_quartzPage;
    CBox                     m_quartzRect;
    // a clean page's own blur for the face, and what the page was made of: the
    // clean frame (and the windows under it, by Beneath::signature) or the live one
    SP<Render::IFramebuffer> m_quartzBlur;
    bool                     m_quartzClean = false;
    uint64_t                 m_quartzUnder = 0;
    // The wallpaper the clean page was made over (Backdrop::stamp): a new
    // one makes the page stale, as windows moving under it do.
    uint64_t                 m_quartzBackdrop = 0;
    // what the last frame's glass read, for commitMatters()
    bool                        m_lastXray = false;
    std::vector<Beneath::SRect> m_lastUnder;
    [[nodiscard]] std::vector<Beneath::SRect> windowsUnder(PHLMONITOR monitor) const;
    Vector2D     m_samplePaddingRatio;

    // Last geometry seen in updateWindow(), to detect real moves/resizes
    Vector2D m_lastPosition;
    Vector2D m_lastSize;

    // Window background cache state.
    bool m_hasCachedSample = false;
    bool m_backgroundDirty = false;
    // The cached sample came from the x-ray snapshot: switching x-ray on or off
    // must not reuse a sample of the other.
    bool m_cachedFromSnapshot = false;
    std::chrono::steady_clock::time_point m_lastDirtyMark{};

    // Scene generation at the last real sample, plus the monitor it was
    // captured against. SGlobalState::sceneGeneration is deliberately keyed
    // per-MONITORID (see Globals.hpp), so a bare generation compare could
    // alias a value from a different monitor's independent counter sequence
    // when this window is dragged across monitors — storing the monitor
    // alongside the generation and treating any mismatch as "wants resample"
    // closes that gap.
    uint64_t  m_lastSceneGeneration   = 0;
    MONITORID m_lastGenerationMonitor = 0;

    // Whether we currently hold the noblur window property on our window.
    // Glass replaces Hyprland's blur, and Hyprland's cached-blur optimization
    // (blur:new_optimizations) composites translucent windows against a
    // snapshot taken before plugin decorations render — without noblur the
    // glass is invisible on static windows (#46).
    bool m_noBlurApplied = false;

    // Glass can start without a config reload (tags, hyprctl keyword), and
    // each start rechecks that Hyprland shadows are on.
    bool m_glassWasEnabled = false;

    float m_lastSelfSample = 0.0f;

    // Frame serial the last glass element was queued for, and its index in that
    // frame. Hyprland renders a floating window over fullscreen more than once
    // per frame, and only the last glass may sample and blur.
    uint64_t m_glassFrameSerial = 0;
    uint32_t m_glassQueueIndex  = 0;

    // Frame serial this window last folded into the monitor's render-order
    // fingerprint. draw() itself can run more than once per monitor frame for
    // the same window (floating over fullscreen), and only the first copy may
    // fold — a second fold in the same frame would make the hash depend on how
    // many duplicate copies happened to render, not on the scene.
    uint64_t m_lastFoldedFrameSerial = 0;

    void               queueGlassPass(float alpha, bool xray, std::vector<Beneath::SRect> under);
    [[nodiscard]] bool isCurrentGlassPass(uint64_t serial, uint32_t index) const {
        return serial == 0 || (serial == m_glassFrameSerial && index == m_glassQueueIndex);
    }

    void updateNoBlurProp(bool glassEnabled);
    void withdrawNoBlur();

    // The reason resolveEnabled() reached its answer, so callers can
    // attribute a Diagnostics counter to the actual cause instead of
    // re-deriving "is it opaque" independently — a window can be opaque and
    // still disabled for another reason (tag, global config) that took
    // precedence, and re-deriving would mis-attribute those to opaque-skip.
    enum class EEnabledResolution {
        Enabled,
        Disabled,              // hyprglass_disabled tag, or plugin:hyprglass:enabled = 0
        DisabledBecauseOpaque, // skip_opaque_windows and the window is opaque
        FrameOnly,             // windows:border, and the window gets no pane: its frame alone
    };

    [[nodiscard]] EEnabledResolution resolveEnabled() const;
    [[nodiscard]] bool               resolveXray() const;
    [[nodiscard]] bool               resolveThemeIsDark() const;
    [[nodiscard]] std::string        resolvePresetName() const;

    friend class CGlassPassElement;
};
