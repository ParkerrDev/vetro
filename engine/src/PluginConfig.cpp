#include "PluginConfig.hpp"
#include "Globals.hpp"

#include <algorithm>
#include <charconv>
#include <cstring>
#include <hyprland/src/config/ConfigManager.hpp>
#include <hyprland/src/config/lua/ConfigManager.hpp>
#include <hyprland/src/config/values/ConfigValues.hpp>
#include <hyprland/src/helpers/Color.hpp>
#include <hyprland/src/plugins/PluginAPI.hpp>

extern "C" {
#include <lauxlib.h>
#include <lua.h>
}

// ── Config registration ──────────────────────────────────────────────────────

namespace {

template <typename T, typename Default>
static void addConfigValue(HANDLE handle, const char* name, Default defaultValue) {
    HyprlandAPI::addConfigValueV2(handle,
        Config::Values::makeConfigValue<T>(name, "", defaultValue, Config::Values::valueOptions_t<T>{}));
}

// The name of option `index` of `kind` (0 float, 1 int) at a level of the config
// ("", "dark:" or "light:"), and the value the global level is registered with.
struct SOptionKey {
    std::string key;
    double      globalDefault = -1.0;
};

std::optional<SOptionKey> optionKey(uint32_t kind, uint32_t index) {
    GlassOptionInfo info{};
    if (glass_option_info(kind, index, &info) != GLASS_OK)
        return std::nullopt;
    return SOptionKey{std::string(reinterpret_cast<const char*>(info.key), info.key_len), info.global_default};
}

// Every option the model knows, at the three levels. The global level carries
// the default the model registers it with (a negative one is "not set": a
// preset or the model's own default decides); dark: and light: are never set.
void registerOptions(HANDLE handle) {
    for (const char* level : {"", "dark:", "light:"}) {
        const bool global = level[0] == '\0';
        for (uint32_t kind = 0; kind < 2; ++kind) {
            const uint32_t count = kind == 0 ? GLASS_FLOAT_OPTIONS : GLASS_INT_OPTIONS;
            for (uint32_t index = 0; index < count; ++index) {
                const auto option = optionKey(kind, index);
                if (!option)
                    continue;
                const std::string name  = std::string(CONFIG_PREFIX) + level + option->key;
                const double      value = global ? option->globalDefault : -1.0;
                if (kind == 0)
                    addConfigValue<Config::Values::Float>(handle, name.c_str(), Config::FLOAT{value});
                else
                    addConfigValue<Config::Values::Int>(handle, name.c_str(), Config::INTEGER{static_cast<int64_t>(value)});
            }
        }
    }
}

} // namespace

// Forward declarations for Lua handlers (need access to file-static preset/layer data below)
static int handleLuaPreset(lua_State* L);
static int handleLuaLayer(lua_State* L);
static int handleLuaConfig(lua_State* L);
static int handleLuaMotion(lua_State* L);

std::optional<ELayerMaskMode> parseLayerMaskMode(std::string_view value) {
    if (value == "auto")   return ELayerMaskMode::AUTO;
    if (value == "alpha")  return ELayerMaskMode::ALPHA;
    if (value == "region") return ELayerMaskMode::REGION;
    return std::nullopt;
}

std::optional<EDebugMode> parseDebugMode(std::string_view value) {
    if (value == "off")          return EDebugMode::OFF;
    if (value == "hints_only")   return EDebugMode::HINTS_ONLY;
    if (value == "gl_work_only") return EDebugMode::GL_WORK_ONLY;
    return std::nullopt;
}

EDebugMode currentDebugMode() {
    if (!g_pGlobalState)
        return EDebugMode::OFF;
    return parseDebugMode(readStringConfig(g_pGlobalState->config.debugMode)).value_or(EDebugMode::OFF);
}

