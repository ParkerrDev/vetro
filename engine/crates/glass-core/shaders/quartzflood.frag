#version 300 es
precision highp float;

// One pass of the jump flood: each texel keeps whichever point of the outline,
// among those its neighbours a step away name, is nearest to it. Run with steps
// halving from half the field's size to one, every texel ends up naming the
// nearest point of the outline, give or take a texel.

uniform sampler2D tex;        // the last pass: packed coordinates of a point of the outline
uniform ivec2 lowTexel;       // the first and last texel that hold data (all of the
uniform ivec2 highTexel;      // field, or the part of it that is built again)
uniform int stepSize;

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

const float NONE = 65535.0;

vec2 unpack(vec4 c) {
    vec4 b = floor(c * 255.0 + 0.5);
    return vec2(b.x * 256.0 + b.y, b.z * 256.0 + b.w);
}

vec4 pack(vec2 at) {
    vec2 hi = floor(at / 256.0);
    vec2 lo = at - hi * 256.0;
    return vec4(hi.x, lo.x, hi.y, lo.y) / 255.0;
}

void main() {
    ivec2 at   = ivec2(gl_FragCoord.xy);
    vec2  here = vec2(at);
    vec2  best = vec2(NONE);
    float bestDistance = 1.0e20;
    for (int j = -1; j <= 1; j++) {
        for (int i = -1; i <= 1; i++) {
            ivec2 from = at + ivec2(i, j) * stepSize;
            if (from.x < lowTexel.x || from.y < lowTexel.y || from.x > highTexel.x || from.y > highTexel.y)
                continue;
            vec2 seed = unpack(texelFetch(tex, from, 0));
            if (seed.x >= NONE)
                continue;
            vec2  d = seed - here;
            float distance = dot(d, d);
            if (distance < bestDistance) {
                bestDistance = distance;
                best = seed;
            }
        }
    }
    fragColor = pack(best);
}
