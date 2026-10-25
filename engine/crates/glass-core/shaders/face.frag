#version 300 es
precision highp float;

uniform sampler2D tex;        // frosted copy of what lies behind this pane, face applied; or, layered, its cloud
uniform vec2 quadOrigin;      // box-local device pixels of the quad's top left
uniform vec2 quadSize;
uniform vec2 frostOrigin;     // box-local device pixels of the frost texture's top left
uniform vec2 invFrostSize;    // 1 / device pixels the frost texture spans
uniform float opacity;        // glass_opacity times the layer's fade
uniform float noiseStrength;
uniform int layered;          // 1 when the frost is the current material's two copies, and the face is still to apply

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

const vec3 LUMA = @LUMA@;

@LAYERED@

// hash12 from "Hash without Sine" (Dave Hoskins)
float pixelHash(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

void main() {
    vec2 p     = quadOrigin + v_texcoord * quadSize;
    vec3 color = texture(tex, (p - frostOrigin) * invFrostSize).rgb;
    if (layered == 1)
        color = clamp(faceOf(layeredFrost(p, color)), 0.0, 1.0);
    if (noiseStrength > 0.001)
        color = clamp(color + (pixelHash(floor(p)) - 0.5) * noiseStrength * 0.1, 0.0, 1.0);
    fragColor = vec4(color * opacity, opacity);
}
