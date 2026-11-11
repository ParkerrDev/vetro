#pragma once
#include <hyprland/src/render/Renderer.hpp>
#include <hyprland/src/desktop/view/window/WindowPresentation.hpp>
#include <hyprland/src/desktop/view/window/WindowEffectsController.hpp>
#include <hyprland/src/render/transformer/TransformerList.hpp>
namespace VetroRender {
inline thread_local Render::CRenderContext* active = nullptr;
inline Render::CRenderContext& context() {
    return active ? *active : g_pHyprRenderer->context();
}
class Scope {
    Render::CRenderContext* previous;
public:
    explicit Scope(Render::CRenderContext& ctx) : previous(active) { active = &ctx; }
    ~Scope() { active = previous; }
    Scope(const Scope&) = delete;
    Scope& operator=(const Scope&) = delete;
};
}
