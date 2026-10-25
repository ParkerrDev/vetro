#include "Tuning.hpp"
#include "Globals.hpp"
#include "Motion.hpp"

#include <cstdlib>
#include <format>
#include <fstream>
#include <iterator>
#include <optional>
#include <hyprland/src/render/Renderer.hpp>
#include <hyprland/src/state/MonitorState.hpp>

namespace {
    // quartz::Kind::Tuned's code across the C boundary
    constexpr uint32_t TUNED_KIND = 6;
    // a tuning is a few dozen short lines; anything longer is not one
    constexpr std::streamsize MAX_FILE = 64 * 1024;
    // settings given on the command line at once
    constexpr size_t MAX_SETTINGS = 64;

    std::string configPath() {
        if (const char* xdg = std::getenv("XDG_CONFIG_HOME"); xdg && *xdg)
            return std::string(xdg) + "/vetro/glass.conf";
        if (const char* home = std::getenv("HOME"); home && *home)
            return std::string(home) + "/.config/vetro/glass.conf";
        return {};
    }

    // The file's text, at most MAX_FILE bytes of it; none when it cannot be read.
    std::optional<std::string> readFile(const std::string& path) {
        std::ifstream file(path, std::ios::binary);
        if (!file)
            return std::nullopt;
        std::string text(static_cast<size_t>(MAX_FILE), '\0');
        file.read(text.data(), MAX_FILE);
        text.resize(static_cast<size_t>(file.gcount()));
        return text;
    }

    // Every piece of glass is drawn again with the new numbers: its page read
    // afresh (the frost may have changed size) and its monitor repainted.
    void redrawGlass() {
        for (const auto& monitor : State::monitorState()->monitors()) {
            if (!monitor)
                continue;
            g_pGlobalState->bumpSceneGeneration(monitor);
            g_pHyprRenderer->damageMonitor(monitor);
        }
    }

    // Reads `text` over the defaults into the current tuning; answers how many
    // lines were skipped, or -1 when it could not be read at all.
    int adopt(const std::string& text) {
        GlassTuning parsed{};
        const int   skipped = glass_tuning_parse(GlassBytes{reinterpret_cast<const uint8_t*>(text.data()), text.size()}, &parsed);
        if (skipped < 0)
            return -1;
        g_pGlobalState->tuning = parsed;
        redrawGlass();
        Motion::applyPace();
        return skipped;
    }

    std::string currentText() {
        size_t length = 0;
        if (glass_tuning_format(&g_pGlobalState->tuning, nullptr, 0, &length) < 0)
            return {};
        std::string text(length, '\0');
        if (glass_tuning_format(&g_pGlobalState->tuning, reinterpret_cast<uint8_t*>(text.data()), text.size(), &length) < 0)
            return {};
        return text;
    }

    std::string describe() {
        const auto& state = *g_pGlobalState;
        const std::string from = state.tuningFromFile ? state.tuningPath : std::format("defaults ({} not found)", state.tuningPath);
        std::string out = std::format("hyprglass tune: {}{}\n", from,
                                      state.tuningSkipped > 0 ? std::format(", {} line(s) skipped", state.tuningSkipped) : std::string{});
        return out + currentText();
    }

    std::string_view trimmed(std::string_view text) {
        while (!text.empty() && (text.front() == ' ' || text.front() == '\t'))
            text.remove_prefix(1);
        while (!text.empty() && (text.back() == ' ' || text.back() == '\t' || text.back() == '\n'))
            text.remove_suffix(1);
        return text;
    }
} // namespace

namespace Tuning {

    void load() {
        if (!g_pGlobalState)
            return;
        const auto path = configPath();
        const auto text = path.empty() ? std::nullopt : readFile(path);
        g_pGlobalState->tuningPath     = path;
        g_pGlobalState->tuningFromFile = text.has_value();
        g_pGlobalState->tuningSkipped  = adopt(text.value_or(std::string{}));
    }

    std::string command(std::string_view arguments) {
        if (!g_pGlobalState)
            return "hyprglass tune: not loaded\n";
        arguments = trimmed(arguments);
        if (arguments.empty() || arguments == "show")
            return describe();
        if (arguments == "reload") {
            load();
            return describe();
        }

        // settings over the current tuning: later lines win when it is read again
        std::string text = currentText();
        size_t      count = 0;
        while (!arguments.empty() && count < MAX_SETTINGS) {
            const size_t      space   = arguments.find(' ');
            const std::string_view setting = arguments.substr(0, space);
            text += std::string(setting) + "\n";
            arguments = space == std::string_view::npos ? std::string_view{} : trimmed(arguments.substr(space + 1));
            ++count;
        }
        const int skipped = adopt(text);
        return skipped > 0 ? std::format("hyprglass tune: {} setting(s) not understood\n", skipped) + describe() : describe();
    }

    bool quartz(uint32_t code, GlassQuartz& out) {
        if (code == TUNED_KIND && g_pGlobalState)
            return glass_quartz_tuned(&g_pGlobalState->tuning, &out) >= 0;
        return glass_quartz(code, &out) >= 0;
    }

    bool seeWindows() {
        return g_pGlobalState && g_pGlobalState->tuning.values[GLASS_T_SEE_WINDOWS] >= 0.5f;
    }

} // namespace Tuning
