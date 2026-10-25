// Writes every fragment shader the plugin carries to a file of its own, so that
// glslangValidator can compile them where there is no display. The model's
// shaders come out of the Rust library the plugin links, the classic ones out
// of the plugin's own sources.
#include "../src/Shaders.hpp"

#include <glass_ffi.h>

#include <cstdint>
#include <cstdio>
#include <fstream>
#include <string>

namespace {

// Asks for the length of the text, then for the text, the way the plugin does.
std::string textOf(int32_t (*ask)(uint32_t, uint8_t*, size_t, size_t*), uint32_t index) {
    size_t length = 0;
    if (ask(index, nullptr, 0, &length) != GLASS_OK)
        return {};
    std::string text(length, '\0');
    if (ask(index, reinterpret_cast<uint8_t*>(text.data()), text.size(), &length) != GLASS_OK)
        return {};
    return text;
}

bool write(const std::string& directory, const std::string& name, const std::string& text) {
    std::ofstream out(directory + "/" + name);
    out << text;
    if (!out) {
        std::fprintf(stderr, "cannot write %s\n", name.c_str());
        return false;
    }
    return true;
}

} // namespace

int main(int argc, char** argv) {
    if (argc != 2) {
        std::fprintf(stderr, "usage: dump_shaders DIRECTORY\n");
        return 2;
    }

    int written = 0;
    for (uint32_t i = 0; i < glass_shader_count(); ++i) {
        if (!write(argv[1], textOf(glass_shader_name, i), textOf(glass_shader_source, i)))
            return 1;
        ++written;
    }
    for (const auto& [name, source] : SHADERS) {
        // The classic shaders open their raw string on the line before #version.
        std::string text  = source;
        const auto  first = text.find_first_not_of("\r\n");
        if (!write(argv[1], name, first == std::string::npos ? text : text.substr(first)))
            return 1;
        ++written;
    }
    std::printf("%d shaders written\n", written);
    return 0;
}
