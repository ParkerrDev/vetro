#pragma once

#include "GlassRenderer.hpp"
#include "Meniscus.hpp"
#include "PluginConfig.hpp"

#include <chrono>
#include <memory>
#include <optional>
#include <hyprland/src/desktop/view/LayerSurface.hpp>
#include <hyprland/src/render/Framebuffer.hpp>
#include <hyprutils/math/Region.hpp>

class CGlassLayerSurface {
  public:
    explicit CGlassLayerSurface(PHLLS layerSurface);
    ~CGlassLayerSurface();

    // Where the glass mask for this layer comes from this frame.
    enum class EMaskSource { ALPHA_THRESHOLD, PROTOCOL_REGION, NONE };

    // Phase 1 (pre-surface): sample+blur background, redirect currentFB → temp FBO
    void sampleAndRedirect(PHLMONITOR monitor, float alpha, bool xray);

    // Resolved once per frame where the pass element is queued. Never for a
    // background or bottom layer: they render before the snapshot is taken.
    [[nodiscard]] bool resolveXray() const;

    // The x-ray snapshot holds everything this layer would sample.
    [[nodiscard]] bool xraySnapshotCovers(PHLMONITOR monitor) const;

    // Phase 2 (post-surface): restore currentFB, apply glass masked by temp FBO, blit surface
    void compositeAndRestore(PHLMONITOR monitor, float alpha, EMaskSource maskSource);

    void damageIfMoved();

    // Content below committed damage in our sample region — resample next frame
    void markBackgroundDirty();

    // ── Lens model (docs/model.md), implemented in GlassLayerModel.cpp ──────
    // True when this layer's preset asks for the lens model and its buffer can
    // be read as the mask. Otherwise the classic path above draws it.
    [[nodiscard]] bool usesModel(PHLMONITOR monitor) const;

    // Told once per frame, where the pass elements are queued, which path draws this layer.
    void setModelActive(bool active);
    [[nodiscard]] bool modelActive() const noexcept { return m_modelActive; }

    // Draws the model's glass under where the layer's own pixels are about to go.
    // `under`: the windows drawn before the layer (Beneath.hpp); x-ray panes over
    // one of them read the clean page, the others the backdrop.
    void drawModel(PHLMONITOR monitor, float alpha, bool xray, const std::vector<Beneath::SRect>& under);

    // The layer committed new pixels. surfaceBox is what it redrew, in logical
    // pixels of its root surface; none when that is not known.
    void markContentChanged(const std::optional<CBox>& surfaceBox);

    // Something was committed under damagedGlobalBox (global logical pixels):
    // the panes that read from there are resampled. False when this layer is
    // not drawn by the lens model, and the caller's classic test applies.
    [[nodiscard]] bool markBackgroundDirtyWithin(const CBox& damagedGlobalBox);

    // Pixels beyond the layer's box that its glass reads or draws on.
    [[nodiscard]] int paddingPx() const;

    // Called by the model's timer, outside a frame: reads what the GPU has answered
    // since the layer was last drawn. True while this layer is still waiting.
    [[nodiscard]] bool pollModel();

    // One line per pane the lens model sees on this layer, for `hyprctl hyprglass panes`.
    [[nodiscard]] std::string describeModel() const;

    [[nodiscard]] bool liveResampleEnabled() const;

    // Decides ALPHA_THRESHOLD vs PROTOCOL_REGION vs NONE for this layer, based on
    // mask_mode and the root surface's ext-background-effect-v1 state.
    [[nodiscard]] EMaskSource resolveMaskSource() const;

    [[nodiscard]] PHLLS getLayerSurface() const;

    // PROTOCOL_REGION only: the blur region's bounding box in the same transformed
    // monitor-pixel space as transformBox/regionRects (matches transformedBlurRegion()
    // below). layerBox is the caller's already-computed LayerGeometry::computeLayerBox()
    // result (both call sites have one in hand; avoids recomputing it here). Used to
    // shrink the sample blit box, never the composite draw or boundingBox() (both stay
    // full-layer — see GlassLayerSurface.cpp). Nullopt when the transformed region is empty.
    [[nodiscard]] std::optional<CBox> regionBoundingBoxAbsolute(PHLMONITOR monitor, const CBox& layerBox) const;

    // PROTOCOL_REGION only: the blur region's bounding box in global-logical
    // coordinates — the family BackgroundDamageObserver's damagedBox uses (no
    // monitor scale/transform: logical space needs none). Deliberately a
    // separate coordinate family from regionBoundingBoxAbsolute() above; reusing
    // that monitor-pixel-space box against damagedBox would be silently wrong at
    // any scale != 1 or monitor position != (0,0).
    [[nodiscard]] std::optional<CBox> regionBoundingBoxGlobal() const;

