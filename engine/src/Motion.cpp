#include "Motion.hpp"
#include "GlassDecoration.hpp"
#include "Globals.hpp"
#include "Meniscus.hpp"
#include "PluginConfig.hpp"
#include "RenderGuards.hpp"
#include "Tuning.hpp"
#include "WindowGeometry.hpp"

#include <algorithm>
#include <chrono>
#include <format>
#include <cmath>
#include <GLES3/gl32.h>
#include <memory>
#include <optional>
#include <string>
#include <unordered_map>
#include <vector>

#include <hyprland/src/animation/AnimationManager.hpp>
#include <hyprland/src/config/shared/animation/AnimationTree.hpp>
#include <hyprland/src/desktop/Workspace.hpp>
#include <hyprland/src/desktop/state/FadingOutState.hpp>
#include <hyprland/src/desktop/state/FocusState.hpp>
#include <hyprland/src/desktop/state/ViewState.hpp>
#include <hyprland/src/desktop/state/WindowFadeout.hpp>
#include <hyprland/src/desktop/view/Window.hpp>
#include <hyprland/src/layout/LayoutManager.hpp>
#include <hyprland/src/layout/supplementary/DragController.hpp>
#include <hyprland/src/managers/input/InputManager.hpp>
#include <hyprland/src/render/OpenGL.hpp>
#include <hyprland/src/render/Renderer.hpp>
#include <hyprland/src/render/pass/SurfacePassElement.hpp>
#include <hyprland/src/render/pass/TexPassElement.hpp>
#include <hyprland/src/render/transformer/Transformer.hpp>

namespace {
    using Clock = std::chrono::steady_clock;

    // Cells of a wobbling window's mesh each way, and of the genie's, which
    // bends a window much further and along either axis.
    constexpr uint32_t WOBBLE_CELLS = 12;
    constexpr uint32_t GENIE_CELLS  = 24;
    // The most one frame moves the springs on: a stall is no reason for a
    // window to jump (the model moves its sheet on a fifth of a second at most).
    constexpr double MAX_STEP = 0.2;
    // How wide the genie's mouth is, in logical pixels.
    constexpr double MOUTH = 40.0;
    // A window that closed waits this long for Hyprland's snapshot of it.
    constexpr auto MATCH_WITHIN = std::chrono::milliseconds(250);
    // Framebuffers no animation has used for this long are freed.
    constexpr auto KEEP_BUFFERS = std::chrono::seconds(5);
    // The glass is lifted with this much of the frame round it, in logical pixels.
    constexpr double LIFT_MARGIN = 4.0;
    // The most pieces of a frame's damage copied one by one into the kept
    // window; more are copied as the one box round them all.
    constexpr size_t MAX_PARTS = 8;
    // The names of this library's pass elements, which have to be out of
    // Hyprland's pass before the library is unloaded.
    constexpr const char* GENIE_PASS = "CGlassGenieElement";
    constexpr const char* KEEP_PASS  = "CGlassKeepElement";
    // How the sheet's points hold to each other, and how far they may trail:
    // the model's jelly (glass_core::motion::wobble::WobbleSpec::JELLY).
    constexpr double COUPLING = 2.0;
    constexpr double REACH    = 0.3;

    enum class EOpen : uint8_t {
        None,
        Genie,
        Jelly,
    };

    // A window's glass copied off the frame it was drawn on: the copy, the part
    // of the frame it holds, and the window's pane in it with its corners.
    struct SGlass {
        SP<Render::IFramebuffer> copy;
        CBox                     box;  // the part of the frame, monitor pixels
        CBox                     pane; // the pane, in the copy's pixels
        float                    radius = 0.f;
        float                    power  = 2.f;
        bool                     whole  = false; // the window as it looked, not only its glass
    };

    struct SWindow {
        PHLWINDOWREF      window;
        GlassWobble       sheet{};
        CBox              lastBox; // the window last frame, in global pixels
        bool              seen     = false;
        bool              held     = false;
        bool              moving   = false; // the sheet has not come to rest
        bool              attached = false;
        EOpen             open     = EOpen::None;
        Clock::time_point openedAt;
        double            progress = 0.0; // of the genie: 1 is all of it in the mouth
        GlassMouth        mouth{};        // in monitor pixels
        Clock::time_point stepped;

        // The window's glass this frame, lifted off the frame (CLift), and the
        // frame as it was before the glass, put back.
        SGlass                   lift;
        SP<Render::IFramebuffer> save;
        uint64_t                 liftedIn = 0; // the frame the lift was made in

        uint64_t transformedIn = 0; // the frame of the last transform()
        int      transforms    = 0; // in it: the second is the blur's matte
    };

    struct SClosing {
        MONITORID                monitor = -1;
        CBox                     source; // the window, monitor pixels
        GlassMouth               mouth{};
        Clock::time_point        closedAt;
        bool                     matched = false;
        SP<Render::IFramebuffer> snapshot;
        Render::ITexture*        texture = nullptr; // compared, never used
        float                    alpha   = 1.f;
        Clock::time_point        startedAt;
        SGlass                   glass; // drawn under the snapshot, which has none
    };

    struct SBuffers {
        std::array<SP<Render::IFramebuffer>, 2> out;
        Clock::time_point                      used;
    };

    // How often each part has run since the plugin loaded, for describe().
    struct SCounters {
        uint64_t opens = 0, closes = 0, fadeouts = 0, matched = 0, genies = 0, genieDraws = 0, transforms = 0, meshes = 0, lifts = 0, keeps = 0, dragged = 0;
    };

    struct SState {
        SCounters   counters;
        std::string lastGenie; // what the last genie drew, for describe()
        std::vector<std::unique_ptr<SWindow>>   windows;
        std::vector<SClosing>                   closing;
        std::vector<WP<Desktop::IFadeout>>      known; // every fadeout seen, matched or not
        std::unordered_map<MONITORID, SBuffers> buffers;
        std::unordered_map<MONITORID, bool>     busy; // monitors drawn with motion this frame
        // The window with the focus as it looked, glass and all, copied each
        // time it is drawn whole, for the genie: Hyprland's snapshot of a
        // closing window has no glass in it, since the glass is drawn only on
        // the frame, and some clients (foot) drop their picture before closing.
        SGlass       kept;
        PHLWINDOWREF keptFor;
        // Hyprland's animation speeds and springs as the config set them, before the pace.
        std::unordered_map<std::string, float>                                paces;
        std::unordered_map<std::string, Hyprutils::Animation::SSpringCurve> springs;

