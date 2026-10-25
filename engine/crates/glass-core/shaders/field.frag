#version 300 es
precision highp float;

uniform sampler2D tex;        // blurred coverage, 16 bits in r and g
uniform ivec2 lowTexel;
uniform ivec2 highTexel;

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

float value(ivec2 at) {
    vec4 c = texelFetch(tex, clamp(at, lowTexel, highTexel), 0);
    return (c.r * 65280.0 + c.g * 255.0) / 65535.0;
}

void main() {
    ivec2 at = ivec2(gl_FragCoord.xy);
    float v  = value(at);
    vec2  g  = vec2(value(at + ivec2(1, 0)) - value(at - ivec2(1, 0)), value(at + ivec2(0, 1)) - value(at - ivec2(0, 1)));
    float l  = length(g);
    vec2  n  = l > 1e-5 ? g / l : vec2(0.0);
    fragColor = vec4(n * 0.5 + 0.5, v, 1.0);
}
