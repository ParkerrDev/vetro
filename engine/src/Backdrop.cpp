#include "Backdrop.hpp"
#include "Diagnostics.hpp"
#include "GlassRenderer.hpp"
#include "Globals.hpp"
#include "LayerGeometry.hpp"
#include "RenderGuards.hpp"

#include <algorithm>
#include <cmath>
#include <optional>
#include <GLES3/gl32.h>
#include <hyprland/src/desktop/view/LayerSurface.hpp>
#include <hyprland/src/desktop/view/WLSurface.hpp>
#include <hyprland/src/output/Monitor.hpp>
#include <hyprland/src/protocols/core/Compositor.hpp>
#include <hyprland/src/render/OpenGL.hpp>
#include <hyprland/src/render/pass/TexPassElement.hpp>
#include <hyprland/src/render/Renderer.hpp>
#include <hyprland/src/render/gl/GLFramebuffer.hpp>
#include <hyprutils/utils/ScopeGuard.hpp>

namespace {
    // About 10 s at 60 Hz of this monitor's own frames without a request.
    constexpr uint64_t IDLE_FRAMES = 600;
    // Hyprland's index of the layer-shell background layer.
    constexpr size_t BACKGROUND = 0;

    // FNV-1a over the bytes of one value: enough to tell one state of the
    // wallpaper from the next.
    uint64_t mix(uint64_t hash, uint64_t value) {
        for (int byte = 0; byte < 8; ++byte) {
            hash ^= (value >> (byte * 8)) & 0xffU;
            hash *= 0x100000001b3ULL;
        }
        return hash;
    }

    SP<CWLSurfaceResource> surfaceOf(const PHLLS& layer) {
        const auto wlSurface = layer->wlSurface();
        return wlSurface ? wlSurface->resource() : nullptr;
    }

    // The wallpaper as it stands: each mapped background layer's place, fade
    // and buffer, and the commits counted on their surfaces, which a picture
    // changed inside the same buffer leaves as the only sign.
    uint64_t signature(const PHLMONITOR& monitor, const SGlobalState::SBackdrop& state) {
        uint64_t hash = mix(0xcbf29ce484222325ULL, state.commits);
        for (const auto& ref : monitor->m_layerSurfaceLayers[BACKGROUND]) {
            const auto layer = ref.lock();
            if (!layer || !layer->m_mapped)
                continue;
            const auto box     = LayerGeometry::computeLayerBox(layer, monitor);
            const auto surface = surfaceOf(layer);
            hash = mix(hash, reinterpret_cast<uintptr_t>(layer.get()));
            hash = mix(hash, static_cast<uint64_t>(std::lround(layer->alpha().getTotal() * 1024.0f)));
            hash = mix(hash, surface ? reinterpret_cast<uintptr_t>(surface->m_current.texture.get()) : 0);
            if (box) {
                hash = mix(hash, static_cast<uint64_t>(box->x) ^ (static_cast<uint64_t>(box->y) << 32));
                hash = mix(hash, static_cast<uint64_t>(box->w) ^ (static_cast<uint64_t>(box->h) << 32));
            }
        }
        return hash;
    }

    // Counts the commits of each background surface, and forgets the surfaces
    // that are gone.
    void watch(const PHLMONITOR& monitor, SGlobalState::SBackdrop& state) {
        std::erase_if(state.watched, [](const auto& entry) { return entry.first.expired(); });
        for (const auto& ref : monitor->m_layerSurfaceLayers[BACKGROUND]) {
            const auto layer   = ref.lock();
            const auto surface = layer ? surfaceOf(layer) : nullptr;
            if (!surface)
                continue;
            const bool known = std::ranges::any_of(state.watched, [&](const auto& entry) { return entry.first.lock() == surface; });
            if (known)
                continue;
            const MONITORID id = monitor->m_id;
            state.watched.emplace_back(surface, surface->m_events.commit.listen([id] {
                if (!g_pGlobalState)
                    return;
                if (const auto it = g_pGlobalState->backdrops.find(id); it != g_pGlobalState->backdrops.end())
                    ++it->second.commits;
            }));
        }
    }

