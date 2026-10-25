#pragma once

#include "Beneath.hpp"
#include "GlassBridge.hpp"
#include "PluginConfig.hpp"

#include <array>
#include <chrono>
#include <GLES3/gl32.h>
#include <hyprland/src/desktop/DesktopTypes.hpp>
#include <hyprland/src/render/Framebuffer.hpp>
#include <hyprland/src/render/Shader.hpp>
#include <hyprland/src/render/Texture.hpp>
#include <hyprutils/math/Box.hpp>
#include <hyprutils/math/Region.hpp>
#include <optional>
#include <string>
#include <unordered_map>
#include <vector>

// The meniscus pipeline: glass drawn from the lens model of docs/model.md, on
// panes of any outline. A pane is whatever a layer drew above the mask
// threshold, so a card, a menu or a capsule inside a larger see-through surface
// is a pane of its own, with its own rim.
//
// Three things are kept per layer, and each is redone only when its own input
// changed:
//   the field   what the outline does to light, from the layer's own alpha
//               (redone when the layer commits new content)
//   the frost   a reduced, blurred copy of what lies behind each pane
//               (redone when something behind that pane changed)
//   the draw    one quad per pane under the layer's pixels
//               (redone when Hyprland repaints that part of the screen)
namespace Meniscus {

// True when the preset this context resolves asks for the lens model.
[[nodiscard]] bool presetUsesModel(const SResolveContext& context);
// What the context's preset resolves to for a monitor of this scale, every length in device pixels.
[[nodiscard]] GlassParams resolveParams(const SResolveContext& context, float monitorScale);

// The wide soft copy of the current material for a pane that is not a layer's
// (a window's): `page` is what is behind the pane, already `downscale` times
// smaller than the screen, and `cloud` gets the copy, an eighth of the
// screen's resolution where the page allows it. False when it could not be made.
bool makeCloud(const SP<Render::IFramebuffer>& page, SP<Render::IFramebuffer>& cloud, const GlassParams& params, int downscale,
               const SP<Render::IFramebuffer>& caller);

// Copies `rect` of `source` (whole pixels) into `page` and gives it a mip chain,
// which Apple's shader blurs by reading. False when there was nothing to copy.
bool copyWithMips(SP<Render::IFramebuffer>& page, const SP<Render::IFramebuffer>& source, const CBox& rect, const SP<Render::IFramebuffer>& caller);

// What glass with windows under it reads (Beneath.hpp): the frame being drawn,
// its backdrop, and the windows of the frame under the glass.
struct SCleanSource {
    SP<Render::IFramebuffer>    live;
    SP<Render::IFramebuffer>    backdrop;
    std::vector<Beneath::SRect> under;
};

// The clean page: `rect` of the live frame where the windows under the glass lie
// and of the backdrop everywhere else (quartzclean.frag), into `page` with a mip
// chain. False when there was nothing to copy.
bool copyClean(SP<Render::IFramebuffer>& page, const SCleanSource& source, const CBox& rect, const SP<Render::IFramebuffer>& caller);

// A window's pane for Apple's shader: the page around it, the window's boxes as
// Hyprland draws it (raw, and in the monitor's framebuffer), its corners, and
// for a solid window the width of the frame that alone is drawn (else negative).
struct SQuartzWindow {
    SP<Render::IFramebuffer> page;
    CBox                     pageRect;
    // the page blurred for the face already (the backdrop's), or none
    SP<Render::IFramebuffer> blurred;
    CBox                     rawBox;
    CBox                     transformBox;
    float                    cornerRadius  = 0.0f;
    float                    roundingPower = 2.0f;
    float                    band          = -1.0f;
    float                    alpha         = 1.0f;
    float                    lightGain     = 1.0f;
    GlassQuartz              quartz{};
};
void drawQuartzWindow(const SP<Render::IFramebuffer>& target, const SQuartzWindow& pane);

// A shader program with its uniform locations looked up by name once.
class CProgram {
  public:
    [[nodiscard]] bool  create(const char* fragmentName);
    void                destroy();
    [[nodiscard]] GLint uniform(const char* name);

    SP<CShader> shader = makeShared<CShader>();

  private:
    std::unordered_map<std::string, GLint> m_locations;
};

struct SPrograms {
    CProgram coverage;
    CProgram outline;
    CProgram fieldBlur;
    CProgram field;
    CProgram cells;
    CProgram shadowBlur;
    CProgram reduce;
    CProgram frostBlur;
    CProgram shadow;
    CProgram face;
    CProgram glass;
    CProgram quartz;
    CProgram quartzSeed;
    CProgram quartzFlood;
    CProgram quartzClean;
    CProgram changes;
    GLuint   quartzBlock = 0; // the uniform buffer of Apple's shader