        Meniscus::CProgram    program;
        bool                  programTried = false;
        bool                  programReady = false;
        GLuint                vao = 0, vbo = 0, ibo = 0;
        uint32_t              indexedCols = 0, indexedRows = 0;
        GLsizei               indexCount  = 0;
        std::vector<float>    points;
        std::vector<float>    vertices;
        std::vector<uint16_t> indices;
    };

    std::unique_ptr<SState> s_state;

    float knob(uint32_t index) {
        return g_pGlobalState->tuning.values[index];
    }

    bool on(uint32_t index) {
        return knob(index) >= 0.5f;
    }

    bool enabled() {
        if (!s_state || !g_pGlobalState)
            return false;
        const auto& config = g_pGlobalState->config;
        return config.motionEnabled && **config.motionEnabled;
    }

    double pace() {
        return enabled() ? std::clamp(static_cast<double>(knob(GLASS_T_PACE)), 0.5, 2.0) : 1.0;
    }

    double genieSeconds() {
        return std::max(0.05, static_cast<double>(knob(GLASS_T_GENIE_TIME)) / pace());
    }

    GlassWobbleSpec wobbleSpec() {
        return {static_cast<double>(knob(GLASS_T_WOBBLE_TIME)) / pace(), static_cast<double>(knob(GLASS_T_WOBBLE_BOUNCE)), COUPLING, REACH};
    }

    double seconds(Clock::duration span) {
        return std::chrono::duration<double>(span).count();
    }

    double smoothstep(double from, double to, double at) {
        const double t = std::clamp((at - from) / (to - from), 0.0, 1.0);
        return t * t * (3.0 - 2.0 * t);
    }

    GlassRect rectOf(const CBox& box) {
        return {box.x, box.y, box.w, box.h};
    }

    GLuint fbId(const SP<Render::IFramebuffer>& framebuffer) {
        auto* gl = dynamic_cast<Render::GL::CGLFramebuffer*>(framebuffer.get());
        return gl ? gl->getFBID() : 0;
    }

    bool normal(const PHLMONITOR& monitor) {
        return monitor && monitor->m_transform == WL_OUTPUT_TRANSFORM_NORMAL && monitor->m_scale > 0.f;
    }

    SWindow* find(const PHLWINDOW& window) {
        if (!s_state || !window)
            return nullptr;
        for (const auto& state : s_state->windows) {
            if (state->window.lock() == window)
                return state.get();
        }
        return nullptr;
    }

    SWindow& track(const PHLWINDOW& window) {
        if (auto* state = find(window))
            return *state;
        auto state         = std::make_unique<SWindow>();
        state->window      = window;
        state->sheet.held  = -1;
        state->stepped     = Clock::now();
        s_state->windows.emplace_back(std::move(state));
        return *s_state->windows.back();
    }

    // Where the genie pours a window of this monitor: the middle of its bottom
    // edge, or the pointer, in the monitor's pixels.
    GlassMouth mouthFor(const PHLMONITOR& monitor) {
        const double scale = monitor->m_scale;
        if (on(GLASS_T_GENIE_POINTER) && g_pInputManager) {
            const Vector2D pointer = (g_pInputManager->getMouseCoordsInternal() - monitor->m_position) * scale;
            return {std::clamp(pointer.x, 0.0, monitor->m_pixelSize.x), std::clamp(pointer.y, 0.0, monitor->m_pixelSize.y), MOUTH * scale};
        }
        return {monitor->m_pixelSize.x / 2.0, monitor->m_pixelSize.y, MOUTH * scale};
    }

    // ── GL ───────────────────────────────────────────────────────────────────