void registerConfig(HANDLE handle) {
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::ENABLED, Config::INTEGER{1});
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::MANAGE_WINDOW_BLUR, Config::INTEGER{1});
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::SKIP_OPAQUE_WINDOWS, Config::INTEGER{1});
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::BLUR_FOLD, Config::INTEGER{1});
    addConfigValue<Config::Values::String>(handle, ConfigKeys::DEFAULT_THEME, Config::STRING{"dark"});
    // Regular glass is the default material (docs/model.md). "default" brings back the bare option defaults.
    addConfigValue<Config::Values::String>(handle, ConfigKeys::DEFAULT_PRESET, Config::STRING{"regular"});

    // Performance diagnostics
    addConfigValue<Config::Values::String>(handle, ConfigKeys::DEBUG_MODE, Config::STRING{"off"});
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::DEBUG_TIMERS, Config::INTEGER{0});

    // Layer surface support
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::LAYERS_ENABLED, Config::INTEGER{0});
    addConfigValue<Config::Values::String>(handle, ConfigKeys::LAYERS_NAMESPACES, Config::STRING{});
    addConfigValue<Config::Values::String>(handle, ConfigKeys::LAYERS_EXCLUDE_NAMESPACES, Config::STRING{});
    addConfigValue<Config::Values::String>(handle, ConfigKeys::LAYERS_PRESET, Config::STRING{});
    addConfigValue<Config::Values::String>(handle, ConfigKeys::LAYERS_NAMESPACE_PRESETS, Config::STRING{});
    addConfigValue<Config::Values::Float>(handle, ConfigKeys::LAYERS_MASK_THRESHOLD, Config::FLOAT{0.001});
    addConfigValue<Config::Values::String>(handle, ConfigKeys::LAYERS_NAMESPACE_MASK_THRESHOLDS, Config::STRING{});
    addConfigValue<Config::Values::String>(handle, ConfigKeys::LAYERS_NAMESPACE_LIVE_RESAMPLE, Config::STRING{});
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::LAYERS_LIVE_RESAMPLE, Config::INTEGER{1});
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::LAYERS_LIVE_RESAMPLE_FPS, Config::INTEGER{30});
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::LAYERS_FORCE_LIVE_RESAMPLE, Config::INTEGER{0});
    addConfigValue<Config::Values::String>(handle, ConfigKeys::LAYERS_MASK_MODE, Config::STRING{"auto"});
    addConfigValue<Config::Values::String>(handle, ConfigKeys::LAYERS_NAMESPACE_MASK_MODES, Config::STRING{});
    addConfigValue<Config::Values::Float>(handle, ConfigKeys::LAYERS_MASK_FEATHER, Config::FLOAT{0.0});
    addConfigValue<Config::Values::String>(handle, ConfigKeys::LAYERS_NAMESPACE_MASK_FEATHERS, Config::STRING{});
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::LAYERS_MANAGE_BLUR, Config::INTEGER{1});

    // Subsurface item glass
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::SUBSURFACES_ENABLED, Config::INTEGER{0});
    addConfigValue<Config::Values::String>(handle, ConfigKeys::SUBSURFACES_PRESET, Config::STRING{});
    addConfigValue<Config::Values::Float>(handle, ConfigKeys::SUBSURFACES_RADIUS, Config::FLOAT{-1.0});

    // Window background cache
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::WINDOWS_BACKGROUND_CACHE, Config::INTEGER{1});
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::WINDOWS_LIVE_RESAMPLE, Config::INTEGER{1});
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::WINDOWS_LIVE_RESAMPLE_FPS, Config::INTEGER{30});

    // The window's border as glass
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::WINDOWS_BORDER, Config::INTEGER{0});
    addConfigValue<Config::Values::Float>(handle, ConfigKeys::WINDOWS_BORDER_DIM, Config::FLOAT{1.0});
    addConfigValue<Config::Values::Int>(handle, ConfigKeys::MOTION_ENABLED, Config::INTEGER{0});

    registerOptions(handle);

    // Legacy config keyword plus Lua-config callbacks for custom presets and layers.
    HyprlandAPI::addConfigKeyword(handle, ConfigKeys::PRESET_KEYWORD, handlePresetKeyword, Hyprlang::SHandlerOptions{});
    HyprlandAPI::addLuaFunction(handle, "hyprglass", "preset", handleLuaPreset);
    HyprlandAPI::addLuaFunction(handle, "hyprglass", "layer", handleLuaLayer);
    HyprlandAPI::addLuaFunction(handle, "hyprglass", "config", handleLuaConfig);
    HyprlandAPI::addLuaFunction(handle, "hyprglass", "motion", handleLuaMotion);
}

