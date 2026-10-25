#pragma once

// The plain data of the configuration: where each option's value lives in
// Hyprland's config, and the context an option is resolved in. What an option
// is, which preset sets it and in what order the levels are asked is the
// Rust model's (crates/glass-core, options); this side only reads the values
// out of Hyprland's storage and hands them over.

#include "GlassBridge.hpp"

#include <array>
#include <cstdint>
#include <hyprlang.hpp>
#include <optional>
#include <string>
#include <typeinfo>

// Where the value of every option is kept at one level of the config (global,
// dark: or light:), by the index the model numbers the options with (the
// GLASS_F_ and GLASS_I_ constants). A null pointer is an option the config
// does not have.
struct SOverridableConfig {
    std::array<Hyprlang::FLOAT* const*, GLASS_FLOAT_OPTIONS> floats{};
    std::array<Hyprlang::INT* const*, GLASS_INT_OPTIONS>     ints{};
};

struct StringConfigPtr {
    void* const*          dataptr = nullptr;
    const std::type_info* type    = nullptr;
};


// A Hyprland key whose storage is a bool under the Lua config and an INTEGER
// under legacy .conf: read through its reported type, never as a bare int64.
struct IntegerConfigPtr {
    void* const*          dataptr = nullptr;
    const std::type_info* type    = nullptr;
};

struct SPluginConfig {
    Hyprlang::INT* const* enabled           = nullptr;
    // Glass replaces Hyprland's blur for glassed windows: when set, the plugin
    // marks them with the noblur window property so Hyprland composites them
    // against the live framebuffer (which contains the glass) instead of its
    // pre-frame cached blur.
    Hyprlang::INT* const* manageWindowBlur  = nullptr;
    // Skip glass for windows CWindow::opaque() reports as opaque: nothing behind
    // them is visible, so sampling and blurring their background is wasted work.
    Hyprlang::INT* const* skipOpaqueWindows = nullptr;
    // Derives a smaller blur pass count from the requested radius (GlassRenderer::
    // foldBlurPasses) instead of always running blur_iterations passes at full radius.
    Hyprlang::INT* const* blurFold = nullptr;
    // Hyprland's render:xp_mode: no wallpaper or bottom layers are drawn, so
    // there is nothing for x-ray to show.
    IntegerConfigPtr      xpMode;
    StringConfigPtr      defaultTheme;
    StringConfigPtr      defaultPreset;

    // Performance diagnostics (see Diagnostics.hpp for the hyprctl side)
    StringConfigPtr       debugMode;
    Hyprlang::INT* const* debugTimers = nullptr;

    Hyprlang::INT* const* layersEnabled                  = nullptr;
    StringConfigPtr       layersNamespaces;
    StringConfigPtr       layersExcludeNamespaces;
    StringConfigPtr       layersPreset;
    StringConfigPtr       layersNamespacePresets;
    Hyprlang::FLOAT* const* layersMaskThreshold          = nullptr;
    StringConfigPtr       layersNamespaceMaskThresholds;
    StringConfigPtr       layersNamespaceLiveResample;
    Hyprlang::INT* const* layersLiveResample             = nullptr;
    Hyprlang::INT* const* layersLiveResampleFps          = nullptr;
    Hyprlang::INT* const* layersForceLiveResample        = nullptr;
    StringConfigPtr       layersMaskMode;
    StringConfigPtr       layersNamespaceMaskModes;
    Hyprlang::FLOAT* const* layersMaskFeather            = nullptr;
    StringConfigPtr       layersNamespaceMaskFeathers;
    Hyprlang::INT* const* layersManageBlur               = nullptr;

    Hyprlang::INT* const*   subsurfacesEnabled = nullptr;
    StringConfigPtr         subsurfacesPreset;
    Hyprlang::FLOAT* const* subsurfacesRadius  = nullptr;

    Hyprlang::INT* const* windowsBackgroundCache  = nullptr;
    Hyprlang::INT* const* windowsLiveResample     = nullptr;
    Hyprlang::INT* const* windowsLiveResampleFps  = nullptr;
    Hyprlang::INT* const*   windowsBorder         = nullptr;
    Hyprlang::FLOAT* const* windowsBorderDim      = nullptr;
    Hyprlang::INT* const*   motionEnabled         = nullptr;

    SOverridableConfig global;
    SOverridableConfig dark;
    SOverridableConfig light;
};

// The context an option is asked in: which preset, which appearance, and the
// config the levels come from. The options are resolved once, on the first
// question, by the model's chain (preset, theme level, global level).
struct SResolveContext {
    SResolveContext(const std::string& preset, bool dark, const SPluginConfig& cfg, const Glass::CStore& store) : presetName(preset), isDark(dark), config(cfg), presets(store) {}

    const std::string&   presetName;
    bool                 isDark;
    const SPluginConfig& config;
    const Glass::CStore& presets;

    // Every option as the chain resolves it: negative where nothing sets it.
    [[nodiscard]] const GlassValues& values() const;

    // The value of a float option (a GLASS_F_ constant), or fallback when nothing sets it.
    [[nodiscard]] float f(uint32_t option, float fallback = -1.0f) const {
        const float value = option < GLASS_FLOAT_OPTIONS ? values().f[option] : -1.0f;
        return value >= 0.0f ? value : fallback;
    }

    // The value of an int option (a GLASS_I_ constant), or fallback when nothing sets it.
    [[nodiscard]] int64_t i(uint32_t option, int64_t fallback = -1) const {
        const int64_t value = option < GLASS_INT_OPTIONS ? values().i[option] : -1;
        return value >= 0 ? value : fallback;
    }

  private:
    mutable std::optional<GlassValues> m_values;
};