    // A wallpaper surface the way Hyprland draws it: its viewporter crop, if it
    // has one, picks the part of the buffer that fills its box.
    bool cropOf(const SP<CWLSurfaceResource>& surface, Vector2D& topLeft, Vector2D& bottomRight) {
        const auto& viewport   = surface->m_current.viewport;
        const auto& bufferSize = surface->m_current.bufferSize;
        if (!viewport.hasSource || bufferSize.x <= 0 || bufferSize.y <= 0)
            return false;
        topLeft     = {viewport.source.x / bufferSize.x, viewport.source.y / bufferSize.y};
        bottomRight = {(viewport.source.x + viewport.source.width) / bufferSize.x, (viewport.source.y + viewport.source.height) / bufferSize.y};
        return bottomRight.x > 0.00001 && bottomRight.y > 0.00001;
    }

    void drawLayers(const PHLMONITOR& monitor, const CRegion& fullDamage) {
        auto& renderData = g_pHyprRenderer->m_renderData;
        for (const auto& ref : monitor->m_layerSurfaceLayers[BACKGROUND]) {
            const auto layer = ref.lock();
            if (!layer || !layer->m_mapped)
                continue;
            const float alpha   = layer->alpha().getTotal();
            const auto  box     = LayerGeometry::computeLayerBox(layer, monitor);
            const auto  surface = surfaceOf(layer);
            if (alpha <= 0.0f || !box || !surface)
                continue;
            const auto& texture = surface->m_current.texture;
            // the texture draw asserts on both, and an assert kills the compositor
            if (!texture || !texture->ok() || surface->m_current.size.x < 1 || surface->m_current.size.y < 1)
                continue;

            Vector2D   topLeft{-1, -1};
            Vector2D   bottomRight{-1, -1};
            const bool cropped                     = cropOf(surface, topLeft, bottomRight);
            renderData.primarySurfaceUVTopLeft     = cropped ? topLeft : Vector2D(-1, -1);
            renderData.primarySurfaceUVBottomRight = cropped ? bottomRight : Vector2D(-1, -1);

            g_pHyprRenderer->draw(CTexPassElement::SRenderData{
                .tex           = texture,
                .box           = *box,
                .a             = alpha,
                .damage        = fullDamage,
                .allowCustomUV = cropped,
                .surface       = surface,
            });

            // The texture path tracks no buffer of its own: keep the one just
            // read until the frame is done with it.
            if (surface->m_current.buffer && !surface->m_current.buffer->isSynchronous())
                g_pHyprRenderer->m_usedAsyncBuffers.emplace_back(surface->m_current.buffer);
        }
    }