// ── Config pointer initialization ────────────────────────────────────────────

template <typename T>
static auto* getStaticPtr(HANDLE /*handle*/, const char* key) {
    return reinterpret_cast<T* const*>(Config::mgr()->getConfigValue(key).dataptr);
}

static StringConfigPtr getStringPtr(HANDLE /*handle*/, const char* key) {
    const auto value = Config::mgr()->getConfigValue(key);
    return {.dataptr = value.dataptr, .type = value.type};
}

static IntegerConfigPtr getIntegerPtr(HANDLE /*handle*/, const char* key) {
    const auto value = Config::mgr()->getConfigValue(key);
    return {.dataptr = value.dataptr, .type = value.type};
}

static void initOptionPointers(HANDLE handle, SOverridableConfig& level, const char* prefix) {
    for (uint32_t index = 0; index < GLASS_FLOAT_OPTIONS; ++index) {
        if (const auto option = optionKey(0, index))
            level.floats[index] = getStaticPtr<Hyprlang::FLOAT>(handle, (std::string(CONFIG_PREFIX) + prefix + option->key).c_str());
    }
    for (uint32_t index = 0; index < GLASS_INT_OPTIONS; ++index) {
        if (const auto option = optionKey(1, index))
            level.ints[index] = getStaticPtr<Hyprlang::INT>(handle, (std::string(CONFIG_PREFIX) + prefix + option->key).c_str());
    }
}

void initConfigPointers(HANDLE handle, SPluginConfig& config) {
    config.enabled           = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::ENABLED);
    config.manageWindowBlur  = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::MANAGE_WINDOW_BLUR);
    config.skipOpaqueWindows = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::SKIP_OPAQUE_WINDOWS);
    config.blurFold          = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::BLUR_FOLD);
    config.xpMode            = getIntegerPtr(handle, "render:xp_mode");
    config.defaultTheme  = getStringPtr(handle, ConfigKeys::DEFAULT_THEME);
    config.defaultPreset = getStringPtr(handle, ConfigKeys::DEFAULT_PRESET);

    config.debugMode   = getStringPtr(handle, ConfigKeys::DEBUG_MODE);
    config.debugTimers = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::DEBUG_TIMERS);

    config.layersEnabled           = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::LAYERS_ENABLED);
    config.layersNamespaces        = getStringPtr(handle, ConfigKeys::LAYERS_NAMESPACES);
    config.layersExcludeNamespaces = getStringPtr(handle, ConfigKeys::LAYERS_EXCLUDE_NAMESPACES);
    config.layersPreset            = getStringPtr(handle, ConfigKeys::LAYERS_PRESET);
    config.layersNamespacePresets         = getStringPtr(handle, ConfigKeys::LAYERS_NAMESPACE_PRESETS);
    config.layersMaskThreshold           = getStaticPtr<Hyprlang::FLOAT>(handle, ConfigKeys::LAYERS_MASK_THRESHOLD);
    config.layersNamespaceMaskThresholds = getStringPtr(handle, ConfigKeys::LAYERS_NAMESPACE_MASK_THRESHOLDS);
    config.layersNamespaceLiveResample = getStringPtr(handle, ConfigKeys::LAYERS_NAMESPACE_LIVE_RESAMPLE);
    config.layersLiveResample      = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::LAYERS_LIVE_RESAMPLE);
    config.layersLiveResampleFps   = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::LAYERS_LIVE_RESAMPLE_FPS);
    config.layersForceLiveResample = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::LAYERS_FORCE_LIVE_RESAMPLE);
    config.layersMaskMode           = getStringPtr(handle, ConfigKeys::LAYERS_MASK_MODE);
    config.layersNamespaceMaskModes = getStringPtr(handle, ConfigKeys::LAYERS_NAMESPACE_MASK_MODES);
    config.layersMaskFeather           = getStaticPtr<Hyprlang::FLOAT>(handle, ConfigKeys::LAYERS_MASK_FEATHER);
    config.layersNamespaceMaskFeathers = getStringPtr(handle, ConfigKeys::LAYERS_NAMESPACE_MASK_FEATHERS);
    config.layersManageBlur         = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::LAYERS_MANAGE_BLUR);

    config.subsurfacesEnabled = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::SUBSURFACES_ENABLED);
    config.subsurfacesPreset  = getStringPtr(handle, ConfigKeys::SUBSURFACES_PRESET);
    config.subsurfacesRadius  = getStaticPtr<Hyprlang::FLOAT>(handle, ConfigKeys::SUBSURFACES_RADIUS);

    config.windowsBackgroundCache = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::WINDOWS_BACKGROUND_CACHE);
    config.windowsLiveResample    = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::WINDOWS_LIVE_RESAMPLE);
    config.windowsLiveResampleFps = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::WINDOWS_LIVE_RESAMPLE_FPS);
    config.windowsBorder          = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::WINDOWS_BORDER);
    config.windowsBorderDim       = getStaticPtr<Hyprlang::FLOAT>(handle, ConfigKeys::WINDOWS_BORDER_DIM);
    config.motionEnabled          = getStaticPtr<Hyprlang::INT>(handle, ConfigKeys::MOTION_ENABLED);

    initOptionPointers(handle, config.global, "");
    initOptionPointers(handle, config.dark, "dark:");
    initOptionPointers(handle, config.light, "light:");
}

