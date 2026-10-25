#include "PresetTypes.hpp"

// Reads the config's levels out of Hyprland's storage and asks the model to
// resolve every option through them. Nothing here decides which level wins.

namespace {

// The values of one level; negative where the config has no such option or leaves it unset.
GlassValues snapshot(const SOverridableConfig& level) {
    GlassValues values{};
    for (size_t i = 0; i < level.floats.size(); ++i) {
        const auto pointer = level.floats[i];
        values.f[i]        = pointer && *pointer ? static_cast<float>(**pointer) : -1.0f;
    }
    for (size_t i = 0; i < level.ints.size(); ++i) {
        const auto pointer = level.ints[i];
        values.i[i]        = pointer && *pointer ? static_cast<int64_t>(**pointer) : -1;
    }
    return values;
}

} // namespace

const GlassValues& SResolveContext::values() const {
    if (m_values)
        return *m_values;

    GlassResolveRequest request{};
    request.store  = GlassBytes{presets.raw()->data, presets.raw()->len};
    request.preset = Glass::bytes(presetName);
    request.dark   = isDark ? 1 : 0;
    request.levels = GlassLevels{snapshot(config.global), snapshot(config.dark), snapshot(config.light)};

    GlassValues resolved{};
    if (glass_resolve_options(&request, &resolved) != GLASS_OK) {
        // the model could not answer: nothing is set anywhere, and every caller falls back to its own default
        for (auto& value : resolved.f)
            value = -1.0f;
        for (auto& value : resolved.i)
            value = -1;
    }
    m_values = resolved;
    return *m_values;
}
