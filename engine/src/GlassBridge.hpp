#pragma once

// The C++ side of the glass model's C interface (crates/glass-ffi, glass_ffi.h).
// Thin wrappers that own the memory the interface asks the caller for and turn
// its integer answers into the types the plugin works with. The model itself,
// every number it decides and every rule it applies, is in Rust.

#include <glass_ffi.h>

#include <hyprutils/math/Box.hpp>

#include <cstdint>
#include <memory>
#include <string>
#include <string_view>
#include <vector>

namespace Glass {

using Hyprutils::Math::CBox;

[[nodiscard]] inline GlassRect toRect(const CBox& box) noexcept {
    return GlassRect{box.x, box.y, box.w, box.h};
}

[[nodiscard]] inline CBox toBox(const GlassRect& rect) noexcept {
    return CBox{rect.x, rect.y, rect.w, rect.h};
}

[[nodiscard]] inline GlassBytes bytes(std::string_view text) noexcept {
    return GlassBytes{reinterpret_cast<const uint8_t*>(text.data()), text.size()};
}

// True when the library linked in is the one the header was written for.
[[nodiscard]] inline bool abiMatches() noexcept {
    return glass_abi_version() == GLASS_ABI_VERSION;
}

// Which layer of a preset a setting goes to (the numbers glass_store_set_field takes).
enum class EVariant : uint32_t { SHARED = 0, DARK = 1, LIGHT = 2 };

// A block of presets in memory this side owns, which the library rewrites in place.
class CStore {
  public:
    CStore();
    CStore(CStore&&) noexcept            = default;
    CStore& operator=(CStore&&) noexcept = default;
    CStore(const CStore&)                = delete;
    CStore& operator=(const CStore&)     = delete;

    void clear();
    // Adds a line of the legacy form. False for a line that does not read; error says why.
    [[nodiscard]] bool addLine(std::string_view line, std::string& error);
    // One setting of a layer of a preset, from its text. False for a key or value that is not one.
    bool setField(std::string_view name, EVariant variant, std::string_view key, std::string_view value);
    void setInherits(std::string_view name, std::string_view inherits);
    void define(std::string_view name);
    // Becomes the built-in presets, changed by those of user.
    void commitFrom(const CStore& user);
    [[nodiscard]] bool contains(std::string_view name) const;
    // True when any preset sets the float option (a GLASS_F_ constant) to more than zero.
    [[nodiscard]] bool anyPositive(uint32_t floatOption) const;
    [[nodiscard]] const GlassStore* raw() const noexcept { return &m_store; }

  private:
    std::unique_ptr<uint8_t[]> m_memory;
    GlassStore                 m_store{};
};

// The source of the fragment shader called name, with the model's constants put in; empty if there is none.
[[nodiscard]] const std::string& shaderSource(const std::string& name);

// Without a trailing `*`, the way Hyprland stores dynamic tags.
[[nodiscard]] std::string_view stripDynamicTagMarker(std::string_view tag);

} // namespace Glass
