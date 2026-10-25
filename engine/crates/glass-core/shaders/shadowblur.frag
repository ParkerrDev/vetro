#version 300 es
precision highp float;

uniform sampler2D tex;
uniform vec2 texelStep;       // one source texel along the blur axis, in uv
uniform vec2 uvLow;           // centre of the first and last texel that hold data
uniform vec2 uvHigh;
uniform vec2 uvScale;         // v_texcoord to source uv
uniform vec2 uvOffset;
uniform float sigma;          // source texels
uniform int radius;

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

void main() {
    vec2  uv        = v_texcoord * uvScale + uvOffset;
    float inverse   = -0.5 / (sigma * sigma);
    float sum       = texture(tex, clamp(uv, uvLow, uvHigh)).r;
    float weightSum = 1.0;
    for (int i = 1; i <= radius; i++) {
        float w = exp(float(i * i) * inverse);
        sum += w * (texture(tex, clamp(uv + texelStep * float(i), uvLow, uvHigh)).r + texture(tex, clamp(uv - texelStep * float(i), uvLow, uvHigh)).r);
        weightSum += 2.0 * w;
    }
    float s   = sum / weightSum;
    fragColor = vec4(s, s, s, 1.0);
}
