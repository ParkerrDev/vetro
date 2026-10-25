#version 300 es
precision highp float;

// A window's picture laid over a mesh that bends it: the genie, a wobbling
// window. The mesh's points carry where in the picture each one shows, and the
// picture is premultiplied, so the whole of it is scaled by how much shows.
//
// The window's glass is drawn the same way, from a copy of the frame where the
// glass was drawn on it: only the pane is taken from that copy, its rounded
// shape cut out here (a superellipse corner, as Hyprland rounds windows), and
// it is opaque, as the frame is. Over a copy of the whole window, its picture
// gives only what lies outside the pane: the border.

uniform sampler2D tex;   // the window's picture, or the copy with its glass
uniform float alpha;     // how much of it shows, 0 to 1
uniform int pane;        // 1: tex is the copy, and the pane is cut out of it;
                         // 2: tex is the picture, and all but the pane is taken
uniform vec2 texSize;    // the copy's size, in pixels
uniform vec4 shape;      // the pane in the copy: x, y, width, height, in pixels
uniform float radius;    // its corners' radius, in pixels
uniform float power;     // their roundness: 2 is a circle's

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

// How far outside the pane a point is, in pixels: negative inside.
float paneDistance(vec2 point) {
    vec2 half_ = shape.zw * 0.5;
    float r = min(radius, min(half_.x, half_.y));
    vec2 q = abs(point - (shape.xy + half_)) - (half_ - vec2(r));
    vec2 corner = max(q, vec2(0.0));
    float p = max(power, 1.0);
    float out_ = corner.x > 0.0 || corner.y > 0.0 ? pow(pow(corner.x, p) + pow(corner.y, p), 1.0 / p) : 0.0;
    return out_ + min(max(q.x, q.y), 0.0) - r;
}

void main() {
    vec4 colour = texture(tex, v_texcoord);
    if (pane != 0) {
        float cover = clamp(0.5 - paneDistance(v_texcoord * texSize), 0.0, 1.0);
        colour = pane == 1 ? vec4(colour.rgb, 1.0) * cover : colour * (1.0 - cover);
    }
    fragColor = colour * alpha;
}
