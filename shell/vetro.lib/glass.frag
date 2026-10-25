#version 440
// Liquid glass drawn by the shell itself, for the surfaces the glass engine
// never sees (the lock screen above all): a shape of clear glass over a
// backdrop the caller hands in. Near its outline the glass curves, and what is
// behind it is read from further in, as through a magnifier, red a little
// further than blue; a share of it is frosted; it catches the light on the
// edge facing the light and a little on the edge facing away (the two lobes of
// pane.frag); a soft shadow lies just outside it.
//
// The shape is a rounded box worked out here (shaped = 0), or any alpha the
// caller draws, a clock's numerals say (shaped = 1): then `shape` is that
// alpha and `soft` the same alpha blurred, whose slope gives the outline's
// direction and whose height how deep into the glass a point is.
//
// Compiled with: qsb --glsl "100 es,120,150" --hlsl 50 --msl 12 -o glass.frag.qsb glass.frag

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    vec4 area;         // this item within the backdrop: x, y, width, height as shares of it
    vec2 extent;       // this item, in logical pixels
    vec2 texel;        // one pixel of the soft alpha, as a share of it
    float margin;      // room left round a box for its shadow
    float radius;      // a box's corner radius
    float shaped;      // 0: a rounded box; 1: the shape is the alpha
    float bezel;       // how far in from the outline the glass curves
    float refraction;  // how far the curve moves what is behind it, in pixels
    float dispersion;  // how much further red is moved than blue, as a share
    float frost;       // 0 clear, 1 the blurred backdrop
    float tintAmount;  // how much of the tint colour lies on it
    vec4 tint;         // the tint colour
    float saturation;  // 1 as it is, more for richer colour
    float lightAngle;  // where the light comes from: 0 straight above, positive from the right
    float glare;       // how bright the lit edge is
    float opposite;    // the share of it on the edge facing away
    float sharpness;   // how tightly the light keeps to the edge facing it
    float hairline;    // the line along the very edge
    float shadow;      // how dark the shadow just outside is
    float pixel;       // one device pixel, in logical pixels
};

layout(binding = 1) uniform sampler2D backdrop;
layout(binding = 2) uniform sampler2D blurred;
layout(binding = 3) uniform sampler2D shape;
layout(binding = 4) uniform sampler2D soft;

// Signed distance to a rounded box (negative inside) and the outward normal of
// the nearest point of its outline, continuous where the sides meet the
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

// Upper arc of a unit circle: the glass's height across its bezel.
float arc(float t) {
    t = clamp(t, 0.0, 1.0);
    return sqrt(max((2.0 - t) * t, 0.0));
}

vec3 saturate3(vec3 c, float amount) {
    float grey = dot(c, vec3(0.2126, 0.7152, 0.0722));
    return clamp(mix(vec3(grey), c, amount), 0.0, 1.0);
}

void main() {
    vec2 uv = qt_TexCoord0;
    vec2 p = (uv - 0.5) * extent;

    // How much of this pixel is glass, how deep in (0 on the outline, 1 past
    // the bezel), and the outline's outward direction.
    float coverage;
    float depth;
    float outside;
    vec2 n;
    if (shaped < 0.5) {
        float d = roundBox(p, extent * 0.5 - margin, radius, n);
        coverage = clamp(0.5 - d / pixel, 0.0, 1.0);
        depth = clamp(-d / max(bezel, 1.0), 0.0, 1.0);
        outside = exp(-max(d, 0.0) / max(margin * 0.45, 1.0)) * step(0.0, d);
    } else {
        coverage = texture(shape, uv).a;
        float h = texture(soft, uv).a;
        vec2 slope = vec2(texture(soft, uv + vec2(texel.x, 0.0)).a - texture(soft, uv - vec2(texel.x, 0.0)).a,
                          texture(soft, uv + vec2(0.0, texel.y)).a - texture(soft, uv - vec2(0.0, texel.y)).a);
        float l = length(slope);
        n = l > 1e-5 ? -slope / l : vec2(0.0, -1.0);
        depth = clamp((h - 0.5) * 2.0, 0.0, 1.0);
        outside = h * 1.6;
    }

    // The shadow, under the glass and round it.
    float shade = clamp(shadow * outside, 0.0, 1.0) * (1.0 - coverage);
    if (coverage <= 0.0) {
        fragColor = vec4(0.0, 0.0, 0.0, shade * qt_Opacity);
        return;
    }

    // The curve: what is behind the bezel is read from further in.
    float bend = refraction * (1.0 - arc(depth));
    vec2 inward = -n * bend / extent;
    vec2 at = area.xy + uv * area.zw;
    vec2 step1 = inward * area.zw;
    vec3 clear = vec3(texture(backdrop, at + step1 * (1.0 + dispersion)).r,
                      texture(backdrop, at + step1).g,
                      texture(backdrop, at + step1 * (1.0 - dispersion)).b);
    vec3 frosted = texture(blurred, at + step1).rgb;
    vec3 c = mix(clear, frosted, clamp(frost, 0.0, 1.0));
    c = saturate3(c, saturation);
    c = mix(c, tint.rgb, clamp(tintAmount, 0.0, 1.0));

    // The light on the edge, strongest where it faces the light.
    vec2 light = vec2(sin(lightAngle), -cos(lightAngle));
    float facing = dot(n, light);
    float lit = pow(max(facing, 0.0), sharpness) + opposite * pow(max(-facing, 0.0), sharpness);
    float rim = 1.0 - depth;
    float band = rim * rim * rim;
    float line = clamp(1.0 - depth * max(bezel, 1.0) / (pixel * 1.5), 0.0, 1.0);
    float glow = glare * lit * band + hairline * line * (0.35 + 0.65 * clamp(lit, 0.0, 1.0));
    c = c + (1.0 - c) * clamp(glow, 0.0, 1.0);

    float a = coverage * qt_Opacity;
    fragColor = vec4(c * a, a) + vec4(0.0, 0.0, 0.0, shade * qt_Opacity);
}
