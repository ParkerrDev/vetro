#version 300 es
precision highp float;

// The page a piece of glass reads when windows lie under it: the frame as it is
// where one of those windows covers a pixel, and the backdrop (the wallpaper
// alone) everywhere else, so that other windows show through glass and the
// desktop's widgets, drawn between the wallpaper and the windows, never do.
// One texel of the frame to one texel of the page, as a blit would copy it.

uniform sampler2D liveTex;     // the frame being drawn
uniform sampler2D backdropTex; // the monitor's backdrop, the same size and rows as the frame
uniform ivec2 origin;          // the page's first texel in the frame
uniform int   count;           // windows below
uniform vec4  boxes[16];       // each window's box: x, y, width, height in frame texels
uniform vec2  corners[16];     // its corner radius in texels and Hyprland's rounding_power

layout(location = 0) out vec4 fragColor;

// How much of the pixel at p a rounded box covers: within a corner's square the
// outline is |x|^n + |y|^n = r^n, as Hyprland rounds a window.
float covered(vec2 p, vec4 box, vec2 corner) {
    vec2  halfSize = 0.5 * box.zw;
    float r        = clamp(corner.x, 0.0, min(halfSize.x, halfSize.y));
    float n        = max(corner.y, 1.01);
    vec2  q        = abs(p - (box.xy + halfSize)) - halfSize + vec2(r);
    vec2  m        = max(q, vec2(0.0));
    float dist     = pow(pow(m.x, n) + pow(m.y, n), 1.0 / n) + min(max(q.x, q.y), 0.0) - r;
    return clamp(0.5 - dist, 0.0, 1.0);
}

void main() {
    ivec2 at   = origin + ivec2(gl_FragCoord.xy);
    vec2  p    = vec2(at) + 0.5;
    float seen = 0.0;
    for (int i = 0; i < 16; ++i) {
        if (i >= count)
            break;
        seen = max(seen, covered(p, boxes[i], corners[i]));
    }
    vec4 live = texelFetch(liveTex, at, 0);
    vec4 back = texelFetch(backdropTex, at, 0);
    fragColor = mix(back, live, seen);
}