// ── Preset keyword parsing ───────────────────────────────────────────────────

// Presets built during config parse, swapped into g_pGlobalState on configReloaded
static Glass::CStore& pendingPresets() {
    static Glass::CStore pending;
    return pending;
}

Hyprlang::CParseResult handlePresetKeyword(const char* /*command*/, const char* value) {
    Hyprlang::CParseResult result;

    std::string error;
    if (!pendingPresets().addLine(value, error))
        result.setError(error.c_str());
    return result;
}

void clearPendingPresets() {
    pendingPresets().clear();
}

void commitPendingPresets() {
    if (!g_pGlobalState) return;

    // The built-in presets, and over them what the user defined (only what a user preset sets wins)
    g_pGlobalState->customPresets.commitFrom(pendingPresets());
    pendingPresets().clear();
}

// ── Lua config handler ──────────────────────────────────────────────────────
// hyprglass.config({...}) forwards to a single hl.config({ plugin = { hyprglass = ... } }).
// Keys are validated first so unknown options are reported with the user's file and line instead of '[C]'.
// Lua errors longjmp, so they are raised only while no C++ object lives in our frames.

static WP<Config::Lua::CConfigManager> luaConfigManager() {
    return dynamicPointerCast<Config::Lua::CConfigManager>(WP<Config::IConfigManager>(Config::mgr()));
}

struct SLuaConfigContext {
    lua_State*                                       L;
    WP<Config::Lua::CConfigManager>                  luaMgr;
    std::string                                      where;
    std::vector<std::pair<std::string, std::string>> applied; // {option path, dotted key}

    void report(const std::string& msg) const {
        if (!luaMgr) return;
        luaMgr->addError(where.empty() ? "hyprglass.config: " + msg : where + ": hyprglass.config: " + msg);
    }
};

