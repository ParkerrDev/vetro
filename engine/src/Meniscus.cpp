#include "Meniscus.hpp"
#include "Backdrop.hpp"
#include "Diagnostics.hpp"
#include "Globals.hpp"
#include "Tuning.hpp"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <hyprland/src/helpers/Color.hpp>
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <hyprland/src/render/OpenGL.hpp>
#include <hyprland/src/render/Renderer.hpp>

namespace Meniscus {

namespace {

// The uniform buffer binding point of Apple's shader's block. Hyprland's own
// shaders take no uniform blocks; the binding is undone after each use.
constexpr GLuint QUARTZ_BLOCK_BINDING = 5;

// Maps the quad Hyprland's vertex shader draws, [0, 1] x [0, 1], onto the whole viewport.
constexpr std::array<float, 9> WHOLE_VIEWPORT = {
    2.0f, 0.0f, 0.0f,
    0.0f, 2.0f, 0.0f,
   -1.0f,-1.0f, 1.0f,
};

constexpr size_t MAX_QUADS = 96; // rectangles drawn for one region before its extents are drawn instead
// A layer whose outline moves on every frame, a card sliding inside it, has its field
// rebuilt at most this often: between rebuilds the glass keeps the outline it had.
constexpr auto MIN_BUILD_INTERVAL = std::chrono::milliseconds(GLASS_MIN_BUILD_INTERVAL_MS);
// A small layer whose outline was seen to move this recently is rebuilt on every
// commit, so a pane gliding inside it (the workspace selector in the bar) is
// followed frame by frame; otherwise it is asked first, as a large one is.
constexpr auto OUTLINE_MOVING = std::chrono::milliseconds(250);
// Texels of the question's output a side per texel of the map of where the outline moved.
constexpr int CHANGE_BLOCK = 8;
// A large layer built because its outline moved is built once more, around its
// panes, when it has committed nothing for this long (see update()).
constexpr auto SETTLED = std::chrono::milliseconds(120);
// A field is built in part only where the part is at most this share of it.
constexpr double PARTIAL_MOST = 0.85;

GLuint fbId(const SP<Render::IFramebuffer>& framebuffer) {
    return dynamic_cast<Render::GL::CGLFramebuffer*>(framebuffer.get())->getFBID();
}

// A scratch framebuffer that holds at least width x height and never shrinks.
void ensureAtLeast(SP<Render::IFramebuffer>& framebuffer, const char* name, int width, int height, DRMFormat format) {
    if (!framebuffer)
        framebuffer = g_pHyprRenderer->createFB(name);

    const int have_w = static_cast<int>(framebuffer->m_size.x);
    const int have_h = static_cast<int>(framebuffer->m_size.y);
    if (have_w >= width && have_h >= height && framebuffer->m_drmFormat == format)
        return;

    framebuffer->alloc(std::max(width, have_w), std::max(height, have_h), format);
}

void ensureExact(SP<Render::IFramebuffer>& framebuffer, const char* name, int width, int height, DRMFormat format) {
    if (!framebuffer)
        framebuffer = g_pHyprRenderer->createFB(name);

    if (static_cast<int>(framebuffer->m_size.x) != width || static_cast<int>(framebuffer->m_size.y) != height || framebuffer->m_drmFormat != format)
        framebuffer->alloc(width, height, format);
}

// Every pass here overwrites its target: blending would mix in what the target
// held, and a scissor or stencil left on by the render pass would leave texels unwritten.
void beginOffscreen() {
    g_pHyprRenderer->blend(false);
    g_pHyprOpenGL->setCapStatus(GL_SCISSOR_TEST, false);
    if (glIsEnabled(GL_SCISSOR_TEST)) {
        glDisable(GL_SCISSOR_TEST);
        Diagnostics::recordStateDesync("scissor on before the glass passes");
    }
    if (glIsEnabled(GL_STENCIL_TEST)) {
        glDisable(GL_STENCIL_TEST);
        Diagnostics::recordStateDesync("stencil on before the glass passes");
    }
    glActiveTexture(GL_TEXTURE0);
}

// Hands the GL state back as Hyprland keeps it between elements. The viewport
// comes from the framebuffer's own size: monitor sizes are wrong for it on
// rotated outputs.
void endOffscreen(const SP<Render::IFramebuffer>& caller) {
    g_pHyprRenderer->blend(true);
    glActiveTexture(GL_TEXTURE0);
    glBindVertexArray(0);
    if (!caller)
        return;
    glBindFramebuffer(GL_FRAMEBUFFER, fbId(caller));
    g_pHyprOpenGL->setViewport(0, 0, static_cast<int>(caller->m_size.x), static_cast<int>(caller->m_size.y));
}

// Binds a program for a pass that fills the first width x height texels of target.
WP<CShader> beginPass(CProgram& program, const SP<Render::IFramebuffer>& target, int width, int height) {
    auto shader = g_pHyprOpenGL->useShader(program.shader);
    shader->setUniformMatrix3fv(SHADER_PROJ, 1, GL_FALSE, WHOLE_VIEWPORT);
    shader->setUniformInt(SHADER_TEX, 0);
    glBindVertexArray(shader->getUniformLocation(SHADER_SHADER_VAO));
    glBindFramebuffer(GL_FRAMEBUFFER, fbId(target));
    g_pHyprOpenGL->setViewport(0, 0, width, height);
    return shader;
}

void bindTexture(GLenum unit, const SP<Render::ITexture>& texture) {
    glActiveTexture(unit);
    texture->bind();
}

// A client's buffer gets its filters from Hyprland only when Hyprland first
// draws it, and the glass is drawn before that. Until then the texture is
// incomplete to GL and reads as opaque black, which would make the whole layer
// one pane. The setter is Hyprland's own, so its record of the state stays true.
void bindMask(GLenum unit, const SP<Render::ITexture>& texture) {
    glActiveTexture(unit);
    texture->bind();
    texture->setTexParameter(GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    texture->setTexParameter(GL_TEXTURE_MAG_FILTER, GL_LINEAR);
}

// The separable Gaussian of a frost, run as its plan says between `texture`,
// which holds the copy and gets the result, and `other`. `lastly` is what the
// last pass does besides (frostblur.frag's applyFace).
void frostBlurPasses(CProgram& program, const SP<Render::IFramebuffer>& texture, const SP<Render::IFramebuffer>& other, int width, int height,
                     const GlassFrostPlan& plan, int lastly) {
    const auto pass = [&](const SP<Render::IFramebuffer>& from, const SP<Render::IFramebuffer>& to, bool across, bool last) {
        const float allocW = static_cast<float>(from->m_size.x);
        const float allocH = static_cast<float>(from->m_size.y);
        beginPass(program, to, width, height);
        bindTexture(GL_TEXTURE0, from->getTexture());
        glUniform2f(program.uniform("texelStep"), across ? 1.0f / allocW : 0.0f, across ? 0.0f : 1.0f / allocH);
        glUniform2f(program.uniform("uvLow"), 0.5f / allocW, 0.5f / allocH);
        glUniform2f(program.uniform("uvHigh"), (static_cast<float>(width) - 0.5f) / allocW, (static_cast<float>(height) - 0.5f) / allocH);
        glUniform2f(program.uniform("uvScale"), static_cast<float>(width) / allocW, static_cast<float>(height) / allocH);
        glUniform1f(program.uniform("sigma"), plan.pass_sigma);
        glUniform1i(program.uniform("pairs"), plan.pairs);
        glUniform1i(program.uniform("applyFace"), last ? lastly : 0);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    };
    for (int i = 0; i < plan.passes; ++i) {
        pass(texture, other, true, false);
        pass(other, texture, false, i == plan.passes - 1);
    }
}

CBox unite(const CBox& a, const CBox& b) {
    const double x1 = std::min(a.x, b.x);
    const double y1 = std::min(a.y, b.y);
    const double x2 = std::max(a.x + a.w, b.x + b.w);
    const double y2 = std::max(a.y + a.h, b.y + b.h);
    return CBox{x1, y1, x2 - x1, y2 - y1};
}

void widen(std::optional<CBox>& into, const CBox& box) {
    into = into ? unite(*into, box) : box;
}

// The rectangles of a region, or its extents when there are too many to be worth drawing one by one.
std::vector<CBox> quadsOf(const CRegion& region) {
    std::vector<CBox> quads;
    const auto        rects = region.getRects();
    if (rects.size() > MAX_QUADS) {
        quads.push_back(region.copy().getExtents());
        return quads;
    }
    quads.reserve(rects.size());
    for (const auto& rect : rects)
        quads.emplace_back(static_cast<double>(rect.x1), static_cast<double>(rect.y1), static_cast<double>(rect.x2 - rect.x1), static_cast<double>(rect.y2 - rect.y1));
    return quads;
}

// The panes the model answered with, as this side keeps them.
std::vector<SPane> toPanes(const GlassPane* panes, size_t count, const GlassRect* interiors) {
    std::vector<SPane> out;
    out.reserve(count);
    for (size_t i = 0; i < count; ++i) {
        SPane pane;
        pane.box       = Glass::toBox(panes[i].rect);
        pane.shortSide = panes[i].short_side;
        for (uint32_t k = 0; k < panes[i].interior_count; ++k)
            pane.interiors.push_back(Glass::toBox(interiors[panes[i].first_interior + k]));
        out.push_back(std::move(pane));
    }
    return out;
}

} // namespace

// ── Parameters ───────────────────────────────────────────────────────────────

bool presetUsesModel(const SResolveContext& context) {
    return glass_uses_model(&context.values()) == 1;
}

GlassParams resolveParams(const SResolveContext& context, float monitorScale) {
    GlassParams params{};
    (void)glass_model_params(&context.values(), monitorScale, &params);
    return params;
}

// ── Programs ─────────────────────────────────────────────────────────────────

bool CProgram::create(const char* fragmentName) {
    const std::string& source = Glass::shaderSource(fragmentName);
    if (source.empty())
        return false;

    m_locations.clear();
    // dynamic: a shader that does not compile must report it, not abort the compositor
    return shader->createProgram(g_pHyprOpenGL->m_shaders->TEXVERTSRC, source, true);
}

void CProgram::destroy() {
    shader->destroy();
    m_locations.clear();
}

GLint CProgram::uniform(const char* name) {
    const auto found = m_locations.find(name);
    if (found != m_locations.end())
        return found->second;

    const GLint location = glGetUniformLocation(shader->program(), name);
    m_locations.emplace(name, location);
    return location;
}

void SPrograms::initializeIfNeeded() {
    if (m_ready)
        return;

    m_ready = coverage.create("coverage.frag") && outline.create("outline.frag") && fieldBlur.create("fieldblur.frag") && field.create("field.frag") && cells.create("cells.frag") &&
        shadowBlur.create("shadowblur.frag") && reduce.create("reduce.frag") && frostBlur.create("frostblur.frag") && shadow.create("shadow.frag") &&
        face.create("face.frag") && glass.create("meniscus.frag") && quartz.create("quartz27.frag") && quartzSeed.create("quartzseed.frag") &&
        quartzFlood.create("quartzflood.frag") && quartzClean.create("quartzclean.frag") && changes.create("changes.frag");

    // Apple's shader takes its numbers as one uniform block, Apple's own layout.
    if (m_ready) {
        const GLuint index = glGetUniformBlockIndex(quartz.shader->program(), "GlassParams27");
        if (index != GL_INVALID_INDEX)
            glUniformBlockBinding(quartz.shader->program(), index, QUARTZ_BLOCK_BINDING);
        if (!quartzBlock)
            glGenBuffers(1, &quartzBlock);
    }

    if (!m_ready && !m_failed)
        HyprlandAPI::addNotification(PHANDLE, std::format("[{}] Failed to compile the lens model's shaders", PLUGIN_NAME), CHyprColor{1.0, 0.2, 0.2, 1.0}, 5000);
    m_failed = !m_ready;
}

void SPrograms::destroy() {
    for (auto* program : {&coverage, &outline, &fieldBlur, &field, &cells, &shadowBlur, &reduce, &frostBlur, &shadow, &face, &glass, &quartz, &quartzSeed, &quartzFlood,
                          &quartzClean, &changes})
        program->destroy();
    if (quartzBlock) {
        glDeleteBuffers(1, &quartzBlock);
        quartzBlock = 0;
    }
    m_ready  = false;
    m_failed = false;
}

void SScratch::release() {
    for (auto* framebuffer : {&maskA, &maskB, &shadowA, &shadowB, &blur, &cloud, &seedA, &seedB})
        framebuffer->reset();
}

// ── The cloud of a window's pane ─────────────────────────────────────────────

bool makeCloud(const SP<Render::IFramebuffer>& page, SP<Render::IFramebuffer>& cloud, const GlassParams& params, int downscale,
               const SP<Render::IFramebuffer>& caller) {
    auto& global   = *g_pGlobalState;
    auto& programs = global.meniscusPrograms;
    programs.initializeIfNeeded();
    if (!page || !caller || !programs.ready() || downscale < 1)
        return false;

    // The plan is made for the screen's own pixels. The page is already smaller
    // by `downscale`, so only the rest of the reduction is left to do, and the
    // plan's blur holds when the two come to the plan's texel exactly.
    GlassFrostPlan plan{};
    (void)glass_frost_plan(params.layered.frost_wide, &plan);
    const int  factor   = std::max(1, plan.reduction / downscale);
    const bool planned  = downscale * factor == plan.reduction && plan.face_applied != 0;
    const bool weighted = planned && factor > 1 && std::abs(params.layered.cloud_weight - 1.0f) > 1e-4f;
    const int  sourceW  = static_cast<int>(page->m_size.x);
    const int  sourceH  = static_cast<int>(page->m_size.y);
    const int  width    = std::max(1, sourceW / factor);
    const int  height   = std::max(1, sourceH / factor);
    const auto format   = weighted ? DRM_FORMAT_ARGB8888 : page->m_drmFormat;

    ensureExact(cloud, "hyprglass-window-cloud", width, height, format);
    beginOffscreen();
    if (factor == 1) {
        glBindFramebuffer(GL_READ_FRAMEBUFFER, fbId(page));
        glBindFramebuffer(GL_DRAW_FRAMEBUFFER, fbId(cloud));
        glBlitFramebuffer(0, 0, sourceW, sourceH, 0, 0, width, height, GL_COLOR_BUFFER_BIT, GL_NEAREST);
    } else {
        auto& program = programs.reduce;
        beginPass(program, cloud, width, height);
        bindTexture(GL_TEXTURE0, page->getTexture());
        glUniform2f(program.uniform("sourceOrigin"), 0.0f, 0.0f);
        glUniform2f(program.uniform("invSourceSize"), 1.0f / static_cast<float>(sourceW), 1.0f / static_cast<float>(sourceH));
        glUniform1i(program.uniform("scale"), factor);
        glUniform1f(program.uniform("whiteWeight"), weighted ? params.layered.cloud_weight : 0.0f);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    }
    if (planned) {
        auto& scratch = global.meniscusScratch;
        auto& other   = weighted ? scratch.cloud : scratch.blur;
        ensureAtLeast(other, weighted ? "hyprglass-cloud-blur" : "hyprglass-frost-blur", width, height, format);
        frostBlurPasses(programs.frostBlur, cloud, other, width, height, plan, weighted ? 2 : 0);
        if (const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock())
            Diagnostics::recordBlurPasses(monitor->m_id, static_cast<uint64_t>(plan.passes) * 2);
    }
    endOffscreen(caller);
    return true;
}

// ── Per-layer state ──────────────────────────────────────────────────────────

CLayer::CLayer() {
    // until the first update: no frost to reach for, no shadow
    (void)glass_reaches(&m_params, 0, &m_reaches);
}

CLayer::~CLayer() {
    releaseGl();
}

void CLayer::releaseGl() {
    if (!m_fence && !m_pixelBuffer && !m_lensTable && !m_fitTexture && !m_changeQuery && !m_moveQuery && !m_changeFence && !m_changeBuffer)
        return;

    // A layer can close outside a frame: the objects belong to Hyprland's context.
    if (!g_pHyprOpenGL)
        return;
    g_pHyprOpenGL->makeEGLCurrent();

    if (m_fence) {
        glDeleteSync(m_fence);
        m_fence = nullptr;
    }
    if (m_pixelBuffer) {
        glDeleteBuffers(1, &m_pixelBuffer);
        m_pixelBuffer = 0;
    }
    if (m_changeFence) {
        glDeleteSync(m_changeFence);
        m_changeFence = nullptr;
    }
    if (m_changeBuffer) {
        glDeleteBuffers(1, &m_changeBuffer);
        m_changeBuffer = 0;
    }
    if (m_lensTable) {
        glDeleteTextures(1, &m_lensTable);
        m_lensTable = 0;
    }
    if (m_fitTexture) {
        glDeleteTextures(1, &m_fitTexture);
        m_fitTexture = 0;
    }
    if (m_changeQuery) {
        glDeleteQueries(1, &m_changeQuery);
        m_changeQuery = 0;
    }
    if (m_moveQuery) {
        glDeleteQueries(1, &m_moveQuery);
        m_moveQuery = 0;
    }
}

// One byte per cell of the pane map. Read filtered: between two panes there is
// no glass, so what the values blend to there is never used.
void CLayer::uploadFit(const std::vector<uint8_t>& fit, int width, int height) {
    if (!m_fitTexture)
        glGenTextures(1, &m_fitTexture);
    glActiveTexture(GL_TEXTURE0);
    glBindTexture(GL_TEXTURE_2D, m_fitTexture);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_R8, width, height, 0, GL_RED, GL_UNSIGNED_BYTE, fit.data());
    glPixelStorei(GL_UNPACK_ALIGNMENT, 4);
    glBindTexture(GL_TEXTURE_2D, 0);
    m_fitCells = Vector2D{static_cast<double>(width), static_cast<double>(height)};
}

void CLayer::contentChanged(const std::optional<CBox>& box) {
    // No box means everything may have changed: wider than any layer can be.
    widen(m_candidate, box.value_or(CBox{-1e6, -1e6, 2e6, 2e6}));
    m_checkWanted  = true;
    m_lastCommitAt = std::chrono::steady_clock::now();
}

bool CLayer::takePollRequest() noexcept {
    const bool wanted = m_wantsPoll;
    m_wantsPoll       = false;
    return wanted;
}

CLayer::EWaiting CLayer::poll() {
    pollOutlineChange();
    pollOutlineMove();
    // The map of the panes is read here too: after the last commit of a burst no
    // frame may follow in which update() could read it.
    pollPanes();
    if (m_changePending || m_fence || m_movePending)
        return EWaiting::ANSWER;
    if (m_contentDirty)
        return (isSmall() || std::chrono::steady_clock::now() - m_lastBuild >= MIN_BUILD_INTERVAL) ? EWaiting::FRAME : EWaiting::ANSWER;
    if (m_settleWanted && !m_checkWanted)
        return std::chrono::steady_clock::now() - m_lastCommitAt >= SETTLED ? EWaiting::FRAME : EWaiting::ANSWER;
    // committed again while the last question was out: the next frame asks anew
    return m_checkWanted ? EWaiting::FRAME : EWaiting::NOTHING;
}

std::optional<CBox> CLayer::takeRepaint() noexcept {
    const auto repaint = m_repaint;
    m_repaint.reset();
    return repaint;
}

bool CLayer::takeMapAdopted() noexcept {
    const bool adopted = m_mapAdopted;
    m_mapAdopted       = false;
    return adopted;
}

std::string CLayer::describeState() const {
    std::string state;
    const auto  add = [&state](bool condition, const char* text) {
        if (condition)
            state += std::string(state.empty() ? "" : ", ") + text;
    };
    add(!m_fieldValid, "no field yet");
    add(!m_panesKnown, "map of the panes not read yet");
    add(m_contentDirty, "field to be rebuilt");
    add(m_checkWanted, "outline to be asked about");
    add(m_changePending, "waiting for the answer about the outline");
    add(m_fence != nullptr, "waiting for the map of the panes");
    add(m_unknown.has_value(), "part of the layer redrawn since the map");
    return state.empty() ? "settled" : state;
}

bool CLayer::isSmall() const noexcept {
    return m_layout.small != 0;
}

void CLayer::drawOutline(const SMaskSource& mask, const SP<Render::IFramebuffer>& target, bool compare) {
    auto&     program = g_pGlobalState->meniscusPrograms.outline;
    const int width   = static_cast<int>(m_fieldTexels.x);
    const int height  = static_cast<int>(m_fieldTexels.y);

    beginPass(program, target, width, height);
    bindMask(GL_TEXTURE0, mask.texture);
    if (compare) {
        bindTexture(GL_TEXTURE1, m_outline->getTexture());
        glActiveTexture(GL_TEXTURE0);
    }
    if (!compare) {
        // the target of this pass may still be bound there from the last question
        glActiveTexture(GL_TEXTURE1);
        glBindTexture(GL_TEXTURE_2D, 0);
        glActiveTexture(GL_TEXTURE0);
    }
    glUniform1i(program.uniform("previous"), 1);
    glUniform2f(program.uniform("invPreviousSize"), 1.0f / static_cast<float>(width), 1.0f / static_cast<float>(height));
    glUniform2f(program.uniform("boxSize"), static_cast<float>(m_boxSize.x), static_cast<float>(m_boxSize.y));
    glUniform2f(program.uniform("margin"), static_cast<float>(m_margin), static_cast<float>(m_margin));
    glUniform2f(program.uniform("maskUvScale"), static_cast<float>(mask.uvScale.x), static_cast<float>(mask.uvScale.y));
    glUniform1f(program.uniform("threshold"), mask.threshold);
    glUniform1i(program.uniform("scale"), m_fieldScale);
    glUniform1i(program.uniform("compare"), compare ? 1 : 0);
    glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
}

// One cheap pass under an occlusion query: did any texel of the outline change
// since the field was built? The answer is read on a later frame, without waiting.
void CLayer::askWhetherOutlineChanged(const SMaskSource& mask, const SP<Render::IFramebuffer>& caller) {
    auto& scratch = g_pGlobalState->meniscusScratch;
    ensureAtLeast(scratch.maskB, "hyprglass-mask-b", static_cast<int>(m_fieldTexels.x), static_cast<int>(m_fieldTexels.y), DRM_FORMAT_ARGB8888);
    if (!m_changeQuery)
        glGenQueries(1, &m_changeQuery);

    beginOffscreen();
    // cleared first, so that what the question writes is where the outline differs
    glBindFramebuffer(GL_FRAMEBUFFER, fbId(scratch.maskB));
    glClearColor(0.0f, 0.0f, 0.0f, 0.0f);
    glClear(GL_COLOR_BUFFER_BIT);
    glBeginQuery(GL_ANY_SAMPLES_PASSED, m_changeQuery);
    drawOutline(mask, scratch.maskB, true);
    glEndQuery(GL_ANY_SAMPLES_PASSED);
    // a small layer is built whole anyway
    if (!isSmall())
        mapChanges(scratch.maskB);
    endOffscreen(caller);

    m_changePending = true;
    m_checkWanted   = false;
    m_questioned    = m_candidate;
    m_candidate.reset();
    m_wantsPoll = true;
}

void CLayer::pollOutlineChange() {
    if (!m_changePending)
        return;

    GLuint available = GL_FALSE;
    glGetQueryObjectuiv(m_changeQuery, GL_QUERY_RESULT_AVAILABLE, &available);
    if (!available)
        return;

    // where comes with it, a moment after: that is waited for too
    bool                mapped = false;
    std::optional<CBox> where;
    if (m_changeFence) {
        const GLenum state = glClientWaitSync(m_changeFence, 0, 0);
        if (state == GL_TIMEOUT_EXPIRED)
            return;
        glDeleteSync(m_changeFence);
        m_changeFence = nullptr;
        if (state == GL_ALREADY_SIGNALED || state == GL_CONDITION_SATISFIED) {
            where  = readChanges();
            mapped = true;
        }
    }

    GLuint changed = GL_FALSE;
    glGetQueryObjectuiv(m_changeQuery, GL_QUERY_RESULT, &changed);
    m_changePending = false;

    if (changed) {
        // the outline moved somewhere in what was redrawn: that is where a pane may be new,
        // and where the glass on screen is no longer the right one
        m_contentDirty    = true;
        m_outlineMovedAt  = std::chrono::steady_clock::now();
        if (mapped && where)
            widen(m_moved, *where);
        else
            m_movedAnywhere = true;
        if (m_questioned) {
            widen(m_unknown, *m_questioned);
            widen(m_repaint, *m_questioned);
            if (m_fence)
                widen(m_unknownAfterRequest, *m_questioned);
        }
    }
    m_questioned.reset();
}

// The same pass as the question, for a small layer rebuilt at once while its
// outline moves: it changes nothing but whether the layer still counts as
// moving, so the first commit that leaves the outline where it was ends that.
void CLayer::probeOutlineMove(const SMaskSource& mask, const SP<Render::IFramebuffer>& caller) {
    auto& scratch = g_pGlobalState->meniscusScratch;
    ensureAtLeast(scratch.maskB, "hyprglass-mask-b", static_cast<int>(m_fieldTexels.x), static_cast<int>(m_fieldTexels.y), DRM_FORMAT_ARGB8888);
    if (!m_moveQuery)
        glGenQueries(1, &m_moveQuery);

    beginOffscreen();
    glBeginQuery(GL_ANY_SAMPLES_PASSED, m_moveQuery);
    drawOutline(mask, scratch.maskB, true);
    glEndQuery(GL_ANY_SAMPLES_PASSED);
    endOffscreen(caller);

    m_movePending = true;
    m_wantsPoll   = true;
}

void CLayer::pollOutlineMove() {
    if (!m_movePending)
        return;

    GLuint available = GL_FALSE;
    glGetQueryObjectuiv(m_moveQuery, GL_QUERY_RESULT_AVAILABLE, &available);
    if (!available)
        return;

    GLuint moved = GL_FALSE;
    glGetQueryObjectuiv(m_moveQuery, GL_QUERY_RESULT, &moved);
    m_movePending    = false;
    m_outlineMovedAt = moved ? std::chrono::steady_clock::now() : std::chrono::steady_clock::time_point{};
}

void CLayer::uploadLensTable() {
    std::vector<uint8_t> texels(static_cast<size_t>(GLASS_LENS_TABLE_SIZE) * 2 * 4);
    double               scale = 1.0;
    if (glass_lens_table(&m_params.lens, texels.data(), texels.size(), &scale) != GLASS_OK)
        return;
    m_deviationScale = scale;

    if (!m_lensTable)
        glGenTextures(1, &m_lensTable);
    glActiveTexture(GL_TEXTURE0);
    glBindTexture(GL_TEXTURE_2D, m_lensTable);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA8, GLASS_LENS_TABLE_SIZE, 2, 0, GL_RGBA, GL_UNSIGNED_BYTE, texels.data());
    glBindTexture(GL_TEXTURE_2D, 0);
}