    [[nodiscard]] bool ready() const noexcept { return m_ready; }
    [[nodiscard]] bool failed() const noexcept { return m_failed; }
    void               initializeIfNeeded();
    void               destroy();

  private:
    bool m_ready  = false;
    bool m_failed = false;
};

// Framebuffers every layer borrows while it works, one after another. They only
// ever grow, so layers of different sizes do not make them reallocate each frame.
struct SScratch {
    SP<Render::IFramebuffer> maskA;   // field resolution
    SP<Render::IFramebuffer> maskB;
    SP<Render::IFramebuffer> shadowA; // shadow resolution
    SP<Render::IFramebuffer> shadowB;
    SP<Render::IFramebuffer> blur;    // the screen's format: the frost blur's other side
    SP<Render::IFramebuffer> cloud;   // with an alpha channel: the other side of a weighted blur
    SP<Render::IFramebuffer> seedA;   // field resolution: the two sides of the outline's jump flood
    SP<Render::IFramebuffer> seedB;

    void release();
};

// One piece of glass: a connected patch of the layer that counts as pane.
struct SPane {
    CBox                     box;       // its extent, device pixels relative to the layer box
    double                   shortSide = 0.0; // of the pane itself, device pixels: what the lens is fitted to
    // Rectangles inside the pane, relative to the layer box, where every pixel is
    // face only: deep inside, clear of the rim. Drawn with one read per pixel.
    std::vector<CBox>        interiors;
    SP<Render::IFramebuffer> frost;     // what lies behind it, reduced and blurred
    CBox                     sampled;   // device pixels of the monitor the frost holds
    int                      reduction = 1;
    bool                     faceApplied = false; // the frost holds the face already
    // The current material shows the page twice: `frost` is then the wide copy
    // alone, the cloud, and this is the page as it is, for the narrow one.
    SP<Render::IFramebuffer> sharp;
    CBox                     sharpRect; // device pixels of the monitor it holds
    // Apple's own shader (GlassParams.quartz) reads the page around the pane
    // through a mip chain: this copy, and the monitor pixels it holds.
    SP<Render::IFramebuffer> page;
    CBox                     pageRect;
    // the page blurred for the face: the backdrop's, or this pane's own
    SP<Render::IFramebuffer> blurred;
    // a clean page's own blur (the backdrop's is shared, and never written)
    SP<Render::IFramebuffer> ownBlur;
    bool                     layered = false;
    bool                     hasSample = false;
    bool                     dirty     = true; // something behind it changed since it was sampled
};

// What a layer drew, as the mask reads it.
struct SMaskSource {
    SP<Render::ITexture> texture;
    Vector2D             uvScale{1.0, 1.0}; // box uv to texture uv
    float                threshold = 0.001f;
    float                feather   = 0.0f;
};

// Where the engine's time goes and how often it does what, for `hyprctl hyprglass stats`.
struct SLayerCounters {
    uint64_t fieldBuilds   = 0;
    uint64_t paneReadbacks = 0;
};

class CLayer {
  public:
    CLayer();
    ~CLayer();
    CLayer(const CLayer&)            = delete;
    CLayer& operator=(const CLayer&) = delete;

    // The layer committed new pixels, which may or may not have moved its
    // outline. box is what it redrew, device pixels relative to the layer box;
    // none means all of it.
    void contentChanged(const std::optional<CBox>& box);

    // True once after update() when it is waiting for the GPU's answer or for its
    // own pacing: poll() should then be called a few milliseconds later.
    [[nodiscard]] bool takePollRequest() noexcept;

    // What the layer is waiting for, asked outside a frame with the GL context current.
    enum class EWaiting {
        NOTHING, // nothing pending
        ANSWER,  // the GPU has not said yet whether the outline moved, or a rebuild is being paced: ask again
        FRAME,   // update() has work to do: the layer needs to be drawn
    };
    [[nodiscard]] EWaiting poll();

    // Set by update() when it rebuilt the field after the outline moved: the part
    // of the layer box, in device pixels relative to it, whose glass is now
    // different from what is on screen.
    [[nodiscard]] std::optional<CBox> takeRepaint() noexcept;

    // True once after a new map of the panes was read: the panes update() sets
    // next, and the lens fitted to them, differ from what is on screen.
    [[nodiscard]] bool takeMapAdopted() noexcept;