  private:
    PHLLSREF     m_layerSurface;
    SP<Render::IFramebuffer> m_sampleFramebuffer;
    SP<Render::IFramebuffer> m_surfaceTempFramebuffer;
    Vector2D     m_samplePaddingRatio;
    bool         m_hasCachedSample = false;
    bool         m_backgroundDirty = false;
    std::chrono::steady_clock::time_point m_lastDirtyMark{};

    // Set at the end of sampleAndRedirect() when currentFB was actually redirected
    // to the temp FBO this frame, cleared by compositeAndRestore() after it reads it.
    // Guards against compositing against a stale temp FBO when the render pass
    // discarded the pre-surface element (see disableSimplification() in
    // GlassLayerPassElement.cpp).
    bool         m_redirectedThisFrame = false;

    void damageSampleRegion();

    // The cached sample came from the x-ray snapshot: switching x-ray on or off
    // must not reuse a sample of the other.
    bool m_cachedFromSnapshot = false;

    // Track last position/size to detect movement and expand damage
    Vector2D     m_lastPosition;
    Vector2D     m_lastSize;

    // Scene generation at last blur — skip re-sampling when only the layer
    // surface content changed (e.g. clock tick) but the background didn't.
    uint64_t     m_lastSceneGeneration = 0;

    // Transformed blur region computed by sampleAndRedirect() this frame
    // (PROTOCOL_REGION only, empty otherwise), reused by compositeAndRestore()'s
    // regionRects mask upload so the scale/translate/transform sequence doesn't
    // run twice per frame. compositeAndRestore() recomputes via
    // transformedBlurRegion() as a defensive fallback if this is empty when it
    // shouldn't be (mask source disagreeing between the two calls in one frame).
    CRegion      m_cachedTransformedRegion;

    // Region bounding box computed by sampleAndRedirect() this frame (same
    // coordinate space as m_cachedTransformedRegion's extents, PROTOCOL_REGION
    // only, nullopt otherwise). Reused by compositeAndRestore() to build the
    // sampleUVOffset/uvScale mask uniforms that reconcile sampleBackground()'s
    // region-shrunk sample box against the full-layer quad UV (see
    // toSampleBoxUV() in Shaders.hpp) — without this, sampleBlurred() would
    // index the (now smaller) sample texture as if it still covered the whole
    // layer, warping the visible blur. Also compared frame-to-frame in
    // sampleAndRedirect() to force a resample when the region itself moves or
    // resizes with no other invalidation trigger (a layer's own commit doesn't
    // mark itself dirty).
    std::optional<CBox> m_cachedRegionSampleBox;

    // Surface-local logical blur region intersected to the layer's own size,
    // then scaled/translated/transformed into transformBox's pixel space —
    // shared by regionBoundingBoxAbsolute() and compositeAndRestore()'s
    // regionRects upload so both use identical, proven math.
    [[nodiscard]] CRegion transformedBlurRegion(PHLMONITOR monitor, const CBox& rawBox) const;

    // Saved currentFB pointer, restored in compositeAndRestore
    SP<Render::IFramebuffer> m_savedCurrentFB;

    [[nodiscard]] bool           resolveThemeIsDark() const;
    [[nodiscard]] std::string    resolvePresetName() const;
    [[nodiscard]] ELayerMaskMode resolveMaskMode() const;

    // Lens model state. The field, the panes and their frost live in m_model,
    // which exists only while the model draws this layer.
    std::unique_ptr<Meniscus::CLayer> m_model;
    GlassParams                       m_modelParams{};
    bool                              m_modelActive = false;

    void               resolveMaskLimits(float& threshold, float& feather) const;
    // Resamples the panes whose background changed, where this frame's damage allows it.
    void               sampleModelPanes(PHLMONITOR monitor, const SP<Render::IFramebuffer>& source, const CBox& layerBox, bool xray,
                                        const std::vector<Beneath::SRect>& under);
    void               damageModelPanes();
    void               damagePanes(const std::vector<Meniscus::SPane>& panes);

    // A change behind the layer that arrived too soon after the last one to be
    // acted on: what it covered, kept until the pacing interval has passed, so
    // that the last change of a burst is never the one that is lost.
    std::optional<CBox> m_pendingBackgroundMark;
    [[nodiscard]] CBox globalLogicalBox(const CBox& monitorPixels, const PHLMONITOR& monitor) const;
};
