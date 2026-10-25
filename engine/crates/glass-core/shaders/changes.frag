#version 300 es
precision highp float;

// Where the outline moved, coarsely: the question (outline.frag comparing, over
// a target cleared first) writes only the texels that differ, so a block of
// them is marked here when any of its texels was written. Read back, the marks
// bound the part of the field that has to be built again.

uniform sampler2D tex;        // the question's output: alpha 1 where the outline differs, 0 elsewhere
uniform ivec2 highTexel;      // the last texel of it that holds data
uniform int blockSize;        // its texels a side per texel here

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

void main() {
    ivec2 origin = ivec2(gl_FragCoord.xy) * blockSize;
    float moved  = 0.0;
    for (int j = 0; j < blockSize; j++) {
        for (int i = 0; i < blockSize; i++) {
            ivec2 at = origin + ivec2(i, j);
            if (at.x > highTexel.x || at.y > highTexel.y)
                continue;
            moved = max(moved, texelFetch(tex, at, 0).a);
        }
    }
    fragColor = vec4(moved, 0.0, 0.0, 1.0);
}