    // The panes of the last map read, which the next update() makes current.
    [[nodiscard]] const std::vector<SPane>& knownPanes() const noexcept { return m_knownPanes; }

    // What the layer is waiting for, in words, for `hyprctl hyprglass panes`.
    [[nodiscard]] std::string describeState() const;

    // Brings the field and the list of panes up to date with what the layer
    // drew. Returns false when there is nothing to draw glass on yet.
    [[nodiscard]] bool update(const SMaskSource& mask, const Vector2D& boxSize, const GlassParams& params, const SP<Render::IFramebuffer>& caller);

    [[nodiscard]] std::vector<SPane>& panes() noexcept { return m_panes; }

    // Device pixels a pane's frost reads beyond the pane, and its shadow reaches beyond it;
    // how far to damage around a pane, how much of the background a sample must include
    // beyond the layer, and how far around a part of the outline that moved to repaint.
    [[nodiscard]] int    frostReach() const noexcept { return m_reaches.frost; }
    [[nodiscard]] int    shadowReach() const noexcept { return m_reaches.shadow; }
    [[nodiscard]] int    damageReach() const noexcept { return m_reaches.damage; }
    [[nodiscard]] int    paddingReach() const noexcept { return m_reaches.padding; }
    [[nodiscard]] double repaintReach() const noexcept { return m_reaches.repaint; }

    // The monitor rectangle a pane's frost would be read from, snapped to the
    // reduction's grid and clipped to the source.
    [[nodiscard]] CBox sampleRect(const SPane& pane, const Vector2D& boxPosition, const Vector2D& sourceSize) const;

    // Reads what lies behind one pane out of source, reduces and frosts it; with
    // `clean`, the clean page of the windows under it instead (Apple's shader only).
    void sample(SPane& pane, const SP<Render::IFramebuffer>& source, const Vector2D& boxPosition, const SP<Render::IFramebuffer>& caller,
                const SCleanSource* clean = nullptr);

    // Draws the panes' shadows and then every pane that has a sample into target,
    // under where the layer's own pixels will go.
    void draw(const SP<Render::IFramebuffer>& target, const CBox& layerBox, const SMaskSource& mask, float alpha);

    [[nodiscard]] bool fieldReady() const noexcept { return m_fieldValid; }

  private:
    // True when it was built in part (partialBuild).
    bool buildField(const SMaskSource& mask, const SP<Render::IFramebuffer>& caller);
    // The part of the field a build has to redo, field pixels (the box's device
    // pixels plus the margin); none for all of it.
    [[nodiscard]] std::optional<CBox> partialBuild() const;
    void startEmpty(const SP<Render::IFramebuffer>& caller);
    void mapChanges(const SP<Render::IFramebuffer>& answer);
    [[nodiscard]] std::optional<CBox> readChanges();
    void drawOutline(const SMaskSource& mask, const SP<Render::IFramebuffer>& target, bool compare);
    void askWhetherOutlineChanged(const SMaskSource& mask, const SP<Render::IFramebuffer>& caller);
    void pollOutlineChange();
    void probeOutlineMove(const SMaskSource& mask, const SP<Render::IFramebuffer>& caller);
    void pollOutlineMove();
    void requestPanes();
    void pollPanes();
    void adoptCells(const std::vector<uint8_t>& cells);
    // The panes that stand now: the last map read, plus whatever was redrawn since.
    [[nodiscard]] std::vector<SPane> wantedPanes() const;
    void setPanes(std::vector<SPane> wanted);
    void uploadLensTable();
    void uploadFit(const std::vector<uint8_t>& fit, int width, int height);
    void drawShadows(const SP<Render::IFramebuffer>& target, const CBox& layerBox, float alpha);
    // Apple's own shader: its copy of the page, and its one pass over each pane.
    void sampleQuartz(SPane& pane, const SP<Render::IFramebuffer>& source, const Vector2D& boxPosition, const SP<Render::IFramebuffer>& caller,
                      const SCleanSource* clean);
    [[nodiscard]] CBox quartzRect(const SPane& pane, const Vector2D& boxPosition, const Vector2D& sourceSize) const;
    void drawQuartz(const SP<Render::IFramebuffer>& target, const CBox& layerBox, const SMaskSource& mask, float opacity);
    void setMaterialUniforms(CProgram& program) const;
    void setPaneUniforms(CProgram& program, const SPane& pane, const CBox& layerBox) const;
    void releaseGl();

