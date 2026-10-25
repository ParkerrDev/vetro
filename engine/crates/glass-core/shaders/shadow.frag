#version 300 es
precision highp float;

uniform sampler2D tex;        // blurred coverage for the drop shadow
uniform vec2 quadOrigin;      // box-local device pixels of the quad's top left
uniform vec2 quadSize;
uniform vec2 fieldMargin;     // device pixels of field left and above the box
uniform vec2 invShadowSize;   // 1 / device pixels the shadow texture spans
uniform vec2 shadowOffset;    // device pixels
uniform float shadowOpacity;  // times the layer's fade

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

void main() {
    vec2 p    = quadOrigin + v_texcoord * quadSize;
    fragColor = vec4(0.0, 0.0, 0.0, shadowOpacity * texture(tex, (p - shadowOffset + fieldMargin) * invShadowSize).r);
}