    // Draws the wallpaper into the backdrop: black, as Vetro sets
    // misc:background_color, then each mapped background layer at its place
    // and fade as Hyprland draws them, then the mip chain the blur reads.
    void draw(const PHLMONITOR& monitor, const SP<Render::IFramebuffer>& target) {
        auto&          renderData = g_pHyprRenderer->m_renderData;
        const Vector2D size       = target->m_size;

        // Declared before the framebuffer guard, so it runs after the frame's
        // own target is bound again, on the throwing path too.
        const Hyprutils::Utils::CScopeGuard restoreGLState([&] {
            g_pHyprOpenGL->scissor(nullptr);
            if (const auto& restored = renderData.currentFB)
                g_pHyprOpenGL->setViewport(0, 0, static_cast<int>(restored->m_size.x), static_cast<int>(restored->m_size.y));
        });

        auto guard = g_pHyprRenderer->bindTempFB(target);

        const auto savedProjection      = renderData.projectionType;
        const auto savedFbSize          = renderData.fbSize;
        const auto savedRenderModif     = renderData.renderModif;
        const auto savedWindow          = renderData.currentWindow;
        const auto savedSurface         = renderData.surface;
        const auto savedClipBox         = renderData.clipBox;
        const auto savedUVTopLeft       = renderData.primarySurfaceUVTopLeft;
        const auto savedUVBottomRight   = renderData.primarySurfaceUVBottomRight;
        const bool savedTransformDamage = renderData.transformDamage;
        const Hyprutils::Utils::CScopeGuard restoreRenderData([&] {
            renderData.primarySurfaceUVBottomRight = savedUVBottomRight;
            renderData.primarySurfaceUVTopLeft     = savedUVTopLeft;
            renderData.clipBox                     = savedClipBox;
            renderData.surface                     = savedSurface;
            renderData.currentWindow               = savedWindow;
            renderData.renderModif                 = savedRenderModif;
            renderData.transformDamage             = savedTransformDamage;
            // before setProjectionType: RPT_FB recomputes the projection from fbSize
            renderData.fbSize = savedFbSize;
            g_pHyprRenderer->setProjectionType(savedProjection);
        });

        renderData.fbSize = size;
        g_pHyprRenderer->setProjectionType(Render::RPT_EXPORT);
        // the boxes and the damage are backdrop pixels, not monitor space
        renderData.transformDamage = false;
        // an overview plugin's modifier would otherwise be applied a second time
        renderData.renderModif = {};
        renderData.clipBox     = {};
        renderData.currentWindow.reset();

        g_pHyprOpenGL->setViewport(0, 0, static_cast<int>(size.x), static_cast<int>(size.y));
        g_pHyprOpenGL->setCapStatus(GL_SCISSOR_TEST, false);
        if (glIsEnabled(GL_SCISSOR_TEST))
            glDisable(GL_SCISSOR_TEST);
        glClearColor(0.0f, 0.0f, 0.0f, 1.0f);
        glClear(GL_COLOR_BUFFER_BIT);
        g_pHyprRenderer->blend(true);

        drawLayers(monitor, CRegion{CBox{0, 0, size.x, size.y}});
        guard.reset();

        // The blur reads levels of it (blurLOD), as macOS 27 does. The texture is
        // the engine's alone; the setter is Hyprland's, so its record stays true.
        glActiveTexture(GL_TEXTURE0);
        const auto texture = target->getTexture();
        texture->bind();
        texture->setTexParameter(GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR);
        texture->setTexParameter(GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        texture->setTexParameter(GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        texture->setTexParameter(GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        glGenerateMipmap(GL_TEXTURE_2D);
        glBindTexture(GL_TEXTURE_2D, 0);
        Diagnostics::recordXrayCopy(monitor->m_id);
    }
    // A blur's radius is at most this many pixels at the size it is made at.
    constexpr float MAX_SMALL_RADIUS = 16.0f;
    // The Gaussian pass takes eight taps a side, a radius of 8.
    constexpr float PASS_RADIUS  = 8.0f;
    constexpr int   MAX_PASSES   = 4;

    // The mip level, 1 to 3, the blur is made at: the largest that keeps the
    // radius within MAX_SMALL_RADIUS there.
    int levelFor(float radius) {
        int level = 1;
        while (level < 3 && radius / static_cast<float>(1 << level) > MAX_SMALL_RADIUS)
            ++level;
        return level;
    }

    std::optional<GLuint> framebufferId(const SP<Render::IFramebuffer>& framebuffer) {
        auto* gl = dynamic_cast<Render::GL::CGLFramebuffer*>(framebuffer.get());
        if (!gl)
            return std::nullopt;
        return gl->getFBID();
    }

    // Copies mip level `level` of `source`'s texture into `target`, a texel to a
    // pixel: the mip chain has already averaged the boxes, so nothing aliases.
    bool copyLevel(const SP<Render::IFramebuffer>& source, int level, const SP<Render::IFramebuffer>& target) {
        const auto targetId = framebufferId(target);
        if (!targetId)
            return false;
        const int width  = static_cast<int>(target->m_size.x);
        const int height = static_cast<int>(target->m_size.y);
        GLuint    read   = 0;
        glGenFramebuffers(1, &read);
        glBindFramebuffer(GL_READ_FRAMEBUFFER, read);
        glFramebufferTexture2D(GL_READ_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, source->getTexture()->m_texID, level);
        const bool complete = glCheckFramebufferStatus(GL_READ_FRAMEBUFFER) == GL_FRAMEBUFFER_COMPLETE;
        if (complete) {
            g_pHyprOpenGL->setCapStatus(GL_SCISSOR_TEST, false);
            if (glIsEnabled(GL_SCISSOR_TEST))
                glDisable(GL_SCISSOR_TEST);
            glBindFramebuffer(GL_DRAW_FRAMEBUFFER, *targetId);
            glBlitFramebuffer(0, 0, width, height, 0, 0, width, height, GL_COLOR_BUFFER_BIT, GL_NEAREST);
        }
        glBindFramebuffer(GL_READ_FRAMEBUFFER, 0);
        glDeleteFramebuffers(1, &read);
        return complete;
    }

    // The backdrop blurred to `radius` device pixels into `state.blurred`.
    bool makeBlurred(SGlobalState::SBackdrop& state, float radius, const SP<Render::IFramebuffer>& frame) {
        return Backdrop::blurInto(state.framebuffer, state.blurred, radius, frame);
    }
} // namespace

namespace Backdrop {

    // A mip level of `source` copied out, then Gaussian passes over that, which
    // compound in quadrature (n passes of r come to about r times the root of n).
    bool blurInto(const SP<Render::IFramebuffer>& source, SP<Render::IFramebuffer>& out, float radius, const SP<Render::IFramebuffer>& frame) {
        if (!source || !(radius >= 1.0f))
            return false;
        const int level  = levelFor(radius);
        const int width  = std::max(1, static_cast<int>(source->m_size.x) >> level);
        const int height = std::max(1, static_cast<int>(source->m_size.y) >> level);
        if (!out)
            out = g_pHyprRenderer->createFB("hyprglass-blur");
        if (static_cast<int>(out->m_size.x) != width || static_cast<int>(out->m_size.y) != height || out->m_drmFormat != source->m_drmFormat) {
            if (!out->alloc(width, height, source->m_drmFormat))
                return false;
        }
        if (!copyLevel(source, level, out))
            return false;
        const float small  = radius / static_cast<float>(1 << level);
        const int   passes = std::clamp(static_cast<int>(std::ceil((small / PASS_RADIUS) * (small / PASS_RADIUS))), 1, MAX_PASSES);
        GlassRenderer::blurBackground(out, small / std::sqrt(static_cast<float>(passes)), passes, frame);
        const auto texture = out->getTexture();
        glActiveTexture(GL_TEXTURE0);
        texture->bind();
        texture->setTexParameter(GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        texture->setTexParameter(GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        texture->setTexParameter(GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        texture->setTexParameter(GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        glBindTexture(GL_TEXTURE_2D, 0);
        if (const auto frameId = framebufferId(frame))
            glBindFramebuffer(GL_FRAMEBUFFER, *frameId);
        return true;
    }

    bool possible(const PHLMONITOR& monitor) {
        if (!g_pGlobalState || !monitor)
            return false;
        return readIntegerConfig(g_pGlobalState->config.xpMode) == 0 && monitor->m_transform == WL_OUTPUT_TRANSFORM_NORMAL;
    }

    void request(const PHLMONITOR& monitor) {
        if (!g_pGlobalState || !monitor)
            return;
        auto& state            = g_pGlobalState->backdrops[monitor->m_id];
        state.lastRequestFrame = state.monitorFrames;
    }

    uint64_t stamp(const PHLMONITOR& monitor) {
        if (!g_pGlobalState || !monitor)
            return 0;
        const auto it = g_pGlobalState->backdrops.find(monitor->m_id);
        return it == g_pGlobalState->backdrops.end() ? 0 : signature(monitor, it->second);
    }

    void beginFrame() {
        const auto monitor = g_pHyprRenderer->m_renderData.pMonitor.lock();
        if (!g_pGlobalState || !monitor)
            return;

        auto&      backdrops = g_pGlobalState->backdrops;
        const auto it        = backdrops.find(monitor->m_id);
        if (it == backdrops.end())
            return;

        // Here, for every frame, so a solitary fullscreen client's frames still
        // age the backdrop out.
        if (++it->second.monitorFrames > it->second.lastRequestFrame + IDLE_FRAMES) {
            backdrops.erase(it);
            Diagnostics::recordXrayEvict(monitor->m_id);
        }
    }

    SP<Render::IFramebuffer> get(const PHLMONITOR& monitor, const SP<Render::IFramebuffer>& frame) {
        // Not the monitor's own frame (an overview plugin's, a screenshot's): its
        // size need not be the monitor's.
        if (!frame || !possible(monitor) || RenderGuards::isForeignRender() || g_pHyprRenderer->m_bRenderingSnapshot)
            return nullptr;

        const int width  = static_cast<int>(monitor->m_pixelSize.x);
        const int height = static_cast<int>(monitor->m_pixelSize.y);
        if (width < 1 || height < 1 || static_cast<int>(frame->m_size.x) != width || static_cast<int>(frame->m_size.y) != height)
            return nullptr;

        auto& state            = g_pGlobalState->backdrops[monitor->m_id];
        state.lastRequestFrame = state.monitorFrames;
        watch(monitor, state);

        bool fresh = false;
        if (!state.framebuffer)
            state.framebuffer = g_pHyprRenderer->createFB("hyprglass-backdrop");
        if (static_cast<int>(state.framebuffer->m_size.x) != width || static_cast<int>(state.framebuffer->m_size.y) != height ||
            state.framebuffer->m_drmFormat != frame->m_drmFormat) {
            if (!state.framebuffer->alloc(width, height, frame->m_drmFormat)) {
                state.framebuffer.reset();
                return nullptr;
            }
            fresh = true;
        }

        const uint64_t now = signature(monitor, state);
        if (fresh || now != state.drawnSignature) {
            draw(monitor, state.framebuffer);
            state.drawnSignature = now;
        }
        return state.framebuffer;
    }

    SP<Render::IFramebuffer> blurred(const PHLMONITOR& monitor, const SP<Render::IFramebuffer>& frame, float radius) {
        if (!(radius >= 1.0f) || !get(monitor, frame))
            return nullptr;
        auto& state = g_pGlobalState->backdrops[monitor->m_id];
        const bool current = state.blurred && state.blurredSignature == state.drawnSignature && std::abs(state.blurredRadius - radius) < 0.5f;
        if (current)
            return state.blurred;
        if (!makeBlurred(state, radius, frame)) {
            state.blurred.reset();
            return nullptr;
        }
        state.blurredSignature = state.drawnSignature;
        state.blurredRadius    = radius;
        return state.blurred;
    }

    bool isBackdropOrBlur(const SP<Render::IFramebuffer>& framebuffer) {
        if (!g_pGlobalState || !framebuffer)
            return false;
        return std::ranges::any_of(g_pGlobalState->backdrops,
                                   [&](const auto& entry) { return entry.second.framebuffer == framebuffer || entry.second.blurred == framebuffer; });
    }

    bool isBackdrop(const SP<Render::IFramebuffer>& framebuffer) {
        if (!g_pGlobalState || !framebuffer)
            return false;
        return std::ranges::any_of(g_pGlobalState->backdrops, [&](const auto& entry) { return entry.second.framebuffer == framebuffer; });
    }

    void forget(MONITORID monitor) {
        if (g_pGlobalState)
            g_pGlobalState->backdrops.erase(monitor);
    }

    void clear() {
        if (g_pGlobalState)
            g_pGlobalState->backdrops.clear();
    }

} // namespace Backdrop
