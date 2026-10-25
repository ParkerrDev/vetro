#pragma once

#include "PresetTypes.hpp"

#include <hyprland/src/config/shared/Types.hpp>
#include <hyprland/src/plugins/PluginAPI.hpp>
#include <optional>
#include <string>
#include <string_view>
#include <typeinfo>
#include <unordered_map>

inline constexpr std::string_view CONFIG_PREFIX = "plugin:hyprglass:";

// Window tags for theme and preset selection
inline constexpr std::string_view TAG_THEME_PREFIX  = "hyprglass_theme_";
inline constexpr std::string_view TAG_PRESET_PREFIX = "hyprglass_preset_";

// Window tags for per-window enable/disable. Override the global `enabled` setting.
// `hyprglass_disabled` always wins if both are present.
inline constexpr std::string_view TAG_ENABLED  = "hyprglass_enabled";
inline constexpr std::string_view TAG_DISABLED = "hyprglass_disabled";
// A window whose own content is opaque, whatever its buffer's format says (a
// browser, an Electron app): its glass is its frame alone, and nothing under
// its middle can show, which Hyprland is told so that it draws none of it.
inline constexpr std::string_view TAG_SOLID = "hyprglass_solid";

// Layer glass masking strategy: how hyprglass decides which pixels of a layer
// surface get the glass effect.
enum class ELayerMaskMode { AUTO, ALPHA, REGION };

// Parses "auto"/"alpha"/"region"; nullopt for anything else.
[[nodiscard]] std::optional<ELayerMaskMode> parseLayerMaskMode(std::string_view value);

// Instrumentation mode (plugin:hyprglass:debug:mode). Isolates the render-pass
// cost from the GL pipeline cost for A/B measurement: hints_only skips all GL
// work in the pass elements while keeping their boundingBox/needsLiveBlur/
// disableSimplification hints; gl_work_only runs the GL pipeline as normal but
// forces needsLiveBlur/disableSimplification off on both pass elements.
enum class EDebugMode { OFF, HINTS_ONLY, GL_WORK_ONLY };

// Parses "off"/"hints_only"/"gl_work_only"; nullopt for anything else.
[[nodiscard]] std::optional<EDebugMode> parseDebugMode(std::string_view value);

// Reads plugin:hyprglass:debug:mode, falling back to OFF for an unset global
// state or an unrecognized value (validateConfig() warns about the latter).
[[nodiscard]] EDebugMode currentDebugMode();

