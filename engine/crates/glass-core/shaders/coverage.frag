#version 300 es
precision highp float;

uniform sampler2D tex;        // the layer's surface, premultiplied
uniform vec2 boxSize;         // layer box, device pixels
uniform vec2 margin;          // device pixels of margin left and above the box
uniform vec2 maskUvScale;     // box uv to surface texture uv
uniform float threshold;      // alpha from which a pixel is pane
uniform int scale;            // device pixels per texel, 1, 2 or 4

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

void main() {
    vec2  texel = floor(gl_FragCoord.xy);
    float sum   = 0.0;
    for (int j = 0; j < scale; j++) {
        for (int i = 0; i < scale; i++) {
            vec2 p = texel * float(scale) + vec2(float(i), float(j)) + 0.5 - margin;
            if (p.x < 0.0 || p.y < 0.0 || p.x >= boxSize.x || p.y >= boxSize.y)
                continue;
            sum += step(threshold, texture(tex, p / boxSize * maskUvScale).a);
        }
    }
    float share = sum / float(scale * scale);
    fragColor   = vec4(share, share, share, 1.0);
}