// Copies the known options of the table at srcIdx into the table at dstIdx (absolute indices).
static void copyValidatedTable(SLuaConfigContext& ctx, int srcIdx, int dstIdx, const std::string& dottedPrefix, int depth = 0) {
    lua_State* L = ctx.L;
    // options are at most two levels deep; the cap also ends self-referencing tables
    if (depth > 4 || !lua_checkstack(L, 5)) {
        ctx.report(dottedPrefix.empty() ? std::string{"nesting too deep"} : std::format("nesting too deep in '{}'", dottedPrefix));
        return;
    }

    lua_pushnil(L);
    while (lua_next(L, srcIdx) != 0) {
        if (lua_type(L, -2) != LUA_TSTRING) {
            ctx.report(dottedPrefix.empty() ? std::string{"ignoring non-string key"} : std::format("ignoring non-string key in '{}'", dottedPrefix));
            lua_pop(L, 1);
            continue;
        }

        // same normalisation as Hyprland's luaConfigValueName
        std::string key = lua_tostring(L, -2);
        std::replace(key.begin(), key.end(), ':', '.');
        std::replace(key.begin(), key.end(), '-', '_');
        const std::string dotted = dottedPrefix.empty() ? key : dottedPrefix + "." + key;

        std::string optionPath = "plugin:hyprglass:" + dotted;
        std::replace(optionPath.begin(), optionPath.end(), '.', ':');
        const bool known = Config::mgr()->getConfigValue(optionPath).dataptr != nullptr;
        const bool table = lua_istable(L, -1);

        if (!known) {
            if (table) {
                lua_newtable(L);
                const int subDst = lua_gettop(L);
                copyValidatedTable(ctx, subDst - 1, subDst, dotted, depth + 1);
                lua_setfield(L, dstIdx, key.c_str());
            } else {
                ctx.report(std::format("unknown option '{}'", dotted));
            }
        } else if (table) {
            ctx.report(std::format("option '{}' expects a value, not a table", dotted));
        } else {
            lua_pushvalue(L, -1);
            lua_setfield(L, dstIdx, key.c_str());
            ctx.applied.emplace_back(std::move(optionPath), dotted);
        }
        lua_pop(L, 1);
    }
}

// Stack on entry: [T, ..., hl, config]. Returns true with [T, ..., hl], or false with the error message on top.
static bool forwardLuaConfig(lua_State* L) {
    SLuaConfigContext ctx{.L = L, .luaMgr = luaConfigManager()};

    lua_Debug ar{};
    if (lua_getstack(L, 1, &ar) && lua_getinfo(L, "Sl", &ar) && ar.currentline > 0) {
        const char* src = ar.source;
        if (src && *src == '@')
            ++src;
        ctx.where = std::format("{}:{}", src ? src : "?", ar.currentline);
    }

    lua_newtable(L); // root
    lua_newtable(L); // plugin
    lua_newtable(L); // hyprglass
    copyValidatedTable(ctx, 1, lua_gettop(L), "");
    lua_setfield(L, -2, "hyprglass");
    lua_setfield(L, -2, "plugin");

    const std::string errorsBefore = ctx.luaMgr ? ctx.luaMgr->getErrors() : std::string{};
    if (lua_pcall(L, 1, 0, 0) != LUA_OK)
        return false;

    // hl.config reports type and range errors itself; only look for silent no-ops while the config file is parsed.
    if (ctx.luaMgr && !ctx.luaMgr->isDynamicParse() && ctx.luaMgr->getErrors() == errorsBefore) {
        for (const auto& [optionPath, dotted] : ctx.applied) {
            if (!Config::mgr()->getConfigValue(optionPath).setByUser)
                ctx.report(std::format("option '{}' was not applied", dotted));
        }
    }
    return true;
}

static int handleLuaConfig(lua_State* L) {
    if (lua_gettop(L) < 1 || !lua_istable(L, 1))
        return luaL_error(L, "hyprglass.config: expected a table");

    lua_getglobal(L, "hl");
    if (!lua_istable(L, -1))
        return luaL_error(L, "hyprglass.config: hl.config is not available");
    lua_getfield(L, -1, "config");
    if (!lua_isfunction(L, -1))
        return luaL_error(L, "hyprglass.config: hl.config is not available");

    if (!forwardLuaConfig(L))
        return lua_error(L);
    lua_pop(L, 1); // hl
    return 0;
}

// hyprglass.motion({ enabled = true }) is hyprglass.config({ motion = { ... } }):
// a function of its own, so a config can ask the engine it runs whether it has
// the motion at all (`if hg.motion then`), where an unknown key is an error.
static int handleLuaMotion(lua_State* L) {
    if (lua_gettop(L) < 1 || !lua_istable(L, 1))
        return luaL_error(L, "hyprglass.motion: expected a table");
    lua_settop(L, 1);
    lua_newtable(L);
    lua_insert(L, 1);
    lua_setfield(L, 1, "motion");
    return handleLuaConfig(L);
}

// ── Lua preset handler (table + string) ─────────────────────────────────────

// Booleans read as 0/1, so a flag like xray takes `true` as hyprglass.config() does.
static std::optional<std::string> luaPresetValue(lua_State* L, int index) {
    if (lua_isboolean(L, index))
        return lua_toboolean(L, index) ? "1" : "0";
    if (lua_isnumber(L, index))
        return std::to_string(lua_tonumber(L, index));
    return std::nullopt;
}