namespace ConfigKeys {

// Global-only
inline constexpr auto ENABLED             = "plugin:hyprglass:enabled";
inline constexpr auto DEFAULT_THEME       = "plugin:hyprglass:default_theme";
inline constexpr auto DEFAULT_PRESET      = "plugin:hyprglass:default_preset";
inline constexpr auto MANAGE_WINDOW_BLUR  = "plugin:hyprglass:manage_window_blur";
inline constexpr auto SKIP_OPAQUE_WINDOWS = "plugin:hyprglass:skip_opaque_windows";
inline constexpr auto BLUR_FOLD           = "plugin:hyprglass:blur_fold";

// Performance diagnostics
inline constexpr auto DEBUG_MODE   = "plugin:hyprglass:debug:mode";
inline constexpr auto DEBUG_TIMERS = "plugin:hyprglass:debug:timers";

// Preset keyword, registered as unscoped because Hyprlang does not dispatch
// scoped keyword handlers inside the plugin special category.
inline constexpr auto PRESET_KEYWORD = "preset";

// Layer surface support
inline constexpr auto LAYERS_ENABLED            = "plugin:hyprglass:layers:enabled";
inline constexpr auto LAYERS_NAMESPACES         = "plugin:hyprglass:layers:namespaces";
inline constexpr auto LAYERS_EXCLUDE_NAMESPACES = "plugin:hyprglass:layers:exclude_namespaces";
inline constexpr auto LAYERS_PRESET             = "plugin:hyprglass:layers:preset";
inline constexpr auto LAYERS_NAMESPACE_PRESETS          = "plugin:hyprglass:layers:namespace_presets";
inline constexpr auto LAYERS_MASK_THRESHOLD             = "plugin:hyprglass:layers:mask_threshold";
inline constexpr auto LAYERS_NAMESPACE_MASK_THRESHOLDS  = "plugin:hyprglass:layers:namespace_mask_thresholds";
inline constexpr auto LAYERS_NAMESPACE_LIVE_RESAMPLE    = "plugin:hyprglass:layers:namespace_live_resample";
inline constexpr auto LAYERS_LIVE_RESAMPLE              = "plugin:hyprglass:layers:live_resample";
inline constexpr auto LAYERS_LIVE_RESAMPLE_FPS          = "plugin:hyprglass:layers:live_resample_fps";
inline constexpr auto LAYERS_FORCE_LIVE_RESAMPLE        = "plugin:hyprglass:layers:force_live_resample";
inline constexpr auto LAYERS_MASK_MODE                  = "plugin:hyprglass:layers:mask_mode";
inline constexpr auto LAYERS_NAMESPACE_MASK_MODES       = "plugin:hyprglass:layers:namespace_mask_modes";
// Alpha mask only: alpha range above mask_threshold over which layer glass fades in; 0 keeps the hard mask.
inline constexpr auto LAYERS_MASK_FEATHER               = "plugin:hyprglass:layers:mask_feather";
inline constexpr auto LAYERS_NAMESPACE_MASK_FEATHERS    = "plugin:hyprglass:layers:namespace_mask_feathers";
inline constexpr auto LAYERS_MANAGE_BLUR                = "plugin:hyprglass:layers:manage_blur";

// Subsurface item glass support (see GlassSubsurfaceState).
// Preset resolution: subsurfaces:preset -> layers:preset (if set) -> window default preset.
inline constexpr auto SUBSURFACES_ENABLED = "plugin:hyprglass:subsurfaces:enabled";
inline constexpr auto SUBSURFACES_PRESET  = "plugin:hyprglass:subsurfaces:preset";
// Glass shape radius, logical px. -1 (default) = capsule: min(w,h)/2 of the
// glass box (the item's blur-region extents, not necessarily its whole
// surface box). >= 0 = that exact radius, scaled by monitor scale.
inline constexpr auto SUBSURFACES_RADIUS  = "plugin:hyprglass:subsurfaces:radius";

// Window background cache kill switch; commit-driven invalidation (single
// global bool, no per-namespace concept for windows) and its throttle —
// mirrors the layers:live_resample/live_resample_fps keys above.
inline constexpr auto WINDOWS_BACKGROUND_CACHE  = "plugin:hyprglass:windows:background_cache";
inline constexpr auto WINDOWS_LIVE_RESAMPLE     = "plugin:hyprglass:windows:live_resample";
inline constexpr auto WINDOWS_LIVE_RESAMPLE_FPS = "plugin:hyprglass:windows:live_resample_fps";

// The window's border as glass. With windows:border on, a window's pane
// reaches as far beyond the window as Hyprland's border is wide, so the band
// the border takes shows glass; a window that is solid, which gets no pane, is
// given that band alone, as a frame. Set Hyprland's own border colours clear
// with it, or they are drawn over the glass. windows:border_dim is how
// strongly the rim of a window without the focus is lit, as a share of the
// focused window's (1 = no difference).
inline constexpr auto WINDOWS_BORDER     = "plugin:hyprglass:windows:border";
inline constexpr auto WINDOWS_BORDER_DIM = "plugin:hyprglass:windows:border_dim";

// The engine's own window animations (Motion.hpp): the genie, windows that land
// like jelly and wobble as they are dragged. Off unless the config asks for
// them; which of them run, and how, is the tuning's (glass.conf).
inline constexpr auto MOTION_ENABLED = "plugin:hyprglass:motion:enabled";

} // namespace ConfigKeys

inline std::string_view readStringConfig(const StringConfigPtr& ptr) {
    if (!ptr.dataptr || !ptr.type)
        return {};

    if (*ptr.type == typeid(Config::STRING)) {
        const auto* value = *reinterpret_cast<Config::STRING* const*>(ptr.dataptr);
        return value ? std::string_view(*value) : std::string_view{};
    }

    if (*ptr.type == typeid(Hyprlang::STRING)) {
        const auto value = *reinterpret_cast<Hyprlang::STRING const*>(ptr.dataptr);
        return value ? std::string_view(value) : std::string_view{};
    }

    return {};
}

inline Config::INTEGER readIntegerConfig(const IntegerConfigPtr& ptr) {
    if (!ptr.dataptr || !ptr.type)
        return 0;

    if (*ptr.type == typeid(bool))
        return **reinterpret_cast<const bool* const*>(ptr.dataptr);

    if (*ptr.type == typeid(Config::INTEGER))
        return **reinterpret_cast<const Config::INTEGER* const*>(ptr.dataptr);

    return 0;
}

// True when any tier can resolve self_sample above 0. Answers "could a window
// sample itself" without a window at hand, so the surface observer can stay
// unarmed on the default config.
[[nodiscard]] bool anySelfSampleConfigured(const SPluginConfig& config, const Glass::CStore& customPresets);

void registerConfig(HANDLE handle);
void initConfigPointers(HANDLE handle, SPluginConfig& config);

// Preset keyword handler (registered via addConfigKeyword)
Hyprlang::CParseResult handlePresetKeyword(const char* command, const char* value);

// Clear pending presets/layers before config re-parse (called from preConfigReload callback)
void clearPendingPresets();
void clearPendingLayers();

// Swap pending data into active maps (called from configReloaded callback)
void commitPendingPresets();
void commitPendingLayers();

// Validate config values and notify user of misconfigurations
void validateConfig();