bool CLayer::update(const SMaskSource& mask, const Vector2D& boxSize, const GlassParams& params, const SP<Render::IFramebuffer>& caller) {
    if (!mask.texture || boxSize.x < 1.0 || boxSize.y < 1.0)
        return false;

    pollPanes();
    pollOutlineChange();

    // which rebuilds the change of parameters asks for is the model's to say
    GlassChanges changes{1, 1, 1};
    (void)glass_params_changes(&m_params, &params, &changes);
    const bool layoutChanged = !m_fieldValid || boxSize != m_boxSize || changes.field || mask.threshold != m_lastThreshold || mask.feather != m_lastFeather ||
        (params.quartz != 0) != static_cast<bool>(m_seeds);
    const bool lensChanged   = !m_lensTable || changes.lens;
    const bool frostChanged  = changes.frost;
    m_params                 = params;
    if (m_params.quartz != 0 && !Tuning::quartz(m_params.quartz, m_quartz))
        m_params.quartz = 0;
    if (frostChanged) {
        for (auto& pane : m_panes)
            pane.dirty = true;
    }
    if (lensChanged)
        uploadLensTable();

    if (layoutChanged) {
        m_boxSize       = boxSize;
        m_lastThreshold = mask.threshold;
        m_lastFeather   = mask.feather;

        GlassLayout layout{};
        (void)glass_layout(&m_params, boxSize.x, boxSize.y, &layout);
        m_layout       = layout;
        m_fieldScale   = layout.field_scale;
        m_margin       = layout.margin;
        m_shadowReach  = layout.shadow_reach;
        m_fieldTexels  = Vector2D{layout.field_texels[0], layout.field_texels[1]};
        m_shadowTexels = Vector2D{layout.shadow_texels[0], layout.shadow_texels[1]};
        m_cellCount    = Vector2D{layout.cells[0], layout.cells[1]};

        // Nothing known about the new shape: one pane as large as the layer until the map is read,
        // and a lens fitted to that.
        uploadFit({static_cast<uint8_t>(layout.initial_fit)}, 1, 1);
        m_panesKnown = false;
        m_knownPanes.clear();
        m_knownPaneList.clear();
        m_knownInteriors.clear();
        m_unknown.reset();
        m_unknownAfterRequest.reset();
        m_candidate.reset();
        m_questioned.reset();
        // an answer still on its way describes a layout that is gone
        m_changePending = false;
        m_checkWanted   = false;
        m_contentDirty  = true;
        m_movedAnywhere = true;
        if (m_changeFence) {
            glDeleteSync(m_changeFence);
            m_changeFence = nullptr;
        }
        // A large layer starts from the field of nothing drawn and is asked what it
        // has drawn, which is then built where it is: a panel that maps before it
        // fades in is not built whole for an outline it does not have yet.
        if (!isSmall() && m_layout.shadow_on == 0) {
            startEmpty(caller);
            m_contentDirty  = false;
            m_movedAnywhere = false;
            contentChanged(std::nullopt);
        }
    }

    // Reaches follow the parameters even when the field itself does not change.
    (void)glass_reaches(&m_params, m_shadowReach, &m_reaches);

    // A buffer swapped without a commit having been reported: the same question applies.
    if (mask.texture->m_texID != m_lastTexture || mask.texture->m_size != m_lastTextureSize) {
        m_lastTexture     = mask.texture->m_texID;
        m_lastTextureSize = mask.texture->m_size;
        if (!m_checkWanted && !layoutChanged)
            contentChanged(std::nullopt);
    }

    const auto now = std::chrono::steady_clock::now();

    // A small layer whose outline is moving is not asked, but rebuilt at once, so
    // its glass keeps up with it. One whose outline stands still is asked first:
    // a panel fading in, a highlight or a line of text changing inside a card
    // leaves the outline as it was, and its field (the most of what its glass
    // costs on a frame, 2026-10-23) need not be built again for it.
    // Moving or not is asked too, by a probe that only says so, before the field
    // takes the new outline.
    pollOutlineMove();
    if (m_checkWanted && !m_contentDirty && isSmall() && now - m_outlineMovedAt < OUTLINE_MOVING) {
        if (!m_movePending && m_fieldValid && m_outline)
            probeOutlineMove(mask, caller);
        m_contentDirty  = true;
        m_movedAnywhere = true;
        m_checkWanted   = false;
    }
    // The question sees the outline a field texel at a time, so the last
    // antialiased pixels of a fade can settle after the last change it saw: once
    // the layer has drawn nothing new for a moment, its panes are built again.
    bool settling = false;
    if (m_settleWanted && !m_contentDirty && !m_checkWanted && !m_changePending && now - m_lastCommitAt >= SETTLED) {
        m_settleWanted = false;
        if (!m_knownPanes.empty()) {
            const double margin = m_margin;
            for (const auto& pane : m_knownPanes) {
                widen(m_moved, pane.box.copy().translate({margin, margin}));
                widen(m_repaint, pane.box);
            }
            m_contentDirty = true;
            settling       = true;
        }
    }
    if (m_contentDirty && (layoutChanged || isSmall() || now - m_lastBuild >= MIN_BUILD_INTERVAL)) {
        const bool inPart = buildField(mask, caller);
        m_settleWanted    = !isSmall() && !settling;
        m_contentDirty    = false;
        m_lastBuild       = now;
        m_moved.reset();
        m_movedAnywhere = false;
        // The field now matches everything committed so far: whatever was redrawn
        // since the last question is in it, and is where the map may differ. Built
        // in part, it matches it only there: what was committed since the question
        // is asked about.
        if (m_candidate) {
            widen(m_unknown, *m_candidate);
            if (!inPart)
                m_candidate.reset();
        }
        m_checkWanted = inPart && m_candidate.has_value();

        if (m_fence)
            m_cellsStale = true;
        else
            requestPanes();
    } else if (m_contentDirty) {
        m_wantsPoll = true;
    } else if (m_checkWanted && !m_changePending) {
        askWhetherOutlineChanged(mask, caller);
    } else if (m_checkWanted || m_changePending) {
        m_wantsPoll = true;
    }
    // The map asked for comes back between frames, and one read at the top of this
    // frame changed panes outside this frame's damage: poll() sees to both.
    if (m_fence || m_mapAdopted)
        m_wantsPoll = true;

    // The panes that stand now: the last answer, plus whatever was redrawn since.
    // A large layer's map that found nothing says nothing of a layer redrawn too
    // widely since to count (Qt redraws the whole surface): the glass that stood
    // stands until the next map. 2026-10-24: the menu's screen of glass went clear
    // for a few frames as the menu opened, from a first map read before it drew.
    auto       wanted = wantedPanes();
    const bool stale  = wanted.empty() && !isSmall() && m_layout.shadow_on == 0 && m_panesKnown && m_knownPanes.empty() && m_unknown && m_unknown->w > 0.0 &&
        m_unknown->h > 0.0;
    if (!stale)
        setPanes(std::move(wanted));

    return m_fieldValid && !m_panes.empty();
}

