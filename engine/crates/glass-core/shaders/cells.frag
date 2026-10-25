#version 300 es
precision highp float;

uniform sampler2D tex;        // the field
uniform ivec2 lowTexel;
uniform ivec2 highTexel;
uniform int cellSize;

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

void main() {
    ivec2 cell  = ivec2(gl_FragCoord.xy) * cellSize;
    float most  = 0.0;
    float least = 1.0;
    for (int j = -1; j <= cellSize; j++) {
        for (int i = -1; i <= cellSize; i++) {
            ivec2 at = cell + ivec2(i, j);
            if (at.x < lowTexel.x || at.y < lowTexel.y || at.x > highTexel.x || at.y > highTexel.y)
                continue;
            float v = texelFetch(tex, at, 0).b;
            most    = max(most, v);
            least   = min(least, v);
        }
    }
    fragColor = vec4(most, least, 0.0, 1.0);
}
