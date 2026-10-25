#version 300 es
precision highp float;

uniform sampler2D tex;        // the frame so far
uniform vec2 sourceOrigin;    // top left of the rectangle read, in source pixels
uniform vec2 invSourceSize;   // 1 / the source's size in pixels
uniform int scale;            // source pixels per output texel: 2, 4 or 8
uniform float whiteWeight;    // how many times more white weighs than black in the mean; 0 for a plain mean

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

const vec3 LUMA = @LUMA@;

void main() {
    vec2 corner = sourceOrigin + floor(gl_FragCoord.xy) * float(scale);
    int  groups = scale / 2;
    vec4 sum    = vec4(0.0);
    for (int j = 0; j < groups; j++) {
        for (int i = 0; i < groups; i++) {
            vec4 texel = texture(tex, (corner + vec2(float(2 * i + 1), float(2 * j + 1))) * invSourceSize);
            // A weighted mean survives the blurs that follow when each texel carries its
            // weight: colour times weight, and the weight itself in alpha, which the last
            // of them divides by (frostblur.frag). The weight is kept to 1 at most.
            if (whiteWeight > 0.0) {
                vec3  color  = clamp(texel.rgb, 0.0, 1.0);
                float weight = mix(1.0, whiteWeight, dot(color, LUMA)) / max(1.0, whiteWeight);
                texel = vec4(color * weight, weight);
            }
            sum += texel;
        }
    }
    // clamped: an out-of-range texel of a float framebuffer would dominate everything after
    fragColor = clamp(sum / float(groups * groups), 0.0, 1.0);
}
