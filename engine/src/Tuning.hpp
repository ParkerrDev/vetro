#pragma once

#include "glass_ffi.h"

#include <cstdint>
#include <string>
#include <string_view>

// The glass as its user tuned it: ~/.config/vetro/glass.conf, which the tuner
// (`vetro tune`) writes. It is read when the plugin loads, on every config
// reload, and when `hyprctl hyprglass tune reload` asks; settings given to
// `hyprctl hyprglass tune key=value ...` apply at once over what is there and
// last until the file is read again. The model (glass_core::tuning) holds every
// value to its range and turns them into the block of Apple's shader.
namespace Tuning {

    // Reads the file, or takes the defaults where there is none, and has every
    // piece of glass drawn again with it.
    void load();

    // `hyprctl hyprglass tune [show | reload | key=value ...]`: the answer to print.
    [[nodiscard]] std::string command(std::string_view arguments);

    // Apple's shader's block for a preset's kind: the user's tuning for the tuned
    // kind, the kind's own numbers for the others. False for no kind.
    [[nodiscard]] bool quartz(uint32_t code, GlassQuartz& out);

    // Whether windows show through glass laid over them (the tuning's see_windows).
    [[nodiscard]] bool seeWindows();

} // namespace Tuning