static void readPresetValuesFromTable(lua_State* L, int tableIdx, std::string_view preset, Glass::EVariant variant) {
    lua_pushnil(L);
    while (lua_next(L, tableIdx) != 0) {
        if (lua_isstring(L, -2)) {
            if (const auto value = luaPresetValue(L, -1))
                (void)pendingPresets().setField(preset, variant, lua_tostring(L, -2), *value);
        }
        lua_pop(L, 1);
    }
}

static int handleLuaPreset(lua_State* L) {
    int nargs = lua_gettop(L);

    // Legacy: preset("name:clear, glass_opacity:0.8, ...")
    if (nargs == 1 && lua_isstring(L, 1)) {
        const auto result = handlePresetKeyword(ConfigKeys::PRESET_KEYWORD, lua_tostring(L, 1));
        if (result.error)
            return luaL_error(L, "%s", result.getError());
        return 0;
    }

    // Table: preset("clear", { glass_opacity = 0.8, dark = { brightness = 0.7 }, ... })
    if (nargs == 2 && lua_isstring(L, 1) && lua_istable(L, 2)) {
        const std::string baseName(Glass::stripDynamicTagMarker(lua_tostring(L, 1)));
        auto&             pending = pendingPresets();
        pending.define(baseName);

        lua_pushnil(L);
        while (lua_next(L, 2) != 0) {
            if (!lua_isstring(L, -2)) { lua_pop(L, 1); continue; }
            const char* key = lua_tostring(L, -2);

            if (strcmp(key, "inherits") == 0 && lua_isstring(L, -1)) {
                pending.setInherits(baseName, lua_tostring(L, -1));
            } else if (strcmp(key, "dark") == 0 && lua_istable(L, -1)) {
                readPresetValuesFromTable(L, lua_gettop(L), baseName, Glass::EVariant::DARK);
            } else if (strcmp(key, "light") == 0 && lua_istable(L, -1)) {
                readPresetValuesFromTable(L, lua_gettop(L), baseName, Glass::EVariant::LIGHT);
            } else if (const auto value = luaPresetValue(L, -1)) {
                (void)pending.setField(baseName, Glass::EVariant::SHARED, key, *value);
            }
            lua_pop(L, 1);
        }
        return 0;
    }

    return luaL_error(L, "hyprglass.preset: expected (string) or (name, table)");
}

// ── Lua layer handler ───────────────────────────────────────────────────────

struct SPendingLayer {
    std::string                   ns;
    std::string                   preset;
    float                         maskThreshold = -1.0f;
    float                         maskFeather   = -1.0f;
    bool                          exclude       = false;
    int                           liveResample  = -1; // -1 = not set
    std::optional<ELayerMaskMode> maskMode;
};

static std::vector<SPendingLayer> s_pendingLayers;

static int handleLuaLayer(lua_State* L) {
    if (lua_gettop(L) < 1 || !lua_isstring(L, 1))
        return luaL_error(L, "hyprglass.layer: first argument must be a namespace string");

    SPendingLayer entry;
    entry.ns = lua_tostring(L, 1);

    if (lua_gettop(L) >= 2 && lua_istable(L, 2)) {
        lua_getfield(L, 2, "exclude");
        if (lua_isboolean(L, -1) && lua_toboolean(L, -1))
            entry.exclude = true;
        lua_pop(L, 1);

        lua_getfield(L, 2, "preset");
        if (lua_isstring(L, -1))
            entry.preset = lua_tostring(L, -1);
        lua_pop(L, 1);

        lua_getfield(L, 2, "mask_threshold");
        if (lua_isnumber(L, -1))
            entry.maskThreshold = static_cast<float>(lua_tonumber(L, -1));
        lua_pop(L, 1);

        lua_getfield(L, 2, "mask_feather");
        if (lua_isnumber(L, -1))
            entry.maskFeather = static_cast<float>(lua_tonumber(L, -1));
        lua_pop(L, 1);

        lua_getfield(L, 2, "live_resample");
        if (lua_isboolean(L, -1))
            entry.liveResample = lua_toboolean(L, -1) ? 1 : 0;
        lua_pop(L, 1);

        lua_getfield(L, 2, "mask_mode");
        if (lua_isstring(L, -1))
            entry.maskMode = parseLayerMaskMode(lua_tostring(L, -1));
        lua_pop(L, 1);
    }

    s_pendingLayers.push_back(std::move(entry));
    return 0;
}

