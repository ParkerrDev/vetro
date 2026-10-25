#include "GlassBridge.hpp"

#include <unordered_map>

namespace Glass {

CStore::CStore() : m_memory(std::make_unique<uint8_t[]>(GLASS_STORE_CAPACITY)) {
    m_store = GlassStore{m_memory.get(), GLASS_STORE_CAPACITY, 0};
    clear();
}

void CStore::clear() {
    (void)glass_store_clear(&m_store);
}

bool CStore::addLine(std::string_view line, std::string& error) {
    GlassText message{};
    const int code = glass_store_add_line(&m_store, bytes(line), &message);
    if (code == GLASS_OK)
        return true;
    error = code == GLASS_ERR_PARSE ? std::string(reinterpret_cast<const char*>(message.bytes), message.len) : "preset: out of room for another preset";
    return false;
}

bool CStore::setField(std::string_view name, EVariant variant, std::string_view key, std::string_view value) {
    const GlassFieldEdit edit{bytes(name), static_cast<uint32_t>(variant), bytes(key), bytes(value)};
    return glass_store_set_field(&m_store, &edit) == 1;
}

void CStore::setInherits(std::string_view name, std::string_view inherits) {
    (void)glass_store_set_inherits(&m_store, bytes(name), bytes(inherits));
}

void CStore::define(std::string_view name) {
    (void)glass_store_define(&m_store, bytes(name));
}

void CStore::commitFrom(const CStore& user) {
    (void)glass_store_commit(user.raw(), &m_store);
}

bool CStore::contains(std::string_view name) const {
    return glass_store_contains(&m_store, bytes(name)) == 1;
}

bool CStore::anyPositive(uint32_t floatOption) const {
    return glass_store_any_positive(&m_store, floatOption) == 1;
}

const std::string& shaderSource(const std::string& name) {
    static const std::unordered_map<std::string, std::string> sources = [] {
        std::unordered_map<std::string, std::string> all;
        const auto                                   text = [](auto fn, uint32_t index) {
            size_t length = 0;
            if (fn(index, nullptr, 0, &length) != GLASS_OK)
                return std::string();
            std::string out(length, '\0');
            if (fn(index, reinterpret_cast<uint8_t*>(out.data()), out.size(), &length) != GLASS_OK)
                return std::string();
            return out;
        };
        for (uint32_t i = 0; i < glass_shader_count(); ++i)
            all.emplace(text(glass_shader_name, i), text(glass_shader_source, i));
        return all;
    }();
    static const std::string none;
    const auto               found = sources.find(name);
    return found == sources.end() ? none : found->second;
}

std::string_view stripDynamicTagMarker(std::string_view tag) {
    size_t length = tag.size();
    if (glass_strip_dynamic_tag_marker(bytes(tag), &length) != GLASS_OK)
        return tag;
    return tag.substr(0, length);
}

} // namespace Glass