    GlassParams m_params{};
    GlassLayout m_layout{};  // how the field, the shadow and the pane map are laid out
    GlassReaches m_reaches{};
    GlassQuartz  m_quartz{}; // Apple's shader's uniform block, when m_params.quartz names one
    // For Apple's shader: per device pixel of the box and its margin, the
    // nearest point of the outline, and how many texels that field spans.
    SP<Render::IFramebuffer> m_seeds;
    Vector2D                 m_seedTexels;
    void floodOutline(const SMaskSource& mask, const std::optional<CBox>& part);
    Vector2D m_boxSize;
    int      m_fieldScale  = 4; // device pixels per field texel
    int      m_margin      = 0; // device pixels of field around the box
    int      m_shadowReach = 0;
    Vector2D m_fieldTexels;     // texels of the field that hold data
    Vector2D m_shadowTexels;
    Vector2D m_cellCount;

    SP<Render::IFramebuffer> m_field;
    SP<Render::IFramebuffer> m_shadow;
    SP<Render::IFramebuffer> m_cells;
    GLuint                   m_lensTable      = 0;
    double                   m_deviationScale = 1.0;
    // Per cell of the pane map, how much of the whole lens the pane there has room for.
    GLuint                   m_fitTexture     = 0;
    Vector2D                 m_fitCells;

    bool                m_fieldValid   = false;
    bool                m_contentDirty = true;  // the field must be rebuilt

    // A commit only makes the field stale if it moved the outline. Whether it did is
    // asked of the GPU (outline.frag under an occlusion query) and answered a frame later.
    SP<Render::IFramebuffer>              m_outline;         // the outline as it was at the last field build
    GLuint                                m_changeQuery   = 0;
    bool                                  m_changePending = false;
    bool                                  m_checkWanted   = false; // committed since the last question
    std::optional<CBox>                   m_candidate;       // redrawn since the last question
    std::optional<CBox>                   m_questioned;      // redrawn before the question that is pending
    std::chrono::steady_clock::time_point m_lastBuild{};
    std::chrono::steady_clock::time_point m_outlineMovedAt{}; // when a question last found the outline changed
    // With the answer, where: a coarse map of the texels that differ, read back
    // like the panes. The field is then built again around that alone, and the
    // panes, where it was not built of everything (2026-10-24: the panels that
    // drop from the bar are layers the size of the screen, and building all of
    // one cost 8 to 15 ms of a 4 ms frame, five times an opening).
    SP<Render::IFramebuffer>              m_changeMap;
    GLuint                                m_changeBuffer = 0;
    GLsync                                m_changeFence  = nullptr;
    Vector2D                              m_changeCells;
    std::optional<CBox>                   m_moved;            // field pixels where the outline moved since the build
    bool                                  m_movedAnywhere = true; // or it may have moved anywhere
    std::chrono::steady_clock::time_point m_lastCommitAt{};
    bool                                  m_settleWanted = false; // built since it last stood still
    GLuint                                m_moveQuery   = 0;     // did a commit rebuilt at once move the outline?
    bool                                  m_movePending = false;
    bool                                  m_wantsPoll = false;
    bool                                  m_mapAdopted = false;
    std::optional<CBox>                   m_repaint;

    // A layer about the size of what it shows is rebuilt on every commit while its
    // outline is moving, so its glass is a frame behind only when the outline starts
    // to move; while it stands still, it is asked first, as a large one is.
    [[nodiscard]] bool isSmall() const noexcept;
    GLuint              m_lastTexture  = 0;
    Vector2D            m_lastTextureSize;
    float               m_lastThreshold = -1.0f;
    float               m_lastFeather   = -1.0f;

    // Panes are found by the GPU and read back without waiting for it: until an
    // answer arrives the last known panes stand, widened by whatever the layer
    // has redrawn since, which is the only place a new pane can have appeared.
    GLuint              m_pixelBuffer  = 0;
    GLsync              m_fence        = nullptr;
    Vector2D            m_pendingCells;
    bool                m_cellsStale   = false;
    bool                m_panesKnown   = false;
    std::vector<SPane>  m_knownPanes;   // from the last answer: extent and short side only
    std::vector<GlassPane> m_knownPaneList;      // the same, as the model takes them
    std::vector<GlassRect> m_knownInteriors;
    std::optional<CBox> m_unknown;      // redrawn since the content that answer describes
    std::optional<CBox> m_unknownAfterRequest;
    std::vector<SPane>  m_panes;
};

} // namespace Meniscus