void clearPendingLayers() {
    s_pendingLayers.clear();
}

void commitPendingLayers() {
    if (!g_pGlobalState) return;
    for (const auto& entry : s_pendingLayers) {
        if (entry.exclude) {
            g_pGlobalState->layerNamespaceExclude.insert(entry.ns);
        } else {
            g_pGlobalState->layerNamespaceFilter.insert(entry.ns);
            if (!entry.preset.empty())
                g_pGlobalState->layerNamespacePresets[entry.ns] = entry.preset;
            if (entry.maskThreshold >= 0.0f)
                g_pGlobalState->layerNamespaceMaskThresholds[entry.ns] = entry.maskThreshold;
            if (entry.maskFeather >= 0.0f)
                g_pGlobalState->layerNamespaceMaskFeathers[entry.ns] = entry.maskFeather;
            if (entry.liveResample >= 0)
                g_pGlobalState->layerNamespaceLiveResample[entry.ns] = entry.liveResample != 0;
            if (entry.maskMode)
                g_pGlobalState->layerNamespaceMaskModes[entry.ns] = *entry.maskMode;
        }
    }
    s_pendingLayers.clear();
}

void validateConfig() {
    if (!g_pGlobalState) return;

    const auto& config = g_pGlobalState->config;

    const auto theme = readStringConfig(config.defaultTheme);
    if (theme != "dark" && theme != "light") {
        HyprlandAPI::addNotificationV2(PHANDLE, {
            {"text", std::string("[hyprglass] Invalid default_theme '") + std::string(theme) + "', expected 'dark' or 'light'. Falling back to 'dark'."},
            {"time", (uint64_t)5000},
            {"color", CHyprColor{1.0, 0.8, 0.2, 1.0}},
        });
    }

    const auto maskMode = readStringConfig(config.layersMaskMode);
    if (!parseLayerMaskMode(maskMode)) {
        HyprlandAPI::addNotificationV2(PHANDLE, {
            {"text", std::string("[hyprglass] Invalid layers:mask_mode '") + std::string(maskMode) + "', expected 'auto', 'alpha', or 'region'. Falling back to 'auto'."},
            {"time", (uint64_t)5000},
            {"color", CHyprColor{1.0, 0.8, 0.2, 1.0}},
        });
    }

    const auto debugMode = readStringConfig(config.debugMode);
    if (!parseDebugMode(debugMode)) {
        HyprlandAPI::addNotificationV2(PHANDLE, {
            {"text", std::string("[hyprglass] Invalid debug:mode '") + std::string(debugMode) + "', expected 'off', 'hints_only', or 'gl_work_only'. Falling back to 'off'."},
            {"time", (uint64_t)5000},
            {"color", CHyprColor{1.0, 0.8, 0.2, 1.0}},
        });
    }

    const auto preset = readStringConfig(config.defaultPreset);
    if (!preset.empty() && preset != "default") {
        if (!g_pGlobalState->customPresets.contains(preset)) {
            HyprlandAPI::addNotificationV2(PHANDLE, {
                {"text", std::string("[hyprglass] Unknown default_preset '") + std::string(preset) + "'. Using 'default' resolution chain."},
                {"time", (uint64_t)5000},
                {"color", CHyprColor{1.0, 0.8, 0.2, 1.0}},
            });
        }
    }
}

bool anySelfSampleConfigured(const SPluginConfig& config, const Glass::CStore& customPresets) {
    for (const auto* layer : {&config.global, &config.dark, &config.light}) {
        if (auto ptr = layer->floats[GLASS_F_SELF_SAMPLE]; ptr && *ptr && **ptr > 0.0)
            return true;
    }

    return customPresets.anyPositive(GLASS_F_SELF_SAMPLE);
}