// What the model makes of the last map and of what was redrawn since.
std::vector<SPane> CLayer::wantedPanes() const {
    GlassWantedRequest request{};
    request.known     = GlassPaneList{m_knownPaneList.data(), m_knownPaneList.size(), m_knownInteriors.data(), m_knownInteriors.size()};
    request.has_known = m_panesKnown ? 1 : 0;
    if (m_unknown) {
        request.unknown     = Glass::toRect(*m_unknown);
        request.has_unknown = 1;
    }
    request.whole = GlassRect{0.0, 0.0, m_boxSize.x, m_boxSize.y};
    request.small = isSmall() ? 1 : 0;

    std::vector<GlassPane> panes(m_knownPaneList.size() + 1);
    std::vector<GlassRect> interiors(m_knownInteriors.size());
    GlassPaneBuffers       out{};
    out.panes             = panes.data();
    out.pane_capacity     = panes.size();
    out.interiors         = interiors.data();
    out.interior_capacity = interiors.size();
    if (glass_panes_wanted(&request, &out) != GLASS_OK) {
        // the model could not answer: one pane as large as the layer, which is what stands before any map
        SPane whole;
        whole.box       = CBox{0.0, 0.0, m_boxSize.x, m_boxSize.y};
        whole.shortSide = std::min(m_boxSize.x, m_boxSize.y);
        return {whole};
    }
    return toPanes(panes.data(), out.pane_count, interiors.data());
}

void CLayer::setPanes(std::vector<SPane> wanted) {
    // A pane that kept its extent keeps its frost: only new ones need sampling.
    std::vector<SPane> next;
    next.reserve(wanted.size());
    for (auto& pane : wanted) {
        const auto kept = std::ranges::find_if(m_panes, [&pane](const SPane& old) { return old.box == pane.box; });
        if (kept != m_panes.end()) {
            kept->shortSide = pane.shortSide;
            kept->interiors = std::move(pane.interiors);
            next.push_back(std::move(*kept));
            // moved-from entries must not match again
            kept->box = CBox{-1.0, -1.0, 0.0, 0.0};
            continue;
        }
        next.push_back(std::move(pane));
    }
    m_panes = std::move(next);
}

// Texels [x0, x1) x [y0, y1) of a target, for a pass over part of it.
struct STexels {
    int x0 = 0;
    int y0 = 0;
    int x1 = 0;
    int y1 = 0;
};

// The texels of a grid `per` field pixels a texel that cover `part`, grown by
// `grow` texels, within the first `width` x `height`.
STexels texelsOf(const CBox& part, int per, int grow, int width, int height) {
    const auto at = [per](double v, bool up) { return static_cast<int>(up ? std::ceil(v / per) : std::floor(v / per)); };
    return STexels{
        std::clamp(at(part.x, false) - grow, 0, width),
        std::clamp(at(part.y, false) - grow, 0, height),
        std::clamp(at(part.x + part.w, true) + grow, 0, width),
        std::clamp(at(part.y + part.h, true) + grow, 0, height),
    };
}

// The pass that follows writes only these texels (none: all of its viewport).
// Hyprland keeps the last box it gave glScissor and does not give it again, so
// whoever calls this puts that box back afterwards (SScissorKept).
void scissorTo(const std::optional<STexels>& texels) {
    if (!texels) {
        glDisable(GL_SCISSOR_TEST);
        return;
    }
    glEnable(GL_SCISSOR_TEST);
    glScissor(texels->x0, texels->y0, std::max(0, texels->x1 - texels->x0), std::max(0, texels->y1 - texels->y0));
}

// Hyprland's scissor box, as it was, put back when this goes.
struct SScissorKept {
    std::array<GLint, 4> box{};
    SScissorKept() {
        glGetIntegerv(GL_SCISSOR_BOX, box.data());
    }
    ~SScissorKept() {
        glDisable(GL_SCISSOR_TEST);
        glScissor(box[0], box[1], box[2], box[3]);
    }
    SScissorKept(const SScissorKept&)            = delete;
    SScissorKept& operator=(const SScissorKept&) = delete;
};

// What a pass reads stays within these, the first texel and the last.
void boundsTo(CProgram& program, const std::optional<STexels>& texels, int width, int height) {
    if (texels) {
        glUniform2i(program.uniform("lowTexel"), texels->x0, texels->y0);
        glUniform2i(program.uniform("highTexel"), std::max(texels->x0, texels->x1 - 1), std::max(texels->y0, texels->y1 - 1));
    } else {
        glUniform2i(program.uniform("lowTexel"), 0, 0);
        glUniform2i(program.uniform("highTexel"), width - 1, height - 1);
    }
}

