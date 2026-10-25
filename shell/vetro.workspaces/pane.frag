#version 440
// A pane of glass over the focused workspace's number: a rounded tile that is
// clear but for the light its edge catches. The light on the edge follows how
// squarely each point of it faces the light, all the way round, so there is
// no line where lit turns to dark: strongest on the edge that faces the light,
// a share of it on the edge facing away (light that crossed the glass), little
// on the edges across. The two lobes are Liquid Glass Studio's glare
// (iyinchao/liquid-glass-studio, MIT): a strong and an opposite lobe, falling
// off inward from the edge.
//
// Compiled with: qsb --glsl "100 es,120,150" --hlsl 50 --msl 12 -o pane.frag.qsb pane.frag

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    vec2 paneSize;     // the tile, in logical pixels
    float radius;      // its corner radius
    float lightAngle;  // where the light comes from, in radians: 0 is straight above, positive from the right
    float glare;       // how bright the lit edges are
    float opposite;    // the share of it on the edge that faces away
    float sharpness;   // how tightly the light keeps to the edges that face it
    float rimDepth;    // how far in from the edge the edge light reaches
    float fresnel;     // the faint brightening of the whole edge
    float body;        // the tile's own faint fill
    float hairline;    // the line along the very edge
    float glint;       // 0 at rest, up to 1 while the tile slides
    float pixel;       // one device pixel, in logical pixels
};

// Signed distance to a rounded box (negative inside) and the outward normal of
// the nearest point of its outline, continuous where the flat sides meet the
// corners.
float roundBox(vec2 p, vec2 halfSize, float r, out vec2 n) {
    vec2 q = abs(p) - halfSize + r;
    vec2 m = max(q, 0.0);
    vec2 s = vec2(p.x < 0.0 ? -1.0 : 1.0, p.y < 0.0 ? -1.0 : 1.0);
    if (q.x > 0.0 && q.y > 0.0) {
        n = normalize(m) * s;
    } else if (q.x > q.y) {
        n = vec2(s.x, 0.0);
    } else {
        n = vec2(0.0, s.y);
    }
    return length(m) + min(max(q.x, q.y), 0.0) - r;
}

void main() {
    vec2 p = (qt_TexCoord0 - 0.5) * paneSize;
    vec2 n;
    float d = roundBox(p, paneSize * 0.5 - pixel, radius, n);
    float coverage = clamp(0.5 - d / pixel, 0.0, 1.0);
    if (coverage <= 0.0) {
        fragColor = vec4(0.0);
        return;
    }
    float inward = max(-d, 0.0);

    // Toward the light, with y pointing down the screen.
    vec2 light = vec2(sin(lightAngle), -cos(lightAngle));
    float facing = dot(n, light);
    float lit = pow(max(facing, 0.0), sharpness) + opposite * pow(max(-facing, 0.0), sharpness);
    lit *= 1.0 + 0.6 * glint;

    float band = exp(-inward / max(rimDepth, 0.5));
    float edge = pow(1.0 - clamp(inward / max(rimDepth * 2.0, 1.0), 0.0, 1.0), 3.0);
    float line = clamp(1.0 - inward / (pixel * 1.5), 0.0, 1.0);

    float a = body
            + fresnel * edge
            + glare * lit * band
            + hairline * line * (0.3 + 0.7 * clamp(lit, 0.0, 1.0));
    a = clamp(a, 0.0, 1.0) * coverage * qt_Opacity;
    // White light, premultiplied.
    fragColor = vec4(a, a, a, a);
}
