#version 300 es
precision highp float;

uniform sampler2D tex;
uniform vec2 texelStep;       // one texel along the blur axis, in uv
uniform vec2 uvLow;           // centre of the first and last texel that hold data
uniform vec2 uvHigh;
uniform vec2 uvScale;         // v_texcoord to source uv
uniform float sigma;          // texels
uniform int pairs;            // filtered fetches on each side of the centre

uniform int applyFace;        // on the last pass: 1 for the face, 2 to divide a weighted copy by its weight
uniform float saturation;
uniform float brightness;
uniform float contrast;
uniform vec3 tintColor;
uniform float tintOverDark;
uniform float tintOverBright;

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

const vec3 LUMA = @LUMA@;

// The material: colour comes through stronger than light and dark do, then a
// tint by an amount that follows the luminance behind, so that what is written
// on the pane keeps its contrast.
vec3 faceOf(vec3 color) {
    float luminance = dot(color, LUMA);
    color = mix(vec3(luminance), color, saturation);
    color = (color - 0.5) * contrast + 0.5;
    color *= brightness;
    return clamp(mix(color, tintColor, mix(tintOverDark, tintOverBright, clamp(luminance, 0.0, 1.0))), 0.0, 1.0);
}

void main() {
    vec2  uv        = v_texcoord * uvScale;
    float inverse   = -0.5 / (sigma * sigma);
    vec4  sum       = texture(tex, clamp(uv, uvLow, uvHigh));
    float weightSum = 1.0;
    for (int i = 0; i < pairs; i++) {
        float x1 = float(2 * i + 1);
        float x2 = x1 + 1.0;
        float w1 = exp(x1 * x1 * inverse);
        float w2 = exp(x2 * x2 * inverse);
        float w  = w1 + w2;
        vec2  o  = texelStep * ((x1 * w1 + x2 * w2) / w);
        sum += w * (texture(tex, clamp(uv + o, uvLow, uvHigh)) + texture(tex, clamp(uv - o, uvLow, uvHigh)));
        weightSum += 2.0 * w;
    }
    vec4 blurred = sum / weightSum;
    if (applyFace == 2)
        // reduce.frag left colour times weight, and the weight in alpha
        fragColor = vec4(clamp(blurred.rgb / max(blurred.a, 0.004), 0.0, 1.0), 1.0);
    else
        fragColor = applyFace == 1 ? vec4(faceOf(blurred.rgb), 1.0) : blurred;
}