// Where the outline moved, grown to every pane the map knows (a texel of a pane
// takes the nearest point of its own outline, which the flood has to see) and
// beyond them by what the blur that makes the field and the glass drawn past a
// pane reach, in whole cells of the map. None where that is most of the field,
// or where it may have moved anywhere: then all of it is built.
std::optional<CBox> CLayer::partialBuild() const {
    if (m_movedAnywhere || !m_moved || !m_fieldValid || isSmall() || m_layout.shadow_on != 0)
        return std::nullopt;

    const double margin = m_margin;
    double       x0     = m_moved->x;
    double       y0     = m_moved->y;
    double       x1     = m_moved->x + m_moved->w;
    double       y1     = m_moved->y + m_moved->h;
    for (const auto& pane : m_knownPanes) {
        x0 = std::min(x0, pane.box.x + margin);
        y0 = std::min(y0, pane.box.y + margin);
        x1 = std::max(x1, pane.box.x + pane.box.w + margin);
        y1 = std::max(y1, pane.box.y + pane.box.h + margin);
    }
    // the margin holds the field's blur, and two cells the map's coarseness and the glass drawn past a pane
    const double around = margin + 2.0 * GLASS_CELL_PIXELS;
    const double cell   = GLASS_CELL_PIXELS;
    const double spanW  = m_fieldTexels.x * m_fieldScale;
    const double spanH  = m_fieldTexels.y * m_fieldScale;
    x0                  = std::max(0.0, std::floor((x0 - around) / cell) * cell);
    y0                  = std::max(0.0, std::floor((y0 - around) / cell) * cell);
    x1                  = std::min(spanW, std::ceil((x1 + around) / cell) * cell);
    y1                  = std::min(spanH, std::ceil((y1 + around) / cell) * cell);
    if (x1 <= x0 || y1 <= y0 || (x1 - x0) * (y1 - y0) > PARTIAL_MOST * spanW * spanH)
        return std::nullopt;
    return CBox{x0, y0, x1 - x0, y1 - y0};
}

// The question's output as a coarse map of where it differs, read back without
// waiting, as the panes are.
void CLayer::mapChanges(const SP<Render::IFramebuffer>& answer) {
    auto&     program = g_pGlobalState->meniscusPrograms.changes;
    const int texelsW = static_cast<int>(m_fieldTexels.x);
    const int texelsH = static_cast<int>(m_fieldTexels.y);
    const int width   = (texelsW + CHANGE_BLOCK - 1) / CHANGE_BLOCK;
    const int height  = (texelsH + CHANGE_BLOCK - 1) / CHANGE_BLOCK;
    ensureExact(m_changeMap, "hyprglass-changes", width, height, DRM_FORMAT_ARGB8888);

    beginPass(program, m_changeMap, width, height);
    bindTexture(GL_TEXTURE0, answer->getTexture());
    glUniform2i(program.uniform("highTexel"), texelsW - 1, texelsH - 1);
    glUniform1i(program.uniform("blockSize"), CHANGE_BLOCK);
    glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);

    if (!m_changeBuffer)
        glGenBuffers(1, &m_changeBuffer);
    const size_t bytes        = static_cast<size_t>(width) * static_cast<size_t>(height) * 4;
    GLint        previousRead = 0;
    glGetIntegerv(GL_READ_FRAMEBUFFER_BINDING, &previousRead);
    glBindFramebuffer(GL_READ_FRAMEBUFFER, fbId(m_changeMap));
    glBindBuffer(GL_PIXEL_PACK_BUFFER, m_changeBuffer);
    glBufferData(GL_PIXEL_PACK_BUFFER, static_cast<GLsizeiptr>(bytes), nullptr, GL_STREAM_READ);
    glReadPixels(0, 0, width, height, GL_RGBA, GL_UNSIGNED_BYTE, nullptr);
    glBindBuffer(GL_PIXEL_PACK_BUFFER, 0);
    glBindFramebuffer(GL_READ_FRAMEBUFFER, static_cast<GLuint>(previousRead));

    if (m_changeFence)
        glDeleteSync(m_changeFence);
    m_changeFence = glFenceSync(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
    m_changeCells = Vector2D{static_cast<double>(width), static_cast<double>(height)};
}

// The field pixels the marks of the map read back span; none when nothing is marked.
std::optional<CBox> CLayer::readChanges() {
    const int    width  = static_cast<int>(m_changeCells.x);
    const int    height = static_cast<int>(m_changeCells.y);
    const size_t bytes  = static_cast<size_t>(width) * static_cast<size_t>(height) * 4;

    std::optional<CBox> moved;
    glBindBuffer(GL_PIXEL_PACK_BUFFER, m_changeBuffer);
    if (const auto* mapped = static_cast<const uint8_t*>(glMapBufferRange(GL_PIXEL_PACK_BUFFER, 0, static_cast<GLsizeiptr>(bytes), GL_MAP_READ_BIT))) {
        int x0 = width;
        int y0 = height;
        int x1 = -1;
        int y1 = -1;
        for (int y = 0; y < height; ++y) {
            for (int x = 0; x < width; ++x) {
                if (mapped[(static_cast<size_t>(y) * static_cast<size_t>(width) + static_cast<size_t>(x)) * 4] < 128)
                    continue;
                x0 = std::min(x0, x);
                y0 = std::min(y0, y);
                x1 = std::max(x1, x);
                y1 = std::max(y1, y);
            }
        }
        glUnmapBuffer(GL_PIXEL_PACK_BUFFER);
        if (x1 >= 0) {
            const double per = static_cast<double>(CHANGE_BLOCK) * m_fieldScale;
            moved            = CBox{x0 * per, y0 * per, (x1 - x0 + 1) * per, (y1 - y0 + 1) * per};
        }
    }
    glBindBuffer(GL_PIXEL_PACK_BUFFER, 0);
    return moved;
}

// What a build of a layer that drew nothing leaves: no outline, no point of one
// for the flood to name, a field of nothing, no panes in the map.
void CLayer::startEmpty(const SP<Render::IFramebuffer>& caller) {
    const int width  = static_cast<int>(m_fieldTexels.x);
    const int height = static_cast<int>(m_fieldTexels.y);
    ensureExact(m_field, "hyprglass-field", width, height, DRM_FORMAT_ARGB8888);
    ensureExact(m_cells, "hyprglass-cells", static_cast<int>(m_cellCount.x), static_cast<int>(m_cellCount.y), DRM_FORMAT_ARGB8888);
    ensureExact(m_outline, "hyprglass-outline", width, height, DRM_FORMAT_ARGB8888);

    beginOffscreen();
    const auto clear = [](const SP<Render::IFramebuffer>& target, float r, float g, float b, float a) {
        glBindFramebuffer(GL_FRAMEBUFFER, fbId(target));
        glClearColor(r, g, b, a);
        glClear(GL_COLOR_BUFFER_BIT);
    };
    // field.frag's texel with no coverage and no gradient: the normal's zero, half way
    clear(m_field, 0.5f, 0.5f, 0.0f, 1.0f);
    clear(m_cells, 0.0f, 0.0f, 0.0f, 1.0f);
    clear(m_outline, 0.0f, 0.0f, 0.0f, 1.0f);
    if (m_params.quartz != 0) {
        const int seedW = static_cast<int>(std::ceil(m_boxSize.x)) + 2 * m_margin;
        const int seedH = static_cast<int>(std::ceil(m_boxSize.y)) + 2 * m_margin;
        ensureExact(m_seeds, "hyprglass-seeds", seedW, seedH, DRM_FORMAT_ARGB8888);
        m_seedTexels = Vector2D{static_cast<double>(seedW), static_cast<double>(seedH)};
        // every byte at its highest: the flood's NONE
        clear(m_seeds, 1.0f, 1.0f, 1.0f, 1.0f);
    } else
        m_seeds.reset();
    endOffscreen(caller);

    m_fieldValid = true;
    if (m_fence)
        m_cellsStale = true;
    else
        requestPanes();
}

bool CLayer::buildField(const SMaskSource& mask, const SP<Render::IFramebuffer>& caller) {
    Diagnostics::CScopedStageTimer stageTimer(Diagnostics::EStage::ModelField);
    Diagnostics::recordModelFieldBuild();

    auto&       global   = *g_pGlobalState;
    auto&       programs = global.meniscusPrograms;
    auto&       scratch  = global.meniscusScratch;
    const int   width    = static_cast<int>(m_fieldTexels.x);
    const int   height   = static_cast<int>(m_fieldTexels.y);
    const float sigma    = m_layout.blur_sigma;
    const int   radius   = m_layout.blur_radius;
    const bool  shadowOn = m_layout.shadow_on != 0;

    ensureAtLeast(scratch.maskA, "hyprglass-mask-a", width, height, DRM_FORMAT_ARGB8888);
    ensureAtLeast(scratch.maskB, "hyprglass-mask-b", width, height, DRM_FORMAT_ARGB8888);
    ensureExact(m_field, "hyprglass-field", width, height, DRM_FORMAT_ARGB8888);
    ensureExact(m_cells, "hyprglass-cells", static_cast<int>(m_cellCount.x), static_cast<int>(m_cellCount.y), DRM_FORMAT_ARGB8888);

    // Built in part, each pass writes what the next reads of it and no more: the
    // part itself, grown by what the passes after it reach. The rest of the field,
    // the outline and the map stay as the last build left them, which is what
    // they are, the outline not having moved there.
    const auto part = partialBuild();
    if (part)
        Diagnostics::recordModelFieldPartial();
    std::optional<STexels> covered, across, down, normals, outlined, cells;
    if (part) {
        outlined = texelsOf(*part, m_fieldScale, 0, width, height);
        normals  = texelsOf(*part, m_fieldScale, 1, width, height);
        down     = texelsOf(*part, m_fieldScale, 2, width, height);
        across   = texelsOf(*part, m_fieldScale, radius + 4, width, height);
        covered  = texelsOf(*part, m_fieldScale, 2 * radius + 6, width, height);
        cells    = texelsOf(*part, GLASS_CELL_PIXELS, 0, static_cast<int>(m_cellCount.x), static_cast<int>(m_cellCount.y));
    }

    beginOffscreen();
    const SScissorKept kept;

    // 1. coverage: which of the layer's pixels are pane
    {
        auto& program = programs.coverage;
        beginPass(program, scratch.maskA, width, height);
        scissorTo(covered);
        bindMask(GL_TEXTURE0, mask.texture);
        glUniform2f(program.uniform("boxSize"), static_cast<float>(m_boxSize.x), static_cast<float>(m_boxSize.y));
        glUniform2f(program.uniform("margin"), static_cast<float>(m_margin), static_cast<float>(m_margin));
        glUniform2f(program.uniform("maskUvScale"), static_cast<float>(mask.uvScale.x), static_cast<float>(mask.uvScale.y));
        glUniform1f(program.uniform("threshold"), mask.threshold);
        glUniform1i(program.uniform("scale"), m_fieldScale);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    }

    // Apple's shader needs true distance from the outline, from the same coverage.
    if (m_params.quartz != 0)
        floodOutline(mask, part);
    else
        m_seeds.reset();

    // The shadow's shape, from the same coverage before the blur below overwrites it:
    // reduced across while blurred across, then down while blurred down.
    if (shadowOn) {
        scissorTo(std::nullopt);
        const int   shadowW     = static_cast<int>(m_shadowTexels.x);
        const int   shadowH     = static_cast<int>(m_shadowTexels.y);
        const float shadowSigma = m_layout.shadow_sigma;
        const int   shadowTaps  = m_layout.shadow_taps;
        ensureAtLeast(scratch.shadowA, "hyprglass-shadow-a", shadowW, height, DRM_FORMAT_ARGB8888);
        ensureExact(m_shadow, "hyprglass-shadow", shadowW, shadowH, DRM_FORMAT_ARGB8888);

        auto&      program = programs.shadowBlur;
        const auto pass    = [&](const SP<Render::IFramebuffer>& source, int sourceW, int sourceH, const SP<Render::IFramebuffer>& target, int targetW, int targetH, bool across) {
            const float allocW = static_cast<float>(source->m_size.x);
            const float allocH = static_cast<float>(source->m_size.y);
            beginPass(program, target, targetW, targetH);
            bindTexture(GL_TEXTURE0, source->getTexture());
            glUniform2f(program.uniform("texelStep"), across ? 1.0f / allocW : 0.0f, across ? 0.0f : 1.0f / allocH);
            glUniform2f(program.uniform("uvLow"), 0.5f / allocW, 0.5f / allocH);
            glUniform2f(program.uniform("uvHigh"), (static_cast<float>(sourceW) - 0.5f) / allocW, (static_cast<float>(sourceH) - 0.5f) / allocH);
            glUniform2f(program.uniform("uvScale"), static_cast<float>(sourceW) / allocW, static_cast<float>(sourceH) / allocH);
            glUniform2f(program.uniform("uvOffset"), 0.0f, 0.0f);
            glUniform1f(program.uniform("sigma"), shadowSigma);
            glUniform1i(program.uniform("radius"), shadowTaps);
            glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
        };
        pass(scratch.maskA, width, height, scratch.shadowA, shadowW, height, true);
        pass(scratch.shadowA, shadowW, height, m_shadow, shadowW, shadowH, false);
    }

    // 2 and 3. coverage blurred, which is what stands for distance from the outline
    {
        auto&      program = programs.fieldBlur;
        const auto pass    = [&](const SP<Render::IFramebuffer>& source, const SP<Render::IFramebuffer>& target, bool alongRows, bool packedInput,
                              const std::optional<STexels>& writes, const std::optional<STexels>& reads) {
            beginPass(program, target, width, height);
            scissorTo(writes);
            bindTexture(GL_TEXTURE0, source->getTexture());
            glUniform2i(program.uniform("direction"), alongRows ? 1 : 0, alongRows ? 0 : 1);
            boundsTo(program, reads, width, height);
            glUniform1f(program.uniform("sigma"), std::max(sigma, 0.3f));
            glUniform1i(program.uniform("radius"), radius);
            glUniform1i(program.uniform("packedInput"), packedInput ? 1 : 0);
            glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
        };
        pass(scratch.maskA, scratch.maskB, true, false, across, covered);
        pass(scratch.maskB, scratch.maskA, false, true, down, across);
    }

    // 4. the field: inward normal and blurred coverage
    {
        auto& program = programs.field;
        beginPass(program, m_field, width, height);
        scissorTo(normals);
        bindTexture(GL_TEXTURE0, scratch.maskA->getTexture());
        boundsTo(program, down, width, height);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    }

    // What the outline looks like now, for the question a later commit will raise.
    ensureExact(m_outline, "hyprglass-outline", width, height, DRM_FORMAT_ARGB8888);
    scissorTo(outlined);
    drawOutline(mask, m_outline, false);

    // 5. the coarse map of where panes are
    {
        auto& program = programs.cells;
        beginPass(program, m_cells, static_cast<int>(m_cellCount.x), static_cast<int>(m_cellCount.y));
        scissorTo(cells);
        bindTexture(GL_TEXTURE0, m_field->getTexture());
        boundsTo(program, normals, width, height);
        glUniform1i(program.uniform("cellSize"), GLASS_CELL_PIXELS / m_fieldScale);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    }

    endOffscreen(caller);
    m_fieldValid = true;
    return part.has_value();
}

