#!/bin/sh
# Compiles every fragment shader of the plugin with glslangValidator. Needs no
# display and no GPU: it checks the GLSL ES 3.00 grammar and types, which is
# what stops a shader from loading in the compositor.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

command -v glslangValidator >/dev/null 2>&1 || { echo "shaders: glslangValidator is not installed (package glslang), skipped"; exit 0; }

: "${GLASS_LIB:?the path of libglass_ffi.a, which make test builds}"
"${CXX:-c++}" -std=c++23 -O1 -I"$here/../crates/glass-ffi/include" "$here/dump_shaders.cpp" "$GLASS_LIB" -lpthread -ldl -lm -o "$out/dump"
"$out/dump" "$out"

failed=0
for shader in "$out"/*.frag; do
    if glslangValidator -S frag "$shader" > "$out/log" 2>&1; then
        echo "ok   $(basename "$shader")"
    else
        echo "FAIL $(basename "$shader")"
        cat "$out/log"
        failed=1
    fi
done
exit "$failed"
