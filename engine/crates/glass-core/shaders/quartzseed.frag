#version 300 es
precision highp float;

// The first pass of the outline's distance field for Apple's shader: every
// texel of pane that has a texel beside it that is not pane is a point of the
// outline, and names itself; every other texel names no one. Coordinates are
// packed exactly into the four bytes, 16 bits each.

uniform sampler2D tex;        // coverage: the share of each texel that is pane, in r
uniform ivec2 lowTexel;       // the first and last texel that hold data: all of the
uniform ivec2 highTexel;      // field, or the part of it that is built again

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

const float NONE = 65535.0;

bool pane(ivec2 at) {
    return texelFetch(tex, clamp(at, lowTexel, highTexel), 0).r >= 0.5;
}

vec4 pack(vec2 at) {
    vec2 hi = floor(at / 256.0);
    vec2 lo = at - hi * 256.0;
    return vec4(hi.x, lo.x, hi.y, lo.y) / 255.0;
}

void main() {
    ivec2 at = ivec2(gl_FragCoord.xy);
    bool  in0 = pane(at);
    bool  edge = in0 && (!pane(at + ivec2(1, 0)) || !pane(at - ivec2(1, 0)) || !pane(at + ivec2(0, 1)) || !pane(at - ivec2(0, 1)));
    fragColor = edge ? pack(vec2(at)) : pack(vec2(NONE));
}