// Asks for the pane map without waiting for it: the pixels go to a buffer the
// GPU fills when it gets there, and a fence says when it has.
void CLayer::requestPanes() {
    if (!m_cells)
        return;

    const int    width  = static_cast<int>(m_cellCount.x);
    const int    height = static_cast<int>(m_cellCount.y);
    const size_t bytes  = static_cast<size_t>(width) * static_cast<size_t>(height) * 4;

    if (!m_pixelBuffer)
        glGenBuffers(1, &m_pixelBuffer);

    // This runs inside a frame and, from poll(), between frames: whatever is bound
    // for reading is put back, and Hyprland's own reads expect no pack buffer bound.
    GLint previousRead = 0;
    glGetIntegerv(GL_READ_FRAMEBUFFER_BINDING, &previousRead);
    glBindFramebuffer(GL_READ_FRAMEBUFFER, fbId(m_cells));
    glBindBuffer(GL_PIXEL_PACK_BUFFER, m_pixelBuffer);
    glBufferData(GL_PIXEL_PACK_BUFFER, static_cast<GLsizeiptr>(bytes), nullptr, GL_STREAM_READ);
    glReadPixels(0, 0, width, height, GL_RGBA, GL_UNSIGNED_BYTE, nullptr);
    glBindBuffer(GL_PIXEL_PACK_BUFFER, 0);
    glBindFramebuffer(GL_READ_FRAMEBUFFER, static_cast<GLuint>(previousRead));

    m_fence        = glFenceSync(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
    m_pendingCells = m_cellCount;
    // What was redrawn before this field was built is in the map; nothing after it is.
    m_unknownAfterRequest.reset();
    m_cellsStale = false;
    Diagnostics::recordModelPaneReadback();
}

void CLayer::pollPanes() {
    if (!m_fence)
        return;

    const GLenum state = glClientWaitSync(m_fence, 0, 0);
    if (state == GL_TIMEOUT_EXPIRED)
        return;

    glDeleteSync(m_fence);
    m_fence = nullptr;

    if (state == GL_ALREADY_SIGNALED || state == GL_CONDITION_SATISFIED) {
        const size_t         bytes = static_cast<size_t>(m_pendingCells.x) * static_cast<size_t>(m_pendingCells.y) * 4;
        std::vector<uint8_t> cells(bytes);
        glBindBuffer(GL_PIXEL_PACK_BUFFER, m_pixelBuffer);
        if (const void* mapped = glMapBufferRange(GL_PIXEL_PACK_BUFFER, 0, static_cast<GLsizeiptr>(bytes), GL_MAP_READ_BIT)) {
            std::memcpy(cells.data(), mapped, bytes);
            glUnmapBuffer(GL_PIXEL_PACK_BUFFER);
            // An answer for a layout that has since changed describes nothing that is drawn now.
            if (m_pendingCells == m_cellCount)
                adoptCells(cells);
        }
        glBindBuffer(GL_PIXEL_PACK_BUFFER, 0);
    }

    // The field was rebuilt while the answer was on its way: ask again for the current one.
    if (m_cellsStale && m_fieldValid && !m_contentDirty)
        requestPanes();
}

void CLayer::adoptCells(const std::vector<uint8_t>& cells) {
    const int width  = static_cast<int>(m_cellCount.x);
    const int height = static_cast<int>(m_cellCount.y);

    GlassMapRequest request{};
    request.cells       = GlassBytes{cells.data(), cells.size()};
    request.width       = static_cast<size_t>(width);
    request.height      = static_cast<size_t>(height);
    request.margin      = m_margin;
    request.field_scale = m_fieldScale;
    request.sigma       = m_layout.field_sigma;
    request.bezel_width = m_params.lens.bezel_width;
    request.box_w       = m_boxSize.x;
    request.box_h       = m_boxSize.y;

    // Room for the usual few panes; a map with more says how much it needs, and is read again.
    std::vector<GlassPane> panes(32);
    std::vector<GlassRect> interiors(64);
    std::vector<uint8_t>   fit(static_cast<size_t>(width) * static_cast<size_t>(height));
    GlassPaneBuffers       out{};
    const auto             read = [&] {
        out                   = GlassPaneBuffers{};
        out.panes             = panes.data();
        out.pane_capacity     = panes.size();
        out.interiors         = interiors.data();
        out.interior_capacity = interiors.size();
        out.fit               = fit.data();
        out.fit_capacity      = fit.size();
        return glass_panes_from_cells(&request, &out);
    };
    int code = read();
    if (code == GLASS_ERR_CAPACITY) {
        panes.resize(out.pane_count);
        interiors.resize(out.interior_count);
        code = read();
    }
    if (code != GLASS_OK)
        return;

    uploadFit(fit, width, height);

    m_knownPanes = toPanes(panes.data(), out.pane_count, interiors.data());
    m_knownPaneList.assign(panes.begin(), panes.begin() + static_cast<std::ptrdiff_t>(out.pane_count));
    m_knownInteriors.assign(interiors.begin(), interiors.begin() + static_cast<std::ptrdiff_t>(out.interior_count));
    m_panesKnown = true;
    m_mapAdopted = true;
    m_unknown    = m_unknownAfterRequest;
    m_unknownAfterRequest.reset();
}

CBox CLayer::sampleRect(const SPane& pane, const Vector2D& boxPosition, const Vector2D& sourceSize) const {
    // Apple's shader reads further, and its copy is this one rectangle: whether it is
    // fresh, and whether it has moved, is asked of the same rectangle it copies.
    if (m_params.quartz != 0)
        return quartzRect(pane, boxPosition, sourceSize);
    // where the frost is two copies, it is the wide one that is reduced and blurred
    const float            sized = m_params.layered.frost_wide > 0.0f ? m_params.layered.frost_wide : m_params.frost;
    const GlassSampleQuery query{Glass::toRect(pane.box), boxPosition.x, boxPosition.y, sourceSize.x, sourceSize.y, sized, m_reaches.frost};
    GlassRect              rect{};
    if (glass_sample_rect(&query, &rect) != GLASS_OK)
        return CBox{};
    return Glass::toBox(rect);
}

void CLayer::sample(SPane& pane, const SP<Render::IFramebuffer>& source, const Vector2D& boxPosition, const SP<Render::IFramebuffer>& caller,
                    const SCleanSource* clean) {
    if (!source)
        return;
    if (m_params.quartz != 0) {
        sampleQuartz(pane, source, boxPosition, caller, clean);
        return;
    }

    const CBox rect = sampleRect(pane, boxPosition, source->m_size);
    if (rect.w < 1.0 || rect.h < 1.0) {
        pane.hasSample = false;
        return;
    }

    Diagnostics::CScopedStageTimer stageTimer(Diagnostics::EStage::ModelSample);

    auto&      global    = *g_pGlobalState;
    auto&      programs  = global.meniscusPrograms;
    auto&      scratch   = global.meniscusScratch;
    // The current material: the copy made here is its cloud, in which bright and dark
    // weigh differently, and the page as it is goes into a texture of its own.
    const bool layered   = m_params.layered.frost_wide > 0.0f;
    GlassFrostPlan plan{};
    (void)glass_frost_plan(layered ? m_params.layered.frost_wide : m_params.frost, &plan);
    const int  reduction = plan.reduction;
    // the weight of a texel travels in alpha from the reduction to the last blur, which divides by it
    const bool weighted  = layered && reduction > 1 && plan.face_applied != 0 && std::abs(m_params.layered.cloud_weight - 1.0f) > 1e-4f;
    const int  x         = static_cast<int>(rect.x);
    const int  y         = static_cast<int>(rect.y);
    const int  fullW     = static_cast<int>(rect.w);
    const int  fullH     = static_cast<int>(rect.h);
    const int  width     = fullW / reduction;
    const int  height    = fullH / reduction;
    const auto format    = weighted ? DRM_FORMAT_ARGB8888 : source->m_drmFormat;

    if (const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock())
        Diagnostics::recordSampledPixels(monitor->m_id, static_cast<double>(fullW) * static_cast<double>(fullH));

    ensureExact(pane.frost, "hyprglass-frost", width, height, format);

    beginOffscreen();

    if (reduction == 1) {
        // nothing to reduce: a plain copy, which a blit does without a shader
        glBindFramebuffer(GL_READ_FRAMEBUFFER, fbId(source));
        glBindFramebuffer(GL_DRAW_FRAMEBUFFER, fbId(pane.frost));
        glBlitFramebuffer(x, y, x + fullW, y + fullH, 0, 0, width, height, GL_COLOR_BUFFER_BIT, GL_NEAREST);
    } else {
        auto& program = programs.reduce;
        beginPass(program, pane.frost, width, height);
        bindTexture(GL_TEXTURE0, source->getTexture());
        glUniform2f(program.uniform("sourceOrigin"), static_cast<float>(x), static_cast<float>(y));
        glUniform2f(program.uniform("invSourceSize"), 1.0f / static_cast<float>(source->m_size.x), 1.0f / static_cast<float>(source->m_size.y));
        glUniform1i(program.uniform("scale"), reduction);
        glUniform1f(program.uniform("whiteWeight"), weighted ? m_params.layered.cloud_weight : 0.0f);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    }

    // The reduction and the filtered read of the glass shader already spread light by
    // about half a texel each way: the Gaussian supplies the rest of the frost.
    // A frost of two copies gets its face where the two are put together, in the
    // shaders that draw the pane, and not here.
    pane.layered     = layered;
    pane.faceApplied = plan.face_applied != 0 && !layered;
    if (plan.face_applied != 0) {
        auto& other = weighted ? scratch.cloud : scratch.blur;
        ensureAtLeast(other, weighted ? "hyprglass-cloud-blur" : "hyprglass-frost-blur", width, height, format);

        // More than a few filtered fetches a side would cut the kernel short: it runs more than once instead.
        const int   passes    = plan.passes;

        auto& program = programs.frostBlur;
        // the face belongs to the last pass only, and its values do not change between passes
        auto shader = g_pHyprOpenGL->useShader(program.shader);
        glUniform1f(program.uniform("saturation"), m_params.saturation);
        glUniform1f(program.uniform("brightness"), m_params.brightness);
        glUniform1f(program.uniform("contrast"), m_params.contrast);
        glUniform3f(program.uniform("tintColor"), m_params.tint_color[0], m_params.tint_color[1], m_params.tint_color[2]);
        glUniform1f(program.uniform("tintOverDark"), m_params.tint_over_dark);
        glUniform1f(program.uniform("tintOverBright"), m_params.tint_over_bright);

        // what the last pass does besides: the face, or the division a weighted copy is owed
        frostBlurPasses(program, pane.frost, other, width, height, plan, weighted ? 2 : layered ? 0 : 1);

        if (const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock())
            Diagnostics::recordBlurPasses(monitor->m_id, static_cast<uint64_t>(passes) * 2);
    }

    // The page as it is, under the pane and a little beyond it: the narrow copy is
    // read from it around each pixel, and the hairline just outside the outline.
    if (layered) {
        const int  margin = static_cast<int>(std::ceil(m_params.frost + 2.5f * m_params.layered.third)) + 2;
        const CBox wanted = pane.box.copy().translate(boxPosition).expand(margin).intersection(CBox{{}, source->m_size});
        const int  sx     = static_cast<int>(std::floor(wanted.x));
        const int  sy     = static_cast<int>(std::floor(wanted.y));
        const int  sw     = static_cast<int>(std::ceil(wanted.x + wanted.w)) - sx;
        const int  sh     = static_cast<int>(std::ceil(wanted.y + wanted.h)) - sy;
        if (sw >= 1 && sh >= 1) {
            ensureExact(pane.sharp, "hyprglass-sharp", sw, sh, source->m_drmFormat);
            glBindFramebuffer(GL_READ_FRAMEBUFFER, fbId(source));
            glBindFramebuffer(GL_DRAW_FRAMEBUFFER, fbId(pane.sharp));
            glBlitFramebuffer(sx, sy, sx + sw, sy + sh, 0, 0, sw, sh, GL_COLOR_BUFFER_BIT, GL_NEAREST);
            pane.sharpRect = CBox{static_cast<double>(sx), static_cast<double>(sy), static_cast<double>(sw), static_cast<double>(sh)};
        } else {
            pane.layered = false;
        }
    } else {
        pane.sharp.reset();
    }

    endOffscreen(caller);

    pane.sampled   = rect;
    pane.reduction = reduction;
    pane.hasSample = true;
    pane.dirty     = false;
}

// What the two shaders that draw a pane are told of the material, whatever the pane.
void CLayer::setMaterialUniforms(CProgram& program) const {
    const auto& layered = m_params.layered;
    glUniform1i(program.uniform("sharpTex"), 3);
    glUniform1f(program.uniform("ghostSigma"), m_params.frost);
    glUniform1f(program.uniform("ghostWeight"), layered.ghost_weight);
    glUniform1f(program.uniform("frostCloud"), layered.frost_cloud);
    glUniform1f(program.uniform("frostHold"), layered.frost_hold);
    glUniform1f(program.uniform("saturation"), m_params.saturation);
    glUniform1f(program.uniform("brightness"), m_params.brightness);
    glUniform1f(program.uniform("contrast"), m_params.contrast);
    glUniform3f(program.uniform("tintColor"), m_params.tint_color[0], m_params.tint_color[1], m_params.tint_color[2]);
    glUniform1f(program.uniform("tintOverDark"), m_params.tint_over_dark);
    glUniform1f(program.uniform("tintOverBright"), m_params.tint_over_bright);
    glUniform1f(program.uniform("adaptiveSaturation"), layered.adaptive_saturation);
    glUniform1f(program.uniform("bodyShade"), layered.body_shade);
}

// And of one pane: whether its frost is two copies, and where the sharp one is.
void CLayer::setPaneUniforms(CProgram& program, const SPane& pane, const CBox& layerBox) const {
    const bool layered = pane.layered && pane.sharp;
    glUniform1i(program.uniform("layered"), layered ? 1 : 0);
    if (!layered)
        return;
    const float sharpW = static_cast<float>(pane.sharp->m_size.x);
    const float sharpH = static_cast<float>(pane.sharp->m_size.y);
    bindTexture(GL_TEXTURE3, pane.sharp->getTexture());
    glActiveTexture(GL_TEXTURE0);
    glUniform2f(program.uniform("sharpOrigin"), static_cast<float>(pane.sharpRect.x - layerBox.x), static_cast<float>(pane.sharpRect.y - layerBox.y));
    glUniform2f(program.uniform("invSharpSize"), 1.0f / sharpW, 1.0f / sharpH);
    glUniform2f(program.uniform("sharpUvLow"), 0.5f / sharpW, 0.5f / sharpH);
    glUniform2f(program.uniform("sharpUvHigh"), 1.0f - 0.5f / sharpW, 1.0f - 0.5f / sharpH);
}

void CLayer::drawShadows(const SP<Render::IFramebuffer>& target, const CBox& layerBox, float alpha) {
    const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
    if (!monitor || !m_shadow)
        return;

    GlassOpacities opacities{};
    (void)glass_opacities(&m_params, alpha, &opacities);

    const CBox bounds = CBox{layerBox.x - m_margin, layerBox.y - m_margin, layerBox.w + 2.0 * m_margin, layerBox.h + 2.0 * m_margin}.intersection(CBox{{}, target->m_size});

    // Everything a pane's shadow can reach, less the faces the glass will cover whole.
    CRegion region;
    for (const auto& pane : m_panes) {
        if (pane.hasSample)
            region.add(pane.box.copy().translate(layerBox.pos()).expand(m_shadowReach));
    }
    for (const auto& pane : m_panes) {
        for (const auto& interior : pane.interiors)
            region.subtract(interior.copy().translate(layerBox.pos()));
    }
    region.intersect(bounds).intersect(g_pHyprRenderer->m_renderData.damage);
    if (region.empty())
        return;

    auto&      program   = g_pGlobalState->meniscusPrograms.shadow;
    const auto transform = Math::wlTransformToHyprutils(Math::invertTransform(monitor->m_transform));
    auto       shader    = g_pHyprOpenGL->useShader(program.shader);
    shader->setUniformInt(SHADER_TEX, 0);
    glBindVertexArray(shader->getUniformLocation(SHADER_SHADER_VAO));
    bindTexture(GL_TEXTURE0, m_shadow->getTexture());
    glUniform2f(program.uniform("fieldMargin"), static_cast<float>(m_margin), static_cast<float>(m_margin));
    glUniform2f(program.uniform("invShadowSize"), 1.0f / (static_cast<float>(m_shadowTexels.x) * GLASS_SHADOW_PIXELS), 1.0f / (static_cast<float>(m_shadowTexels.y) * GLASS_SHADOW_PIXELS));
    glUniform2f(program.uniform("shadowOffset"), 0.0f, m_params.shadow_offset);
    glUniform1f(program.uniform("shadowOpacity"), opacities.shadow);

    for (const auto& quad : quadsOf(region)) {
        Mat3x3 matrix = g_pHyprRenderer->projectBoxToTarget(quad, transform);
        matrix.transpose();
        shader->setUniformMatrix3fv(SHADER_PROJ, 1, GL_FALSE, matrix.getMatrix());
        glUniform2f(program.uniform("quadOrigin"), static_cast<float>(quad.x - layerBox.x), static_cast<float>(quad.y - layerBox.y));
        glUniform2f(program.uniform("quadSize"), static_cast<float>(quad.w), static_cast<float>(quad.h));
        Diagnostics::recordGlassPixels(monitor->m_id, quad.w * quad.h);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    }
}

void CLayer::draw(const SP<Render::IFramebuffer>& target, const CBox& layerBox, const SMaskSource& mask, float alpha) {
    if (!target || !m_fieldValid || !mask.texture)
        return;

    const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
    if (!monitor)
        return;

    Diagnostics::CScopedStageTimer stageTimer(Diagnostics::EStage::ModelGlass);

    auto&       programs  = g_pGlobalState->meniscusPrograms;
    const bool  shadowOn  = m_params.shadow_opacity > 0.0f && m_params.shadow_radius > 0.0f && m_shadow;
    const auto  transform = Math::wlTransformToHyprutils(Math::invertTransform(monitor->m_transform));
    const CBox  bounds    = CBox{layerBox.x, layerBox.y, layerBox.w, layerBox.h}.intersection(CBox{{}, target->m_size});
    GlassOpacities opacities{};
    (void)glass_opacities(&m_params, alpha, &opacities);
    const float opacity = opacities.glass;
    // what the glass shader is told besides geometry: the lens's reach per thickness, the coverage
    // thresholds, whether red and blue bend apart, and the direction of the light
    GlassDrawConstants constants{};
    (void)glass_draw_constants(&m_params, m_deviationScale, &constants);
    const auto& damage    = g_pHyprRenderer->m_renderData.damage;

    glBindFramebuffer(GL_FRAMEBUFFER, fbId(target));
    g_pHyprRenderer->blend(true);
    // every quad below is cut to the damage by its own rectangle
    g_pHyprOpenGL->scissor(nullptr);

    // Apple's own shader draws the whole pane, shadow included, in one pass.
    if (m_params.quartz != 0) {
        drawQuartz(target, layerBox, mask, opacity);
        return;
    }

    // Only damaged pixels reach the screen, so nothing outside the damage is shaded.
    // 1. the shadows, under everything
    if (shadowOn)
        drawShadows(target, layerBox, alpha);

    // 2. the faces: the inside of each pane, one read per pixel
    {
        auto& program = programs.face;
        auto  shader  = g_pHyprOpenGL->useShader(program.shader);
        shader->setUniformInt(SHADER_TEX, 0);
        glBindVertexArray(shader->getUniformLocation(SHADER_SHADER_VAO));
        glUniform1f(program.uniform("opacity"), opacity);
        glUniform1f(program.uniform("noiseStrength"), m_params.noise_strength);
        setMaterialUniforms(program);
        // an opaque face replaces what is under it: no need to read it back to blend
        if (opacity >= 0.999f)
            g_pHyprRenderer->blend(false);

        for (const auto& pane : m_panes) {
            if (!pane.hasSample || !pane.frost || !(pane.faceApplied || pane.layered) || pane.interiors.empty())
                continue;

            CRegion region;
            for (const auto& interior : pane.interiors)
                region.add(interior.copy().translate(layerBox.pos()));
            region.intersect(bounds).intersect(damage);
            if (region.empty())
                continue;

            setPaneUniforms(program, pane, layerBox);
            bindTexture(GL_TEXTURE0, pane.frost->getTexture());
            glUniform2f(program.uniform("frostOrigin"), static_cast<float>(pane.sampled.x - layerBox.x), static_cast<float>(pane.sampled.y - layerBox.y));
            glUniform2f(program.uniform("invFrostSize"), 1.0f / static_cast<float>(pane.sampled.w), 1.0f / static_cast<float>(pane.sampled.h));
            for (const auto& quad : quadsOf(region)) {
                Mat3x3 matrix = g_pHyprRenderer->projectBoxToTarget(quad, transform);
                matrix.transpose();
                shader->setUniformMatrix3fv(SHADER_PROJ, 1, GL_FALSE, matrix.getMatrix());
                glUniform2f(program.uniform("quadOrigin"), static_cast<float>(quad.x - layerBox.x), static_cast<float>(quad.y - layerBox.y));
                glUniform2f(program.uniform("quadSize"), static_cast<float>(quad.w), static_cast<float>(quad.h));
                Diagnostics::recordGlassPixels(monitor->m_id, quad.w * quad.h);
                glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
            }
        }
        g_pHyprRenderer->blend(true);
    }

    // 3. the rest of each pane: its rim, with the lens, the edge and the light
    auto& program = programs.glass;
    auto  shader  = g_pHyprOpenGL->useShader(program.shader);
    shader->setUniformInt(SHADER_TEX, 0);
    glBindVertexArray(shader->getUniformLocation(SHADER_SHADER_VAO));

    bindTexture(GL_TEXTURE1, m_field->getTexture());
    bindMask(GL_TEXTURE2, mask.texture);
    glActiveTexture(GL_TEXTURE5);
    glBindTexture(GL_TEXTURE_2D, m_fitTexture);
    glActiveTexture(GL_TEXTURE4);
    glBindTexture(GL_TEXTURE_2D, m_lensTable);
    glUniform1i(program.uniform("fieldTex"), 1);
    glUniform1i(program.uniform("maskTex"), 2);
    glUniform1i(program.uniform("lensTable"), 4);
    glUniform1i(program.uniform("fitTex"), 5);

    const float fieldSpanX = static_cast<float>(m_fieldTexels.x) * m_fieldScale;
    const float fieldSpanY = static_cast<float>(m_fieldTexels.y) * m_fieldScale;

    glUniform2f(program.uniform("boxSize"), static_cast<float>(layerBox.w), static_cast<float>(layerBox.h));
    glUniform2f(program.uniform("maskUvScale"), static_cast<float>(mask.uvScale.x), static_cast<float>(mask.uvScale.y));
    glUniform2f(program.uniform("fieldMargin"), static_cast<float>(m_margin), static_cast<float>(m_margin));
    glUniform2f(program.uniform("invFieldSize"), 1.0f / fieldSpanX, 1.0f / fieldSpanY);
    // one value for the whole layer spans it whatever its size; a map spans its cells
    if (m_fitCells.x > 1.0 || m_fitCells.y > 1.0)
        glUniform2f(program.uniform("invFitSize"), 1.0f / static_cast<float>(m_fitCells.x * GLASS_CELL_PIXELS), 1.0f / static_cast<float>(m_fitCells.y * GLASS_CELL_PIXELS));
    else
        glUniform2f(program.uniform("invFitSize"), 0.0f, 0.0f);
    glUniform1f(program.uniform("bezelWidth"), static_cast<float>(m_params.lens.bezel_width));
    glUniform1f(program.uniform("thickness"), static_cast<float>(m_params.lens.thickness));
    glUniform1f(program.uniform("thicknessPerFit"), constants.thickness_per_fit);
    glUniform1f(program.uniform("maskThreshold"), mask.threshold);
    glUniform1f(program.uniform("maskFeather"), mask.feather);
    glUniform1f(program.uniform("interiorCoverage"), constants.interior_coverage);
    glUniform1f(program.uniform("rimCoverage"), constants.rim_coverage);
    glUniform1f(program.uniform("outerCoverage"), constants.outer_coverage);
    glUniform1f(program.uniform("solidCoverage"), constants.solid_coverage);
    glUniform1f(program.uniform("deviationScale"), constants.deviation_scale);
    glUniform1f(program.uniform("tableTexels"), static_cast<float>(GLASS_LENS_TABLE_SIZE));
    glUniform1i(program.uniform("dispersive"), constants.dispersive ? 1 : 0);
    glUniform1f(program.uniform("opacity"), opacity);
    setMaterialUniforms(program);
    glUniform1f(program.uniform("rimLight"), m_params.layered.rim_light);
    glUniform1f(program.uniform("rimShade"), m_params.layered.rim_shade);
    glUniform1f(program.uniform("rimShadeEnds"), m_params.layered.rim_shade_ends);
    glUniform1f(program.uniform("edgeAbsorption"), m_params.layered.edge_absorption);
    glUniform1f(program.uniform("third"), m_params.layered.third);
    glUniform1f(program.uniform("noiseStrength"), m_params.noise_strength);
    glUniform2f(program.uniform("lightDirection"), constants.light_direction[0], constants.light_direction[1]);
    glUniform1f(program.uniform("lightIntensity"), m_params.light_intensity);
    glUniform1f(program.uniform("counterLight"), m_params.counter_light);
    glUniform1f(program.uniform("rimWidth"), m_params.rim_width);
    glUniform1f(program.uniform("rimGlow"), m_params.rim_glow);
    glUniform1f(program.uniform("innerShadow"), m_params.inner_shadow);

    for (const auto& pane : m_panes) {
        if (!pane.hasSample || !pane.frost)
            continue;

        CRegion region{pane.box.copy().translate(layerBox.pos())};
        // where the face pass has drawn, there is nothing left to do
        if (pane.faceApplied || pane.layered) {
            for (const auto& interior : pane.interiors)
                region.subtract(interior.copy().translate(layerBox.pos()));
        }
        region.intersect(bounds).intersect(damage);
        if (region.empty())
            continue;

        const float frostW = static_cast<float>(pane.frost->m_size.x);
        const float frostH = static_cast<float>(pane.frost->m_size.y);
        setPaneUniforms(program, pane, layerBox);
        bindTexture(GL_TEXTURE0, pane.frost->getTexture());
        glUniform2f(program.uniform("frostOrigin"), static_cast<float>(pane.sampled.x - layerBox.x), static_cast<float>(pane.sampled.y - layerBox.y));
        glUniform2f(program.uniform("invFrostSize"), 1.0f / static_cast<float>(pane.sampled.w), 1.0f / static_cast<float>(pane.sampled.h));
        glUniform2f(program.uniform("frostUvLow"), 0.5f / frostW, 0.5f / frostH);
        glUniform2f(program.uniform("frostUvHigh"), 1.0f - 0.5f / frostW, 1.0f - 0.5f / frostH);
        glUniform1i(program.uniform("faceApplied"), pane.faceApplied ? 1 : 0);

        for (const auto& quad : quadsOf(region)) {
            Mat3x3 matrix = g_pHyprRenderer->projectBoxToTarget(quad, transform);
            matrix.transpose();
            shader->setUniformMatrix3fv(SHADER_PROJ, 1, GL_FALSE, matrix.getMatrix());
            glUniform2f(program.uniform("quadOrigin"), static_cast<float>(quad.x - layerBox.x), static_cast<float>(quad.y - layerBox.y));
            glUniform2f(program.uniform("quadSize"), static_cast<float>(quad.w), static_cast<float>(quad.h));
            Diagnostics::recordGlassPixels(monitor->m_id, quad.w * quad.h);
            glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
        }
    }

    glActiveTexture(GL_TEXTURE5);
    glBindTexture(GL_TEXTURE_2D, 0);
    glActiveTexture(GL_TEXTURE4);
    glBindTexture(GL_TEXTURE_2D, 0);
    glActiveTexture(GL_TEXTURE3);
    glBindTexture(GL_TEXTURE_2D, 0);
    glActiveTexture(GL_TEXTURE0);
}

// ── Apple's own shader ────────────────────────────────────────────────────────

// The outline's distance field, one texel a device pixel, since the light of
// Apple's shader is a point wide: which pixels are pane (coverage.frag), the
// pixels of pane beside one that is not name themselves (quartzseed.frag), then
// each texel takes the nearest name among its neighbours a step away, the steps
// halving to one, and once more at one (quartzflood.frag).
void CLayer::floodOutline(const SMaskSource& mask, const std::optional<CBox>& part) {
    auto&     programs = g_pGlobalState->meniscusPrograms;
    auto&     scratch  = g_pGlobalState->meniscusScratch;
    const int width    = static_cast<int>(std::ceil(m_boxSize.x)) + 2 * m_margin;
    const int height   = static_cast<int>(std::ceil(m_boxSize.y)) + 2 * m_margin;
    ensureAtLeast(scratch.seedA, "hyprglass-seed-a", width, height, DRM_FORMAT_ARGB8888);
    ensureAtLeast(scratch.seedB, "hyprglass-seed-b", width, height, DRM_FORMAT_ARGB8888);
    ensureExact(m_seeds, "hyprglass-seeds", width, height, DRM_FORMAT_ARGB8888);
    m_seedTexels = Vector2D{static_cast<double>(width), static_cast<double>(height)};

    // Built in part, the flood stays inside the part: it holds every pane the map
    // knows and what moved, so it holds the outline nearest to each of their texels.
    std::optional<STexels> inside, covered;
    if (part) {
        inside  = texelsOf(*part, 1, 0, width, height);
        covered = texelsOf(*part, 1, 1, width, height);
    }

    {
        auto& program = programs.coverage;
        beginPass(program, scratch.seedB, width, height);
        scissorTo(covered);
        bindMask(GL_TEXTURE0, mask.texture);
        glUniform2f(program.uniform("boxSize"), static_cast<float>(m_boxSize.x), static_cast<float>(m_boxSize.y));
        glUniform2f(program.uniform("margin"), static_cast<float>(m_margin), static_cast<float>(m_margin));
        glUniform2f(program.uniform("maskUvScale"), static_cast<float>(mask.uvScale.x), static_cast<float>(mask.uvScale.y));
        glUniform1f(program.uniform("threshold"), mask.threshold);
        glUniform1i(program.uniform("scale"), 1);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    }
    {
        auto& program = programs.quartzSeed;
        beginPass(program, scratch.seedA, width, height);
        scissorTo(inside);
        bindTexture(GL_TEXTURE0, scratch.seedB->getTexture());
        boundsTo(program, covered, width, height);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    }

    // the largest power of two under the longer side of what is flooded, then halving, then one more at one
    const int        longer = inside ? std::max(inside->x1 - inside->x0, inside->y1 - inside->y0) : std::max(width, height);
    std::vector<int> steps;
    int              step = 1;
    for (int i = 0; i < 16 && step * 2 < longer; ++i)
        step *= 2;
    for (int i = 0; i < 16 && step >= 1; ++i, step /= 2)
        steps.push_back(step);
    steps.push_back(1);

    auto& program = programs.quartzFlood;
    auto* from    = &scratch.seedA;
    auto* to      = &scratch.seedB;
    for (size_t i = 0; i < steps.size(); ++i) {
        const bool last = i + 1 == steps.size();
        auto&      into = last ? m_seeds : *to;
        beginPass(program, into, width, height);
        scissorTo(inside);
        bindTexture(GL_TEXTURE0, (*from)->getTexture());
        boundsTo(program, inside, width, height);
        glUniform1i(program.uniform("stepSize"), steps[i]);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
        std::swap(from, to);
    }
}

bool copyWithMips(SP<Render::IFramebuffer>& page, const SP<Render::IFramebuffer>& source, const CBox& rect, const SP<Render::IFramebuffer>& caller) {
    const int sx = static_cast<int>(rect.x);
    const int sy = static_cast<int>(rect.y);
    const int sw = static_cast<int>(rect.w);
    const int sh = static_cast<int>(rect.h);
    if (!source || sw < 1 || sh < 1)
        return false;
    ensureExact(page, "hyprglass-quartz-page", sw, sh, source->m_drmFormat);

    beginOffscreen();
    glBindFramebuffer(GL_READ_FRAMEBUFFER, fbId(source));
    glBindFramebuffer(GL_DRAW_FRAMEBUFFER, fbId(page));
    glBlitFramebuffer(sx, sy, sx + sw, sy + sh, 0, 0, sw, sh, GL_COLOR_BUFFER_BIT, GL_NEAREST);

    // The texture is the engine's alone; the setter is Hyprland's, so its record stays true.
    const auto texture = page->getTexture();
    texture->bind();
    texture->setTexParameter(GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR);
    texture->setTexParameter(GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    texture->setTexParameter(GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    texture->setTexParameter(GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glGenerateMipmap(GL_TEXTURE_2D);
    endOffscreen(caller);
    return true;
}

// Gives `page` the mip chain Apple's shader blurs by reading. The texture is the
// engine's alone; the setter is Hyprland's, so its record stays true.
static void giveMips(const SP<Render::IFramebuffer>& page) {
    const auto texture = page->getTexture();
    glActiveTexture(GL_TEXTURE0);
    texture->bind();
    texture->setTexParameter(GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR);
    texture->setTexParameter(GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    texture->setTexParameter(GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    texture->setTexParameter(GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glGenerateMipmap(GL_TEXTURE_2D);
}

// The windows under the glass as the clean shader takes them: up to
// Beneath::MAX_RECTS boxes and their corners.
static void setUnderUniforms(CProgram& program, const std::vector<Beneath::SRect>& under) {
    std::array<float, Beneath::MAX_RECTS * 4> boxes{};
    std::array<float, Beneath::MAX_RECTS * 2> corners{};
    const size_t                              count = std::min(under.size(), Beneath::MAX_RECTS);
    for (size_t i = 0; i < count; ++i) {
        const auto& rect   = under[i];
        boxes[4 * i]       = static_cast<float>(rect.box.x);
        boxes[4 * i + 1]   = static_cast<float>(rect.box.y);
        boxes[4 * i + 2]   = static_cast<float>(rect.box.w);
        boxes[4 * i + 3]   = static_cast<float>(rect.box.h);
        corners[2 * i]     = rect.radius;
        corners[2 * i + 1] = rect.power;
    }
    glUniform1i(program.uniform("count"), static_cast<GLint>(count));
    glUniform4fv(program.uniform("boxes"), static_cast<GLsizei>(Beneath::MAX_RECTS), boxes.data());
    glUniform2fv(program.uniform("corners"), static_cast<GLsizei>(Beneath::MAX_RECTS), corners.data());
}

bool copyClean(SP<Render::IFramebuffer>& page, const SCleanSource& source, const CBox& rect, const SP<Render::IFramebuffer>& caller) {
    auto& programs = g_pGlobalState->meniscusPrograms;
    programs.initializeIfNeeded();
    const int sx = static_cast<int>(rect.x);
    const int sy = static_cast<int>(rect.y);
    const int sw = static_cast<int>(rect.w);
    const int sh = static_cast<int>(rect.h);
    if (!programs.ready() || !source.live || !source.backdrop || sw < 1 || sh < 1 || source.backdrop->m_size != source.live->m_size)
        return false;
    ensureExact(page, "hyprglass-quartz-page", sw, sh, source.live->m_drmFormat);

    beginOffscreen();
    auto& program = programs.quartzClean;
    beginPass(program, page, sw, sh);
    bindTexture(GL_TEXTURE0, source.live->getTexture());
    bindTexture(GL_TEXTURE1, source.backdrop->getTexture());
    glUniform1i(program.uniform("liveTex"), 0);
    glUniform1i(program.uniform("backdropTex"), 1);
    glUniform2i(program.uniform("origin"), sx, sy);
    setUnderUniforms(program, source.under);
    glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    glActiveTexture(GL_TEXTURE1);
    glBindTexture(GL_TEXTURE_2D, 0);

    giveMips(page);
    endOffscreen(caller);
    return true;
}

// The backdrop blurred for the face, on unit 5, where there is one: the shader
// reads it with one tap in place of a mip level of the page.
static void bindBlurred(CProgram& program, const SP<Render::IFramebuffer>& blurred) {
    glActiveTexture(GL_TEXTURE5);
    if (blurred)
        blurred->getTexture()->bind();
    else
        glBindTexture(GL_TEXTURE_2D, 0);
    glActiveTexture(GL_TEXTURE0);
    glUniform1i(program.uniform("blurTex"), 5);
    glUniform1i(program.uniform("preBlurred"), blurred ? 1 : 0);
}

// Uploads Apple's shader's uniform block and binds it.
static void bindQuartzBlock(const GlassQuartz& quartz) {
    auto& programs = g_pGlobalState->meniscusPrograms;
    glBindBuffer(GL_UNIFORM_BUFFER, programs.quartzBlock);
    glBufferData(GL_UNIFORM_BUFFER, sizeof(quartz.block), quartz.block, GL_DYNAMIC_DRAW);
    glBindBufferBase(GL_UNIFORM_BUFFER, QUARTZ_BLOCK_BINDING, programs.quartzBlock);
}

static void unbindQuartz() {
    glBindBufferBase(GL_UNIFORM_BUFFER, QUARTZ_BLOCK_BINDING, 0);
    glBindBuffer(GL_UNIFORM_BUFFER, 0);
    for (const GLenum unit : {GL_TEXTURE5, GL_TEXTURE4, GL_TEXTURE2, GL_TEXTURE1}) {
        glActiveTexture(unit);
        glBindTexture(GL_TEXTURE_2D, 0);
    }
    glActiveTexture(GL_TEXTURE0);
}

// A window's pane: the window's own rounded rectangle, the same shader. The quad
// reaches beyond the pane for the dark rim a point outside it; a solid window
// gets the band of its frame and that rim, and not its middle, which it hides.
// The corners stay: the rounding takes them off the window, and the band
// curves in there.
void drawQuartzWindow(const SP<Render::IFramebuffer>& target, const SQuartzWindow& pane) {
    auto&      programs = g_pGlobalState->meniscusPrograms;
    const auto monitor  = g_pHyprRenderer->m_renderData.pMonitor.lock();
    programs.initializeIfNeeded();
    if (!target || !pane.page || !monitor || !programs.ready() || !programs.quartzBlock)
        return;

    const double rim       = std::ceil(2.0 * monitor->m_scale) + 1.0;
    const CBox   rawQuad   = pane.rawBox.copy().expand(rim);
    const CBox   quadBox   = pane.transformBox.copy().expand(rim);
    const auto   transform = Math::wlTransformToHyprutils(Math::invertTransform(monitor->m_transform));

    CRegion region{rawQuad};
    if (pane.band >= 0.0) {
        // The middle the window hides is a cross: inside the band, and clear
        // of the corners. Cut as a box, it took the curve of the band with it.
        const double band   = std::max(1.0, static_cast<double>(pane.band));
        const double corner = std::max(band, static_cast<double>(pane.cornerRadius));
        const CBox&  box    = pane.rawBox;
        for (const CBox& middle : {CBox{box.x + corner, box.y + band, box.w - 2.0 * corner, box.h - 2.0 * band},
                                   CBox{box.x + band, box.y + corner, box.w - 2.0 * band, box.h - 2.0 * corner}}) {
            if (middle.w > 0.0 && middle.h > 0.0)
                region.subtract(middle);
        }
    }
    region.intersect(g_pHyprRenderer->m_renderData.damage);
    if (region.empty())
        return;

    auto& program = programs.quartz;
    auto  shader  = g_pHyprOpenGL->useShader(program.shader);
    shader->setUniformInt(SHADER_TEX, 0);
    glBindVertexArray(shader->getUniformLocation(SHADER_SHADER_VAO));
    glBindFramebuffer(GL_FRAMEBUFFER, fbId(target));
    g_pHyprRenderer->blend(true);
    bindQuartzBlock(pane.quartz);

    Mat3x3 matrix = g_pHyprRenderer->projectBoxToTarget(rawQuad, transform);
    matrix.transpose();
    shader->setUniformMatrix3fv(SHADER_PROJ, 1, GL_FALSE, matrix.getMatrix());

    const float halfW = static_cast<float>(0.5 * pane.transformBox.w);
    const float halfH = static_cast<float>(0.5 * pane.transformBox.h);
    bindTexture(GL_TEXTURE0, pane.page->getTexture());
    bindBlurred(program, pane.blurred);
    glUniform1i(program.uniform("edgeless"), 0);
    glUniform1i(program.uniform("analytic"), 1);
    glUniform2f(program.uniform("quadOrigin"), static_cast<float>(-rim), static_cast<float>(-rim));
    glUniform2f(program.uniform("quadSize"), static_cast<float>(quadBox.w), static_cast<float>(quadBox.h));
    glUniform2f(program.uniform("rectCenter"), halfW, halfH);
    glUniform2f(program.uniform("rectHalf"), halfW, halfH);
    glUniform1f(program.uniform("rectRadius"), pane.cornerRadius);
    glUniform1f(program.uniform("rectPower"), pane.roundingPower);
    glUniform2f(program.uniform("pageOrigin"), static_cast<float>(pane.pageRect.x - pane.transformBox.x), static_cast<float>(pane.pageRect.y - pane.transformBox.y));
    glUniform2f(program.uniform("pageSize"), static_cast<float>(pane.pageRect.w), static_cast<float>(pane.pageRect.h));
    glUniform1f(program.uniform("pointScale"), static_cast<float>(monitor->m_scale));
    glUniform1f(program.uniform("opacity"), pane.alpha);
    glUniform1f(program.uniform("lightGain"), pane.lightGain);

    Diagnostics::recordGlassPixels(monitor->m_id, rawQuad.w * rawQuad.h);
    for (const auto& part : quadsOf(region)) {
        g_pHyprOpenGL->scissor(part);
        glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
    }
    g_pHyprOpenGL->scissor(nullptr);
    unbindQuartz();
}

// The monitor pixels Apple's shader reads for a pane, whole pixels: the pane and,
// around it, the furthest any of its stages displaces or blurs, in points as
// every length of the shader is.
CBox CLayer::quartzRect(const SPane& pane, const Vector2D& boxPosition, const Vector2D& sourceSize) const {
    const auto   monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
    const double scale   = monitor && monitor->m_scale > 0.0f ? monitor->m_scale : 1.0;
    const double margin = std::ceil(m_quartz.reach * scale) + 2.0;
    const CBox   wanted = pane.box.copy().translate(boxPosition).expand(margin).intersection(CBox{{}, sourceSize});
    const double x      = std::floor(wanted.x);
    const double y      = std::floor(wanted.y);
    return CBox{x, y, std::ceil(wanted.x + wanted.w) - x, std::ceil(wanted.y + wanted.h) - y};
}

// The page around the pane, as far as the shader reads it, into a texture with a
// mip chain: the transcription blurs by reading a level of it (blurLOD), as
// macOS 27 does, instead of running a Gaussian.
void CLayer::sampleQuartz(SPane& pane, const SP<Render::IFramebuffer>& source, const Vector2D& boxPosition, const SP<Render::IFramebuffer>& caller,
                          const SCleanSource* clean) {
    Diagnostics::CScopedStageTimer stageTimer(Diagnostics::EStage::ModelSample);

    // The backdrop is read where it is (Backdrop.hpp): every pane on the monitor
    // shares it, and none copies it.
    const bool fromBackdrop = Backdrop::isBackdrop(source);
    if (fromBackdrop || Backdrop::isBackdrop(pane.page))
        pane.page.reset();
    pane.blurred.reset();
    if (fromBackdrop) {
        const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
        const float scale  = monitor && monitor->m_scale > 0.0f ? monitor->m_scale : 1.0f;
        pane.page     = source;
        pane.pageRect = CBox{{}, source->m_size};
        pane.blurred  = Backdrop::blurred(monitor, caller, m_quartz.blur * scale);
        pane.sampled  = pane.pageRect;
        pane.frost.reset();
        pane.sharp.reset();
        pane.layered     = false;
        pane.faceApplied = false;
        pane.reduction   = 1;
        pane.hasSample   = true;
        pane.dirty       = false;
        return;
    }

    const CBox rect = quartzRect(pane, boxPosition, source->m_size);
    const int  sx   = static_cast<int>(rect.x);
    const int  sy   = static_cast<int>(rect.y);
    const int  sw   = static_cast<int>(rect.w);
    const int  sh   = static_cast<int>(rect.h);
    if (sw < 1 || sh < 1) {
        pane.hasSample = false;
        return;
    }

    if (const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock())
        Diagnostics::recordSampledPixels(monitor->m_id, static_cast<double>(sw) * static_cast<double>(sh));

    // Windows under the pane: the clean page, and a blur of its own for the face.
    if (clean) {
        if (!copyClean(pane.page, *clean, rect, caller)) {
            pane.hasSample = false;
            return;
        }
        const auto  monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
        const float scale   = monitor && monitor->m_scale > 0.0f ? monitor->m_scale : 1.0f;
        pane.blurred        = Backdrop::blurInto(pane.page, pane.ownBlur, m_quartz.blur * scale, caller) ? pane.ownBlur : nullptr;
    } else if (!copyWithMips(pane.page, source, rect, caller)) {
        pane.hasSample = false;
        return;
    }

    pane.frost.reset();
    pane.sharp.reset();
    pane.layered     = false;
    pane.faceApplied = false;
    pane.pageRect    = rect;
    pane.sampled     = rect;
    pane.reduction   = 1;
    pane.hasSample   = true;
    pane.dirty       = false;
}

void CLayer::drawQuartz(const SP<Render::IFramebuffer>& target, const CBox& layerBox, const SMaskSource& mask, float opacity) {
    const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
    auto&      programs = g_pGlobalState->meniscusPrograms;
    if (!monitor || !programs.quartzBlock)
        return;

    const auto  transform = Math::wlTransformToHyprutils(Math::invertTransform(monitor->m_transform));
    const CBox  bounds    = CBox{layerBox.x, layerBox.y, layerBox.w, layerBox.h}.intersection(CBox{{}, target->m_size});
    const auto& damage    = g_pHyprRenderer->m_renderData.damage;

    auto& program = programs.quartz;
    auto  shader  = g_pHyprOpenGL->useShader(program.shader);
    shader->setUniformInt(SHADER_TEX, 0);
    glBindVertexArray(shader->getUniformLocation(SHADER_SHADER_VAO));

    bindQuartzBlock(m_quartz);
    glUniform1i(program.uniform("analytic"), 0);
    glUniform1f(program.uniform("lightGain"), 1.0f);

    if (!m_seeds || !m_field)
        return;
    bindTexture(GL_TEXTURE1, m_seeds->getTexture());
    glUniform1i(program.uniform("seedTex"), 1);
    bindTexture(GL_TEXTURE4, m_field->getTexture());
    glUniform1i(program.uniform("fieldTex"), 4);
    glUniform2f(program.uniform("fieldMargin"), static_cast<float>(m_margin), static_cast<float>(m_margin));
    glUniform2f(program.uniform("invFieldSize"), 1.0f / (static_cast<float>(m_fieldTexels.x) * m_fieldScale), 1.0f / (static_cast<float>(m_fieldTexels.y) * m_fieldScale));
    glUniform2f(program.uniform("seedMargin"), static_cast<float>(m_margin), static_cast<float>(m_margin));
    glUniform2i(program.uniform("seedHigh"), static_cast<int>(m_seedTexels.x) - 1, static_cast<int>(m_seedTexels.y) - 1);
    glUniform1f(program.uniform("pointScale"), static_cast<float>(monitor->m_scale));
    bindMask(GL_TEXTURE2, mask.texture);
    glUniform1i(program.uniform("maskTex"), 2);
    glUniform2f(program.uniform("boxSize"), static_cast<float>(layerBox.w), static_cast<float>(layerBox.h));
    glUniform2f(program.uniform("maskUvScale"), static_cast<float>(mask.uvScale.x), static_cast<float>(mask.uvScale.y));
    glUniform1f(program.uniform("maskThreshold"), mask.threshold);
    glUniform1f(program.uniform("opacity"), opacity);

    for (const auto& pane : m_panes) {
        if (!pane.hasSample || !pane.page)
            continue;

        // beyond the outline: its antialiased edge, and the dark rim a point outside it
        CRegion region{pane.box.copy().expand(std::ceil(2.0 * monitor->m_scale) + 1.0).translate(layerBox.pos())};
        region.intersect(bounds).intersect(damage);
        if (region.empty())
            continue;

        bindTexture(GL_TEXTURE0, pane.page->getTexture());
        bindBlurred(program, pane.blurred);
        // a pane over the whole monitor (a menu's backdrop) has no edge: the
        // screen's border is not the glass's
        const CBox onMonitor = pane.box.copy().translate(layerBox.pos());
        const bool edgeless  = onMonitor.x <= 1.0 && onMonitor.y <= 1.0 && onMonitor.x + onMonitor.w >= target->m_size.x - 1.0 && onMonitor.y + onMonitor.h >= target->m_size.y - 1.0;
        glUniform1i(program.uniform("edgeless"), edgeless ? 1 : 0);
        glUniform2f(program.uniform("pageOrigin"), static_cast<float>(pane.pageRect.x - layerBox.x), static_cast<float>(pane.pageRect.y - layerBox.y));
        glUniform2f(program.uniform("pageSize"), static_cast<float>(pane.pageRect.w), static_cast<float>(pane.pageRect.h));

        for (const auto& quad : quadsOf(region)) {
            Mat3x3 matrix = g_pHyprRenderer->projectBoxToTarget(quad, transform);
            matrix.transpose();
            shader->setUniformMatrix3fv(SHADER_PROJ, 1, GL_FALSE, matrix.getMatrix());
            glUniform2f(program.uniform("quadOrigin"), static_cast<float>(quad.x - layerBox.x), static_cast<float>(quad.y - layerBox.y));
            glUniform2f(program.uniform("quadSize"), static_cast<float>(quad.w), static_cast<float>(quad.h));
            Diagnostics::recordGlassPixels(monitor->m_id, quad.w * quad.h);
            glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
        }
    }

    unbindQuartz();
}

} // namespace Meniscus
