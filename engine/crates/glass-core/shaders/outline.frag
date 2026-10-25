#version 300 es
precision highp float;

uniform sampler2D tex;        // the layer's surface, premultiplied
uniform sampler2D previous;   // this pass's output at the last field build
uniform vec2 invPreviousSize; // 1 / texels of that texture
uniform vec2 boxSize;         // layer box, device pixels
uniform vec2 margin;          // device pixels of margin left and above the box
uniform vec2 maskUvScale;     // box uv to surface texture uv
uniform float threshold;      // alpha from which a pixel is pane
uniform int scale;            // device pixels per texel
uniform int compare;          // 1: keep only the texels that differ from previous

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

void main() {
    vec2  p      = (floor(gl_FragCoord.xy) + 0.5) * float(scale) - margin;
    float inside = 0.0;
    if (p.x >= 0.0 && p.y >= 0.0 && p.x < boxSize.x && p.y < boxSize.y)
        inside = step(threshold, texture(tex, p / boxSize * maskUvScale).a);
    if (compare == 1 && abs(texture(previous, gl_FragCoord.xy * invPreviousSize).r - inside) < 0.5)
        discard;
    fragColor = vec4(inside, inside, inside, 1.0);
}