    bool ready() {
        if (s_state->programReady)
            return true;
        if (s_state->programTried)
            return false;
        s_state->programTried = true;
        if (!s_state->program.create("mesh.frag"))
            return false;

        const auto& shader = s_state->program.shader;
        glGenVertexArrays(1, &s_state->vao);
        glGenBuffers(1, &s_state->vbo);
        glGenBuffers(1, &s_state->ibo);
        glBindVertexArray(s_state->vao);
        glBindBuffer(GL_ARRAY_BUFFER, s_state->vbo);
        const GLint pos      = shader->getUniformLocation(SHADER_POS_ATTRIB);
        const GLint texcoord = shader->getUniformLocation(SHADER_TEX_ATTRIB);
        if (pos >= 0) {
            glEnableVertexAttribArray(pos);
            glVertexAttribPointer(pos, 2, GL_FLOAT, GL_FALSE, 4 * sizeof(float), nullptr);
        }
        if (texcoord >= 0) {
            glEnableVertexAttribArray(texcoord);
            glVertexAttribPointer(texcoord, 2, GL_FLOAT, GL_FALSE, 4 * sizeof(float), reinterpret_cast<void*>(2 * sizeof(float)));
        }
        glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, s_state->ibo);
        glBindVertexArray(0);
        glBindBuffer(GL_ARRAY_BUFFER, 0);
        s_state->programReady = pos >= 0 && texcoord >= 0;
        return s_state->programReady;
    }

    // Two triangles a cell of a mesh of cols by rows cells, into the index
    // buffer, when the mesh's size changed. Needs the vertex array bound.
    void index(uint32_t cols, uint32_t rows) {
        if (s_state->indexedCols == cols && s_state->indexedRows == rows)
            return;
        auto& indices = s_state->indices;
        indices.clear();
        const uint32_t stride = cols + 1;
        for (uint32_t j = 0; j < rows; ++j) {
            for (uint32_t i = 0; i < cols; ++i) {
                const auto at = static_cast<uint16_t>(j * stride + i);
                const auto below = static_cast<uint16_t>(at + stride);
                indices.insert(indices.end(), {at, static_cast<uint16_t>(at + 1), below, static_cast<uint16_t>(at + 1), static_cast<uint16_t>(below + 1), below});
            }
        }
        glBufferData(GL_ELEMENT_ARRAY_BUFFER, static_cast<GLsizeiptr>(indices.size() * sizeof(uint16_t)), indices.data(), GL_STATIC_DRAW);
        s_state->indexedCols = cols;
        s_state->indexedRows = rows;
        s_state->indexCount  = static_cast<GLsizei>(indices.size());
    }

    // The vertices of the mesh in points: each point, and where it shows the
    // texture: the place in `grid` (the box the mesh was made over) it stands
    // for, as the texture holds the frame of `texBox`.
    void fillVertices(uint32_t cols, uint32_t rows, const CBox& grid, const CBox& texBox) {
        const auto& points   = s_state->points;
        auto&       vertices = s_state->vertices;
        vertices.resize(points.size() * 2);
        for (uint32_t j = 0; j <= rows; ++j) {
            for (uint32_t i = 0; i <= cols; ++i) {
                const size_t at = static_cast<size_t>(j) * (cols + 1) + i;
                const double u  = grid.x + grid.w * i / cols;
                const double v  = grid.y + grid.h * j / rows;
                vertices[at * 4 + 0] = points[at * 2 + 0];
                vertices[at * 4 + 1] = points[at * 2 + 1];
                vertices[at * 4 + 2] = static_cast<float>((u - texBox.x) / texBox.w);
                vertices[at * 4 + 3] = static_cast<float>((v - texBox.y) / texBox.h);
            }
        }
    }

    // Binds target for the mesh, the whole of it, and the program over it.
    void beginDraw(const SP<Render::IFramebuffer>& target, std::optional<float> clearAlpha) {
        g_pHyprOpenGL->setCapStatus(GL_SCISSOR_TEST, false);
        if (glIsEnabled(GL_SCISSOR_TEST))
            glDisable(GL_SCISSOR_TEST);
        if (glIsEnabled(GL_STENCIL_TEST))
            glDisable(GL_STENCIL_TEST);
        const auto width  = static_cast<float>(target->m_size.x);
        const auto height = static_cast<float>(target->m_size.y);
        glBindFramebuffer(GL_FRAMEBUFFER, fbId(target));
        g_pHyprOpenGL->setViewport(0, 0, static_cast<int>(width), static_cast<int>(height));
        if (clearAlpha) {
            glClearColor(0.f, 0.f, 0.f, *clearAlpha);
            glClear(GL_COLOR_BUFFER_BIT);
        }
        g_pHyprRenderer->blend(true);
        glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA);
        // pixels of the frame, top left at 0, to clip space: the frame's rows run
        // as the monitor's do in every framebuffer Hyprland draws
        auto shader = g_pHyprOpenGL->useShader(s_state->program.shader);
        shader->setUniformMatrix3fv(SHADER_PROJ, 1, GL_FALSE, {2.f / width, 0.f, 0.f, 0.f, 2.f / height, 0.f, -1.f, -1.f, 1.f});
        shader->setUniformInt(SHADER_TEX, 0);
        glActiveTexture(GL_TEXTURE0);
    }

    struct SPane {
        CBox     box; // in the texture's pixels
        Vector2D size;
        float    radius  = 0.f;
        float    power   = 2.f;
        bool     outside = false; // all of the texture but the pane
    };

    SPane paneOf(const SGlass& glass) {
        return {glass.pane, glass.box.size(), glass.radius, glass.power};
    }

    // Draws the vertices made by fillVertices with `texture` over them.
    void drawMesh(const SP<Render::ITexture>& texture, uint32_t cols, uint32_t rows, float alpha, const SPane* pane) {
        auto& program = s_state->program;
        texture->bind();
        texture->setTexParameter(GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        texture->setTexParameter(GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        glUniform1f(program.uniform("alpha"), std::clamp(alpha, 0.f, 1.f));
        glUniform1i(program.uniform("pane"), pane ? (pane->outside ? 2 : 1) : 0);
        if (pane) {
            glUniform2f(program.uniform("texSize"), static_cast<float>(pane->size.x), static_cast<float>(pane->size.y));
            glUniform4f(program.uniform("shape"), static_cast<float>(pane->box.x), static_cast<float>(pane->box.y), static_cast<float>(pane->box.w),
                        static_cast<float>(pane->box.h));
            glUniform1f(program.uniform("radius"), pane->radius);
            glUniform1f(program.uniform("power"), pane->power);
        }
        glBindVertexArray(s_state->vao);
        index(cols, rows);
        glBindBuffer(GL_ARRAY_BUFFER, s_state->vbo);
        const auto& vertices = s_state->vertices;
        glBufferData(GL_ARRAY_BUFFER, static_cast<GLsizeiptr>(vertices.size() * sizeof(float)), vertices.data(), GL_STREAM_DRAW);
        glDrawElements(GL_TRIANGLES, s_state->indexCount, GL_UNSIGNED_SHORT, nullptr);
        glBindVertexArray(0);
        glBindBuffer(GL_ARRAY_BUFFER, 0);
    }

    // Hands the GL state back as Hyprland keeps it between elements.
    void endDraw(const SP<Render::IFramebuffer>& caller) {
        glActiveTexture(GL_TEXTURE0);
        if (!caller)
            return;
        glBindFramebuffer(GL_FRAMEBUFFER, fbId(caller));
        g_pHyprOpenGL->setViewport(0, 0, static_cast<int>(caller->m_size.x), static_cast<int>(caller->m_size.y));
    }

    void ensure(SP<Render::IFramebuffer>& framebuffer, const char* name, int width, int height, DRMFormat format) {
        if (!framebuffer)
            framebuffer = g_pHyprRenderer->createFB(name);
        if (static_cast<int>(framebuffer->m_size.x) != width || static_cast<int>(framebuffer->m_size.y) != height || framebuffer->m_drmFormat != format)
            framebuffer->alloc(width, height, format);
    }

    void blit(const SP<Render::IFramebuffer>& from, const CBox& fromBox, const SP<Render::IFramebuffer>& to, const CBox& toBox) {
        glBindFramebuffer(GL_READ_FRAMEBUFFER, fbId(from));
        glBindFramebuffer(GL_DRAW_FRAMEBUFFER, fbId(to));
        glBlitFramebuffer(static_cast<int>(fromBox.x), static_cast<int>(fromBox.y), static_cast<int>(fromBox.x + fromBox.w), static_cast<int>(fromBox.y + fromBox.h),
                          static_cast<int>(toBox.x), static_cast<int>(toBox.y), static_cast<int>(toBox.x + toBox.w), static_cast<int>(toBox.y + toBox.h), GL_COLOR_BUFFER_BIT,
                          GL_NEAREST);
    }

    // ── The glass ────────────────────────────────────────────────────────────

    // The pane of a window's glass with a little of the frame round it, inside
    // the frame, and the pane in that part.
    std::optional<std::pair<CBox, CBox>> liftShape(const PHLWINDOW& window, const PHLMONITOR& monitor, const SP<Render::IFramebuffer>& target) {
        const auto pane = WindowGeometry::computeWindowBox(window, monitor);
        if (!pane)
            return std::nullopt;
        const CBox lift = pane->copy().expand(LIFT_MARGIN * monitor->m_scale).round();
        const auto x0 = std::max(0.0, lift.x), y0 = std::max(0.0, lift.y);
        const auto x1 = std::min(target->m_size.x, lift.x + lift.w), y1 = std::min(target->m_size.y, lift.y + lift.h);
        if (x1 - x0 < 1.0 || y1 - y0 < 1.0)
            return std::nullopt;
        const CBox box{x0, y0, x1 - x0, y1 - y0};
        return std::pair{box, pane->copy().translate(-box.pos())};
    }

    // Copies the part of the frame the glass holds into its copy, and takes the
    // corners of the window's pane, rounded as the glass rounds them.
    void copyGlass(SGlass& glass, const PHLWINDOW& window, const PHLMONITOR& monitor, const SP<Render::IFramebuffer>& target, const char* name) {
        ensure(glass.copy, name, static_cast<int>(glass.box.w), static_cast<int>(glass.box.h), target->m_drmFormat);
        g_pHyprOpenGL->setCapStatus(GL_SCISSOR_TEST, false);
        if (glIsEnabled(GL_SCISSOR_TEST))
            glDisable(GL_SCISSOR_TEST);
        blit(target, glass.box, glass.copy, {0, 0, glass.box.w, glass.box.h});
        const float scale   = monitor->m_scale;
        const float rounded = window->rounding() * scale;
        glass.radius        = rounded > 0.f ? rounded + WindowGeometry::glassBorder(window) * scale : 0.f;
        glass.power         = window->roundingPower();
    }

    // Whether a window is kept for the genie: the window with the focus, which
    // is nearly always the one closed, when it has glass. Its pane is opaque
    // then, so the copy holds the window and nothing behind it.
    bool keeps(const PHLWINDOW& window) {
        if (!enabled() || !on(GLASS_T_GENIE_CLOSE) || !g_pGlobalState->renderPassAddHook || window != Desktop::focusState()->window())
            return false;
        const auto* decoration = glassDecorationFor(window);
        return decoration && decoration->isGlassEnabled();
    }

    // ── The mesh ─────────────────────────────────────────────────────────────

    // The window as its transformer draws it: the whole of what Hyprland draws
    // of it, decorations and all, in the monitor's pixels.
    std::optional<CBox> drawnBox(const PHLWINDOW& window, const PHLMONITOR& monitor) {
        CBox       box       = window->getFullWindowBoundingBox();
        const auto workspace = window->m_workspace;
        box.translate((window->m_pinned || !workspace ? Vector2D{} : workspace->m_renderOffset->value()) + window->m_floatingOffset - monitor->m_position);
        box.scale(monitor->m_scale);
        if (!std::isfinite(box.x) || !std::isfinite(box.y) || box.w < 1.0 || box.h < 1.0)
            return std::nullopt;
        return box;
    }

    // The window's mesh over `box` this frame into the points; the cells each
    // way, or 0 when there is none.
    uint32_t windowMesh(const SWindow& state, const CBox& box) {
        const bool     genie = state.open == EOpen::Genie;
        const uint32_t cells = genie ? GENIE_CELLS : WOBBLE_CELLS;
        auto&          points = s_state->points;
        points.resize(static_cast<size_t>(cells + 1) * (cells + 1) * 2);
        GlassMesh       mesh{points.data(), points.size(), cells, cells};
        const GlassRect rect = rectOf(box);
        const int32_t   made = genie ? glass_genie_mesh(&rect, &state.mouth, state.progress, &mesh) : glass_wobble_mesh(&state.sheet, &rect, &mesh);
        return made > 0 ? cells : 0;
    }

    SP<Render::IFramebuffer>& outBuffer(const PHLMONITOR& monitor, bool matte, const SP<Render::IFramebuffer>& like) {
        auto& buffers = s_state->buffers[monitor->m_id];
        buffers.used  = Clock::now();
        auto& out     = buffers.out[matte ? 1 : 0];
        ensure(out, matte ? "hyprglass-motion-matte" : "hyprglass-motion", static_cast<int>(like->m_size.x), static_cast<int>(like->m_size.y), like->m_drmFormat);
        return out;
    }

    // ── The transformer ──────────────────────────────────────────────────────

    // Draws a window Hyprland drew into a framebuffer of its own over the
    // window's mesh: its glass first, lifted off the frame by CLift, then the
    // window. Hyprland draws the result over the monitor.
    class CMotionTransformer final : public Render::IWindowTransformer {
      public:
        explicit CMotionTransformer(PHLWINDOWREF window) : m_window(std::move(window)) {}

        SP<Render::IFramebuffer> transform(SP<Render::IFramebuffer> in) override {
            if (!s_state || !in || !in->getTexture() || g_pHyprRenderer->m_bRenderingSnapshot)
                return in;
            const auto window  = m_window.lock();
            auto*      state   = find(window);
            const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
            if (!state || !normal(monitor) || !ready())
                return in;

            // Hyprland asks twice for a window with its blur on: the window, then the blur's matte
            const uint64_t frame = g_pGlobalState->frameSerial;
            if (state->transformedIn != frame) {
                state->transformedIn = frame;
                state->transforms    = 0;
            }
            const bool matte = state->transforms++ > 0;
            ++s_state->counters.transforms;

            const auto box   = drawnBox(window, monitor);
            const auto cells = box ? windowMesh(*state, *box) : 0;
            if (!cells)
                return in;
            ++s_state->counters.meshes;

            auto& out = outBuffer(monitor, matte, in);
            beginDraw(out, matte ? 1.f : 0.f);
            // the lift is the glass as it was drawn over the frame, alpha and all
            if (!matte && state->liftedIn == frame && state->lift.copy && state->lift.copy->getTexture()) {
                fillVertices(cells, cells, *box, state->lift.box);
                const SPane pane = paneOf(state->lift);
                drawMesh(state->lift.copy->getTexture(), cells, cells, 1.f, &pane);
            }
            fillVertices(cells, cells, *box, CBox{0, 0, in->m_size.x, in->m_size.y});
            drawMesh(in->getTexture(), cells, cells, 1.f, nullptr);
            endDraw(g_pHyprRenderer->m_renderData.currentFB);
            return out;
        }

      private:
        PHLWINDOWREF m_window;
    };

    void attach(SWindow& state, const PHLWINDOW& window) {
        if (state.attached || !window)
            return;
        window->m_transformers.emplace_back(makeUnique<CMotionTransformer>(state.window));
        state.attached = true;
    }

    void detach(SWindow& state) {
        if (!state.attached)
            return;
        if (const auto window = state.window.lock())
            std::erase_if(window->m_transformers, [](const auto& transformer) { return dynamic_cast<CMotionTransformer*>(transformer.get()) != nullptr; });
        state.attached = false;
        state.lift     = {};
        state.save.reset();
    }

    // ── The closing genie ────────────────────────────────────────────────────

    // A closing window's snapshot poured into the genie's mouth, drawn where
    // Hyprland would have drawn the snapshot fading out.
    class CGenieElement final : public IPassElement {
      public:
        struct SData {
            SP<Render::IFramebuffer> snapshot;
            CBox                     source;
            GlassMouth               mouth{};
            double                   progress = 0.0;
            float                    alpha    = 1.f;
            SGlass                   glass;
        };

        explicit CGenieElement(SData data) : m_data(std::move(data)) {}

        std::vector<UP<IPassElement>> draw() override {
            const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
            const auto target  = g_pHyprRenderer->m_renderData.currentFB;
            if (!s_state || !m_data.snapshot || !m_data.snapshot->getTexture() || !target || !normal(monitor) || !ready())
                return {};
            auto& points = s_state->points;
            points.resize(static_cast<size_t>(GENIE_CELLS + 1) * (GENIE_CELLS + 1) * 2);
            GlassMesh       mesh{points.data(), points.size(), GENIE_CELLS, GENIE_CELLS};
            const GlassRect rect = rectOf(m_data.source);
            if (glass_genie_mesh(&rect, &m_data.mouth, m_data.progress, &mesh) <= 0)
                return {};
            ++s_state->counters.genieDraws;
            const bool glass   = m_data.glass.copy && m_data.glass.copy->getTexture();
            s_state->lastGenie = std::format("{:.0f}x{:.0f} at {:.0f},{:.0f}, progress {:.2f}, alpha {:.2f}, {}", m_data.source.w, m_data.source.h, m_data.source.x,
                                             m_data.source.y, m_data.progress, m_data.alpha, glass ? "with its glass" : "without glass");
            beginDraw(target, std::nullopt);
            if (glass) {
                fillVertices(GENIE_CELLS, GENIE_CELLS, m_data.source, m_data.glass.box);
                const SPane pane = paneOf(m_data.glass);
                drawMesh(m_data.glass.copy->getTexture(), GENIE_CELLS, GENIE_CELLS, m_data.alpha, &pane);
            }
            // the snapshot: all of it over the glass alone, and only what lies
            // outside the pane (the border) around the whole window
            const CBox snapshot{0, 0, m_data.snapshot->m_size.x, m_data.snapshot->m_size.y};
            fillVertices(GENIE_CELLS, GENIE_CELLS, m_data.source, snapshot);
            if (glass && m_data.glass.whole) {
                const SPane around{m_data.glass.pane.copy().translate(m_data.glass.box.pos()), snapshot.size(), m_data.glass.radius, m_data.glass.power, true};
                drawMesh(m_data.snapshot->getTexture(), GENIE_CELLS, GENIE_CELLS, m_data.alpha, &around);
            } else
                drawMesh(m_data.snapshot->getTexture(), GENIE_CELLS, GENIE_CELLS, m_data.alpha, nullptr);
            endDraw(target);
            return {};
        }

        bool needsLiveBlur() override {
            return false;
        }
        bool needsPrecomputeBlur() override {
            return false;
        }
        std::optional<CBox> boundingBox() override {
            const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
            if (!monitor)
                return std::nullopt;
            return CBox{0, 0, monitor->m_size.x, monitor->m_size.y};
        }
        CRegion opaqueRegion() override {
            return {};
        }
        bool disableSimplification() override {
            return true;
        }
        const char* passName() override {
            return GENIE_PASS;
        }
        ePassElementType type() override {
            return EK_CUSTOM;
        }

      private:
        SData m_data;
    };

    // After the main surface of the window with the focus: copies the window
    // as it looks on the frame, when the frame draws all of it.
    class CKeepElement final : public IPassElement {
      public:
        explicit CKeepElement(PHLWINDOWREF window) : m_window(std::move(window)) {}

        std::vector<UP<IPassElement>> draw() override {
            const auto  window  = m_window.lock();
            const auto  monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
            const auto& target  = g_pHyprRenderer->m_renderData.currentFB;
            if (!s_state || !window || !normal(monitor) || !target || !fbId(target))
                return {};
            const auto shape = liftShape(window, monitor, target);
            if (!shape)
                return {};
            const auto& damage = g_pHyprRenderer->m_renderData.damage;
            auto&       kept   = s_state->kept;
            if (kept.copy && s_state->keptFor.lock() == window && kept.box == shape->first) {
                // the copy takes what this frame drew of the window again, so
                // it is the window as it is, at the cost of what changed
                CRegion fresh = damage.copy().intersect(shape->first);
                if (fresh.empty())
                    return {};
                g_pHyprOpenGL->setCapStatus(GL_SCISSOR_TEST, false);
                if (glIsEnabled(GL_SCISSOR_TEST))
                    glDisable(GL_SCISSOR_TEST);
                std::vector<CBox> pieces;
                if (const auto parts = fresh.getRects(); parts.size() > MAX_PARTS)
                    pieces.push_back(fresh.getExtents());
                else
                    for (const auto& part : parts)
                        pieces.emplace_back(part.x1, part.y1, part.x2 - part.x1, part.y2 - part.y1);
                for (const auto& piece : pieces)
                    blit(target, piece, kept.copy, piece.copy().translate(-kept.box.pos()));
                glBindFramebuffer(GL_FRAMEBUFFER, fbId(target));
                ++s_state->counters.keeps;
                return {};
            }
            // a new copy only of a frame that draws all of the window: one that
            // draws part of it, or draws something over it, leaves the rest as
            // it was, which may be anything
            CRegion missing{shape->first};
            missing.subtract(damage);
            if (!missing.empty())
                return {};
            kept.box   = shape->first;
            kept.pane  = shape->second;
            kept.whole = true;
            copyGlass(kept, window, monitor, target, "hyprglass-motion-kept");
            glBindFramebuffer(GL_FRAMEBUFFER, fbId(target));
            s_state->keptFor = window;
            ++s_state->counters.keeps;
            return {};
        }

        bool needsLiveBlur() override {
            return false;
        }
        bool needsPrecomputeBlur() override {
            return false;
        }
        const char* passName() override {
            return KEEP_PASS;
        }
        ePassElementType type() override {
            return EK_CUSTOM;
        }

      private:
        PHLWINDOWREF m_window;
    };

    // Takes each new fadeout of a window to the close that made it, in the
    // order the windows closed, and lets closes with no snapshot go.
    void matchFadeouts(Clock::time_point now) {
        auto& known = s_state->known;
        std::erase_if(known, [](const auto& fadeout) { return fadeout.expired(); });
        for (const auto& fadeout : Desktop::fadingOutState()->fadeouts()) {
            if (!fadeout || std::ranges::any_of(known, [&](const auto& seen) { return seen.get() == fadeout.get(); }))
                continue;
            known.emplace_back(fadeout);
            ++s_state->counters.fadeouts;
            const auto monitor = fadeout->monitor().lock();
            if (!monitor || !dynamic_cast<Desktop::CWindowFadeout*>(fadeout.get()) || !fadeout->framebuffer())
                continue;
            for (auto& closing : s_state->closing) {
                if (closing.matched || closing.monitor != monitor->m_id)
                    continue;
                closing.matched   = true;
                closing.snapshot  = fadeout->framebuffer();
                closing.texture   = closing.snapshot->getTexture().get();
                closing.startedAt = now;
                ++s_state->counters.matched;
                break;
            }
        }
        const double length = genieSeconds();
        std::erase_if(s_state->closing, [&](const SClosing& closing) {
            return closing.matched ? seconds(now - closing.startedAt) >= length : now - closing.closedAt > MATCH_WITHIN;
        });
    }

    void queueGenies(const PHLMONITOR& monitor) {
        const auto   now    = Clock::now();
        const double length = genieSeconds();
        for (const auto& closing : s_state->closing) {
            if (!closing.matched || closing.monitor != monitor->m_id)
                continue;
            const double progress = std::clamp(seconds(now - closing.startedAt) / length, 0.0, 1.0);
            const float  alpha    = closing.alpha * static_cast<float>(1.0 - smoothstep(0.85, 1.0, progress));
            ++s_state->counters.genies;
            g_pHyprRenderer->m_renderPass.add(makeUnique<CGenieElement>(CGenieElement::SData{closing.snapshot, closing.source, closing.mouth, progress, alpha, closing.glass}));
        }
    }

    // ── The windows ──────────────────────────────────────────────────────────

    PHLWINDOW draggedWindow() {
        if (!g_layoutManager)
            return nullptr;
        const auto& drag = g_layoutManager->dragController();
        if (!drag || drag->mode() != MBIND_MOVE)
            return nullptr;
        const auto target = drag->target();
        return target ? target->window() : nullptr;
    }

    bool animating(const PHLWINDOW& window) {
        return window->positionAnimation()->isBeingAnimated() || window->sizeAnimation()->isBeingAnimated();
    }

    // Moves a window's sheet on to now: the jelly's squeeze, the hold of the
    // pointer or of a layout move, the trail of a move and the springs.
    void stepSheet(SWindow& state, const PHLWINDOW& window, const CBox& box, bool dragged, double dt) {
        const auto spec = wobbleSpec();
        if (state.open == EOpen::Jelly && box.w > 0.0 && box.h > 0.0) {
            glass_wobble_squeeze(&state.sheet, box.w, box.h, knob(GLASS_T_JELLY_SQUASH));
            state.open   = EOpen::None;
            state.moving = true;
        }

        const bool layoutMove = !dragged && on(GLASS_T_WOBBLE_MOVES) && animating(window);
        if (dragged && box.w > 0.0 && box.h > 0.0) {
            const auto   monitor = window->m_monitor.lock();
            const double scale   = monitor ? monitor->m_scale : 1.0;
            const auto   pointer = g_pInputManager->getMouseCoordsInternal() * scale;
            (void)glass_wobble_hold(&state.sheet, (pointer.x - box.x) / box.w, (pointer.y - box.y) / box.h);
        } else if (layoutMove)
            (void)glass_wobble_hold(&state.sheet, 0.5, 0.5);
        else if (state.held)
            (void)glass_wobble_hold(&state.sheet, -1.0, -1.0);
        state.held = dragged || layoutMove;

        if (state.seen && box != state.lastBox && state.held) {
            const GlassRect from = rectOf(state.lastBox), to = rectOf(box);
            (void)glass_wobble_moved(&state.sheet, &spec, &from, &to);
            state.moving = true;
        }
        state.lastBox = box;
        state.seen    = true;
        if (state.moving)
            state.moving = glass_wobble_step(&state.sheet, &spec, dt) == 0 || state.held;
    }

    // Moves a window's animation on to now; whether it still runs.
    bool stepWindow(SWindow& state, const PHLWINDOW& window, const PHLMONITOR& monitor, const PHLWINDOW& dragged, Clock::time_point now) {
        const double dt = std::clamp(seconds(now - state.stepped), 0.0, MAX_STEP);
        state.stepped   = now;
        const double scale = monitor->m_scale;
        const CBox   box{window->positionAnimation()->value() * scale, window->sizeAnimation()->value() * scale};

        if (state.open == EOpen::Genie) {
            state.progress = 1.0 - std::clamp(seconds(now - state.openedAt) / genieSeconds(), 0.0, 1.0);
            if (state.progress <= 0.0)
                state.open = EOpen::None;
        }
        stepSheet(state, window, box, on(GLASS_T_WOBBLE) && dragged == window, dt);
        return state.open == EOpen::Genie || state.moving;
    }

    // Every window of the monitor that moves now and should wobble for it.
    void findMovers(const PHLMONITOR& monitor, const PHLWINDOW& dragged) {
        if (dragged && on(GLASS_T_WOBBLE) && dragged->m_monitor.lock() == monitor) {
            track(dragged);
            ++s_state->counters.dragged;
        }
        if (!on(GLASS_T_WOBBLE_MOVES))
            return;
        for (const auto& window : Desktop::viewState()->windows()) {
            if (window && window->m_isMapped && !window->isHidden() && window->m_monitor.lock() == monitor && animating(window))
                track(window);
        }
    }

    void advance(const PHLMONITOR& monitor) {
        const auto now = Clock::now();
        matchFadeouts(now);
        bool busy = std::ranges::any_of(s_state->closing, [&](const auto& closing) { return closing.monitor == monitor->m_id; });

        const auto dragged = draggedWindow();
        if (enabled() && normal(monitor))
            findMovers(monitor, dragged);
        for (auto& state : s_state->windows) {
            const auto window = state->window.lock();
            if (!window || window->m_monitor.lock() != monitor)
                continue;
            const bool runs = enabled() && normal(monitor) && window->m_isMapped && stepWindow(*state, window, monitor, dragged, now);
            busy            = busy || runs || state->attached;
            if (runs)
                attach(*state, window);
            else
                detach(*state);
        }
        // a window held still (picked up, not moved yet) stays, to trail from where it was
        std::erase_if(s_state->windows, [](const auto& state) { return state->window.expired() || (!state->attached && state->open == EOpen::None && !state->held); });
        std::erase_if(s_state->buffers, [&](const auto& entry) { return now - entry.second.used > KEEP_BUFFERS; });
        if (s_state->kept.copy && (s_state->keptFor.expired() || !enabled() || !on(GLASS_T_GENIE_CLOSE))) {
            s_state->kept = {};
            s_state->keptFor.reset();
        }
        s_state->busy[monitor->m_id] = busy;
    }

    // ── The pace ─────────────────────────────────────────────────────────────

    // Sets every animation the config set itself to its speed at `quickness`
    // (2 is twice as quick), and every spring to its stiffness and damping at
    // it: a spring takes no speed, and settles in a time that goes as the root
    // of its stiffness, its bounce kept with the damping in step. The first call
    // after a reload remembers what the config set.
    void setPace(double quickness) {
        const auto& tree = Config::animationTree();
        if (!tree || !s_state || !Animation::mgr())
            return;
        auto& paces   = s_state->paces;
        auto& springs = s_state->springs;
        if (paces.empty() && springs.empty()) {
            for (const auto& [name, config] : tree->getAnimationConfig()) {
                if (config && config->pValues.get() == config.get() && config->internalSpeed > 0.f)
                    paces.emplace(name, config->internalSpeed);
            }
            for (const auto& [name, spring] : Animation::mgr()->getAllSprings()) {
                if (spring)
                    springs.emplace(name, *spring);
            }
        }
        for (const auto& [name, speed] : paces) {
            const auto config = tree->getAnimationPropertyConfig(name);
            if (config && config->pValues.get() == config.get())
                config->internalSpeed = static_cast<float>(speed / quickness);
        }
        for (const auto& [name, base] : springs) {
            if (const auto spring = Animation::mgr()->getSpring(name)) {
                spring->stiffness = static_cast<float>(base.stiffness * quickness * quickness);
                spring->damping   = static_cast<float>(base.damping * quickness);
            }
        }
    }
} // namespace

namespace Motion {

    void init() {
        s_state = std::make_unique<SState>();
    }

    void exit() {
        if (!s_state)
            return;
        for (auto& state : s_state->windows)
            detach(*state);
        if (g_pHyprRenderer) {
            g_pHyprRenderer->m_renderPass.removeAllOfType(GENIE_PASS);
            g_pHyprRenderer->m_renderPass.removeAllOfType(KEEP_PASS);
        }
        setPace(1.0);
        if (s_state->programReady || s_state->programTried) {
            s_state->program.destroy();
            if (s_state->vao)
                glDeleteVertexArrays(1, &s_state->vao);
            if (s_state->vbo)
                glDeleteBuffers(1, &s_state->vbo);
            if (s_state->ibo)
                glDeleteBuffers(1, &s_state->ibo);
        }
        s_state.reset();
    }

    void onOpen(PHLWINDOW window) {
        if (!enabled() || !window)
            return;
        const bool genie = on(GLASS_T_GENIE_OPEN);
        if (!genie && !on(GLASS_T_JELLY))
            return;
        const auto monitor = window->m_monitor.lock();
        if (!normal(monitor))
            return;
        ++s_state->counters.opens;
        auto& state    = track(window);
        state.open     = genie ? EOpen::Genie : EOpen::Jelly;
        state.openedAt = Clock::now();
        state.progress = 1.0;
        state.mouth    = mouthFor(monitor);
    }

    void onClose(PHLWINDOW window) {
        if (!window || !s_state)
            return;
        SGlass glass;
        if (auto* state = find(window)) {
            // a window bent as it closes: its glass of the last frame
            if (state->lift.copy && state->liftedIn + 2 >= g_pGlobalState->frameSerial)
                glass = state->lift;
            // its snapshot is taken next, and must be the window as it is
            detach(*state);
            state->open = EOpen::None;
        }
        if (!glass.copy && s_state->keptFor.lock() == window) {
            glass         = std::move(s_state->kept);
            s_state->kept = {};
            s_state->keptFor.reset();
        }
        // the genie draws in place of the fadeout, which only the hook on the
        // render pass can keep from being drawn as well
        const auto monitor = window->m_monitor.lock();
        if (!enabled() || !on(GLASS_T_GENIE_CLOSE) || !g_pGlobalState->renderPassAddHook || !normal(monitor) || !window->m_workspace ||
            !window->m_workspace->isVisible())
            return;
        CBox source = window->getFullWindowBoundingBox();
        source.translate(window->m_floatingOffset - monitor->m_position).scale(monitor->m_scale);
        if (source.w < 1.0 || source.h < 1.0)
            return;
        // poured without its glass it would be black where the glass was: a
        // window whose glass was not kept fades as Hyprland fades it
        if (const auto* decoration = glassDecorationFor(window); !glass.copy && decoration && decoration->isGlassEnabled())
            return;
        ++s_state->counters.closes;
        // as Hyprland starts the window's fadeout: by the next frame that has run on
        const float alpha = window->alphaValue(Desktop::View::WINDOW_ALPHA_FADE) * window->alphaValue(Desktop::View::WINDOW_ALPHA_FULLSCREEN) *
            window->alphaValue(Desktop::View::WINDOW_ALPHA_LAYOUT);
        s_state->closing.push_back({.monitor = monitor->m_id, .source = source, .mouth = mouthFor(monitor), .closedAt = Clock::now(), .alpha = std::clamp(alpha, 0.f, 1.f),
                                    .glass = std::move(glass)});
    }

    void onRenderPre(PHLMONITOR monitor) {
        if (!s_state || !g_pGlobalState || !monitor)
            return;
        advance(monitor);
        // the whole of this frame, since a bent window may be drawn anywhere,
        // and the next one too: the springs move on with nothing else to draw,
        // and the frame after the last must clear where the window was bent
        if (s_state->busy[monitor->m_id])
            g_pHyprRenderer->damageMonitor(monitor);
    }

    void onRenderStage(eRenderStage stage) {
        if (!s_state || !g_pGlobalState || stage != RENDER_POST_WINDOWS)
            return;
        if (const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock())
            queueGenies(monitor);
    }

    void onMonitorGone(MONITORID monitor) {
        if (!s_state)
            return;
        s_state->buffers.erase(monitor);
        s_state->busy.erase(monitor);
        std::erase_if(s_state->closing, [&](const auto& closing) { return closing.monitor == monitor; });
    }

    void beforeReload() {
        if (!s_state)
            return;
        setPace(1.0);
        s_state->paces.clear();
        s_state->springs.clear();
    }

    void applyPace() {
        if (s_state)
            setPace(pace());
    }

    std::string describe() {
        if (!s_state || !g_pGlobalState)
            return "hyprglass motion: not running\n";
        const auto& counters = s_state->counters;
        std::string out = std::format("hyprglass motion: {}, pace {:.2f}, genie {:.2f} s{}{}{}, jelly {}, wobble {}{}\n", enabled() ? "on" : "off", pace(), genieSeconds(),
                                      on(GLASS_T_GENIE_CLOSE) ? " on close" : "", on(GLASS_T_GENIE_OPEN) ? " on open" : "", on(GLASS_T_GENIE_POINTER) ? " to the pointer" : "",
                                      on(GLASS_T_JELLY) ? "on" : "off", on(GLASS_T_WOBBLE) ? "on" : "off", on(GLASS_T_WOBBLE_MOVES) ? " and when tiled" : "");
        out += std::format("  render pass hook {}, program {}\n", g_pGlobalState->renderPassAddHook ? "in place" : "missing",
                           s_state->programReady ? "ready" : s_state->programTried ? "failed" : "not built yet");
        out += std::format("  opens {}, closes {}, fadeouts {}, matched {}, genies queued {}, drawn {}, transforms {}, meshes {}, lifts {}\n", counters.opens, counters.closes,
                           counters.fadeouts, counters.matched, counters.genies, counters.genieDraws, counters.transforms, counters.meshes, counters.lifts);
        const auto dragged = draggedWindow();
        out += std::format("  dragging {}, {} frames dragged in all\n", dragged ? dragged->m_class : std::string{"nothing"}, counters.dragged);
        out += std::format("  window kept {} times, now {}\n", counters.keeps, s_state->kept.copy && !s_state->keptFor.expired() ? s_state->keptFor.lock()->m_class : std::string{"none"});
        if (!s_state->lastGenie.empty())
            out += "  last genie: " + s_state->lastGenie + "\n";
        out += std::format("  windows {}, closing {}, springs at pace {}, animations {}\n", s_state->windows.size(), s_state->closing.size(), s_state->springs.size(),
                           s_state->paces.size());
        for (const auto& state : s_state->windows) {
            const auto window = state->window.lock();
            out += std::format("    {} {}{}{}{}\n", window ? window->m_class : std::string{"(gone)"}, state->attached ? "bent" : "flat",
                               state->open == EOpen::Genie ? ", genie" : state->open == EOpen::Jelly ? ", jelly" : "", state->moving ? ", moving" : "", state->held ? ", held" : "");
        }
        return out;
    }

    UP<IPassElement> keeper(IPassElement& surface) {
        if (!s_state || surface.type() != EK_SURFACE || g_pHyprRenderer->m_bRenderingSnapshot || RenderGuards::isForeignRender())
            return nullptr;
        const auto& data = static_cast<CSurfacePassElement&>(surface).m_data;
        if (!data.mainSurface || data.popup || !data.pWindow || !keeps(data.pWindow))
            return nullptr;
        // a window being bent is drawn bent, and its glass is lifted off
        if (const auto* state = find(data.pWindow); state && state->attached)
            return nullptr;
        return makeUnique<CKeepElement>(data.pWindow);
    }

    bool swallows(IPassElement& element) {
        if (!s_state || s_state->closing.empty() || element.type() != EK_TEXTURE)
            return false;
        const auto* texture = static_cast<CTexPassElement&>(element).m_data.tex.get();
        return texture && std::ranges::any_of(s_state->closing, [&](const auto& closing) { return closing.matched && closing.texture == texture; });
    }

    bool bends(const PHLWINDOW& window) {
        const auto* state = find(window);
        return state && state->attached;
    }

    CLift::CLift(const PHLWINDOW& window, const PHLMONITOR& monitor) : m_window(window) {
        const auto& target = g_pHyprRenderer->m_renderData.currentFB;
        if (!s_state || !window || !normal(monitor) || !target || !fbId(target) || g_pHyprRenderer->m_bRenderingSnapshot)
            return;
        auto* state = find(window);
        if (!state || !state->attached)
            return;
        const auto shape = liftShape(window, monitor, target);
        if (!shape)
            return;

        state->lift.box  = shape->first;
        state->lift.pane = shape->second;
        ensure(state->save, "hyprglass-motion-save", static_cast<int>(shape->first.w), static_cast<int>(shape->first.h), target->m_drmFormat);
        g_pHyprOpenGL->setCapStatus(GL_SCISSOR_TEST, false);
        if (glIsEnabled(GL_SCISSOR_TEST))
            glDisable(GL_SCISSOR_TEST);
        blit(target, shape->first, state->save, {0, 0, shape->first.w, shape->first.h});
        glBindFramebuffer(GL_FRAMEBUFFER, fbId(target));
        m_active = true;
    }

    CLift::~CLift() {
        if (!m_active || !s_state)
            return;
        const auto  window  = m_window.lock();
        const auto& target  = g_pHyprRenderer->m_renderData.currentFB;
        const auto  monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
        if (!window || !target || !monitor)
            return;

        auto* state = find(window);
        if (!state || !state->save)
            return;
        copyGlass(state->lift, window, monitor, target, "hyprglass-motion-lift");
        blit(state->save, {0, 0, state->lift.box.w, state->lift.box.h}, target, state->lift.box);
        glBindFramebuffer(GL_FRAMEBUFFER, fbId(target));
        state->liftedIn = g_pGlobalState->frameSerial;
        ++s_state->counters.lifts;
    }

} // namespace Motion
