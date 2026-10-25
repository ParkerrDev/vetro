#version 300 es
precision highp float;

uniform sampler2D tex;
uniform ivec2 direction;      // (1, 0) or (0, 1)
uniform ivec2 lowTexel;       // first and last texel that hold data, inclusive
uniform ivec2 highTexel;
uniform float sigma;          // texels
uniform int radius;           // texels
uniform int packedInput;      // 1 when tex holds 16 bits in r and g

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

float value(ivec2 at) {
    vec4 c = texelFetch(tex, clamp(at, lowTexel, highTexel), 0);
    return packedInput == 1 ? (c.r * 65280.0 + c.g * 255.0) / 65535.0 : c.r;
}

void main() {
    ivec2 at        = ivec2(gl_FragCoord.xy);
    float inverse   = -0.5 / (sigma * sigma);
    float sum       = value(at);
    float weightSum = 1.0;
    for (int i = 1; i <= radius; i++) {
        float w = exp(float(i * i) * inverse);
        sum += w * (value(at + direction * i) + value(at - direction * i));
        weightSum += 2.0 * w;
    }
    float v    = clamp(sum / weightSum, 0.0, 1.0) * 65535.0;
    float high = floor(v / 256.0);
    fragColor  = vec4(high / 255.0, (v - high * 256.0) / 255.0, 0.0, 1.0);
}
