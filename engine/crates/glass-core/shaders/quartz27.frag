#version 300 es
precision highp float;

// Apple's own glass shader as transcribed: macOS 27's glass_background_all_lpf
// (QuartzCore default.metallib, build 26A5388g), its 63 uniforms in Apple's
// order and its stages in Apple's order. Adapted from liquid_glass_27.glsl of
// liquid-glass-react-native (MIT, Copyright (c) 2026 liquid-glass
// contributors; see NOTICE). The numbers it is given are crate::quartz.
//
// What is changed from the transcription, and why:
//  * the shape is whatever the layer drew: distance and direction come from a
//    distance field of its outline (quartzseed.frag, quartzflood.frag), as
//    Apple's own come from the SDF texture CASDFLayer makes of a view's shape,
//    and not from a superellipse over the whole panel, which only a capsule
//    would fit;
//  * every length is in points, the same on every pane: the transcription
//    scaled them with the panel's size, so a small bar and a large window were
//    two different materials (2026-10-18: "I want them all to look
//    identical");
//  * the key and fill light is as gpui-glass-demo decoded it from macOS 27.0's
//    QuartzCore (BSD 3-Clause, Copyright (c) 2026 GPUI glass contributors; see
//    NOTICE), not the transcription's model of it: the background's key and
//    fill darken a rim a point outside the face, and the bright light is a
//    separate layer of a sharp and a soft lobe through vibrantColorMatrix
//    (brightLayer); their lengths, and the blur fill's, are points;
//  * a window's pane has no layer to trace: its shape is its own rounded
//    rectangle, with the corners Hyprland draws (rounding and rounding_power),
//    worked out here (analytic);
//  * the page is a copy of what lies around the pane, with a mip chain, in
//    box-local device pixels with row 0 at the top;
//  * the glass ends where the layer drew its pane, as the rest of the engine's
//    glass does, and the result is premultiplied for Hyprland's blend.
// One pass draws the whole pane: Apple's shader has no separate face and rim.

uniform sampler2D tex;        // the page around the pane, with its mip chain
uniform sampler2D maskTex;    // the layer's surface, premultiplied
uniform sampler2D seedTex;    // per field texel, the nearest point of the outline (packed)
uniform sampler2D fieldTex;   // rg inward normal, b blurred coverage
uniform sampler2D blurTex;    // the backdrop blurred for the face, when preBlurred
uniform int preBlurred;
uniform int edgeless;         // 1 for a pane over the whole monitor: it has no edge to bend or light

uniform vec2 quadOrigin;      // box-local device pixels of the quad's top left
uniform vec2 quadSize;
uniform vec2 boxSize;         // layer box, device pixels
uniform vec2 maskUvScale;     // box uv to surface texture uv
uniform float maskThreshold;  // alpha from which the layer's pixels are pane
uniform vec2 fieldMargin;     // device pixels of field left and above the box
uniform vec2 invFieldSize;    // 1 / device pixels the field texture spans
uniform vec2 seedMargin;      // device pixels of the outline's field left and above the box, one texel a pixel
uniform ivec2 seedHigh;       // the last texel of the outline's field that holds data
uniform float pointScale;     // device pixels in a point
uniform int analytic;         // 1 for a window's pane: the shape is the rectangle below, not the layer's outline
uniform vec2 rectCenter;      // box-local device pixels
uniform vec2 rectHalf;
uniform float rectRadius;     // device pixels
uniform float rectPower;      // Hyprland's rounding_power: 2 is a circle
uniform float lightGain;      // the bright layer's share: a window without the focus is lit less
uniform vec2 pageOrigin;      // box-local device pixels of the copy's top left
uniform vec2 pageSize;        // device pixels the copy spans
uniform float opacity;        // the glass's opacity times the layer's fade

layout(std140) uniform GlassParams27 {
    vec4  displacement_mat;
    float inner_refraction_amount;
    float inner_refraction_inv_height;
    float outer_refraction_amount;
    float outer_refraction_inv_height;
    float refraction_threshold0;
    float refraction_threshold1;
    float blur_radius;
    float edge_bleed_blur_radius;
    float edge_bleed_amount;
    float edge_bleed_inv_height;
    float shadow_amount;
    float shadow_inv_height;
    vec2  shadow_offset;
    float shadow_blur_radius;
    float shadow_inv_radius;
    vec4  face_cm0, face_cm1, face_cm2;
    vec4  bleed_cm0, bleed_cm1, bleed_cm2;
    vec4  shadow_cm0, shadow_cm1, shadow_cm2;
    float shadow_contribution;
    float shadow_face_opacity;
    float blur_alpha0, blur_alpha1, blur_alpha2, blur_alpha3;
    float blur_dist0,  blur_dist1,  blur_dist2,  blur_dist3;
    float edge_bleed_dist0, edge_bleed_dist1;
    float edge_bleed_opacity;
    float face_opacity;
    vec2  bleed_darken;
    float shadow_dist_offset;
    float shadow_opacity;
    float refraction_opacity;
    float holding_tone_opacity;
    float sdr_shadow_dist0;
    float sdr_shadow_inv;
    vec2  ring_shadow_offset;
    float ring_shadow_stroke_width;
    float ring_shadow_radius;
    float ring_shadow_opacity;
    float ring_shadow_mask;
    vec2  key_fill_highlight_dir;
    float key_fill_highlight_height;
    float key_fill_highlight_spread;
    float key_fill_highlight_amount;
    float key_fill_highlight_effect_offset;
    float key_fill_highlight_color_bias;
    float blur_fill_blur_radius;
    float blur_fill_lighten_opacity;
    float blur_fill_darken_opacity;
    float blur_fill_normal_opacity;
    float aberration_amount;
    vec2  aberration_dir;
    float bright_layer;           // the engine's: 1 to draw macOS 27's bright layer
    float hairline;               // the engine's: white the lit hairline adds
    vec3  tint;                   // the engine's: the dark glass colour
    float tint_amount;            //   how far the face goes toward it
    float tint_adaptive;          //   and how much further for each step of brightness
    float saturation;             // the engine's: colour behind saturated by this (0 = 1)
    float rim_glow;               // the engine's: soft light inside the lit edges
    float rim_glow_depth;         //   and how deep it reaches, in points
    vec2  light_dir;              // the engine's: where the bright layer's key light comes from (y down)
    vec2  hairline_dir;           // the engine's: the light the hairline and the glow face
    float side_light;             // the engine's: the share of the key the edges across it catch
    float lift;                   // the engine's: the bright layer's matrix over the whole face, iOS 27's light of its own
    float body_bend;              // the engine's: how far a window's whole face bends, as a gently curved sheet
};

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

const float EPS  = 1.0e-4;
const float TINY = 1.0e-6;
// Rec.709, as tile_average_luma's fp16 immediates have it
const vec3 LUMA709 = vec3(0.212646, 0.715332, 0.072205);

const float NONE = 65535.0;

vec2 unpackSeed(vec4 c) {
    vec4 b = floor(c * 255.0 + 0.5);
    return vec2(b.x * 256.0 + b.y, b.z * 256.0 + b.w);
}

// Upper arc of a unit circle, spelled sqrt((2-t)*t) as macOS 27 does.
float lensCurve(float t) {
    t = clamp(t, 0.0, 1.0);
    return clamp(sqrt(max((2.0 - t) * t, 0.0)), 0.0, 1.0);
}
float refractLobe(float d, float amount, float invH, float offset) {
    return amount - lensCurve(clamp((-d - offset) * invH, 0.0, 1.0)) * amount;
}

float blurLOD(float r) {
    r = (r < 2.0) ? (r * 0.5 + 1.0) : r;
    return max(0.0, log2(max(r, TINY)));
}
vec4 tap(vec2 uv, float radius) {
    return textureLod(tex, uv, blurLOD(radius));
}

// The face's read. With the backdrop blurred for it (a true Gaussian, made once
// per wallpaper), one bilinear tap. From a mip chain, four reads a level finer
// on a rotated grid, which smooths the blocks one level would show.
vec4 faceTap(vec2 uv, float radius, vec2 texel) {
    if (preBlurred == 1)
        return texture(blurTex, uv);
    float lod = blurLOD(radius);
    if (lod < 1.0)
        return textureLod(tex, uv, lod);
    vec2 s = texel * exp2(lod - 1.0) * 0.75;
    float fine = lod - 1.0;
    return 0.25 * (textureLod(tex, uv + s * vec2(0.35, 1.0), fine) + textureLod(tex, uv + s * vec2(1.0, -0.35), fine)
                 + textureLod(tex, uv + s * vec2(-0.35, -1.0), fine) + textureLod(tex, uv + s * vec2(-1.0, 0.35), fine));
}

// 4-point piecewise-linear ramp: distance -> blur radius (the transcription's
// orientation, which its author fitted).
float blurRamp(float d, float S) {
    vec3 lo = vec3(blur_dist0, blur_dist1, blur_dist2);
    vec3 hi = vec3(blur_dist1, blur_dist2, blur_dist3);
    vec3 sp = hi - lo;
    vec3 inv = 1.0 / mix(sp, vec3(TINY), step(abs(sp), vec3(TINY)));
    vec3 t = clamp((vec3(d) - lo) * inv, 0.0, 1.0);
    vec3 w = vec3(blur_alpha1, blur_alpha2, blur_alpha3) * t;
    return (blur_alpha0 - (w.x + w.y + w.z)) * blur_radius * S;
}

// Degree-7 minimax 0.5*erfc(x) over [-2,2], from the fp16 immediates.
float erfcHalf(float d, float invR) {
    float u = clamp(0.25 * (d * invR) + 0.5, 0.0, 1.0);
    float x = 4.0 * u - 2.0, x2 = x * x;
    float p = 0.002954;
    p = p * x2 - 0.034460;
    p = p * x2 + 0.168210;
    p = p * x2 - 0.560550;
    return p * x + 0.5;
}

vec3 grade(vec4 c, vec4 r0, vec4 r1, vec4 r2) {
    float a = max(c.a, TINY);
    vec3 rgb = c.rgb / a;
    rgb *= step(vec3(TINY), abs(rgb));
    return vec3(dot(rgb, r0.xyz) + r0.w,
                dot(rgb, r1.xyz) + r1.w,
                dot(rgb, r2.xyz) + r2.w);
}

// sdf_key_fill_highlight as decoded from macOS 27.0 (gpui-glass-demo,
// RESEARCH.md): a band `height` deep from `offset` in, with a radial core whose
// softness is 0.75 for the sharp lobe and 1 (linear) for the soft one, both
// edges antialiased; two opposing lobes past the spread's cosine; and the
// rational shoulder lobes / (1 + curve * (1 - lobes)), where the curve is the
// host's 1 / CA amount - 2.
float keyFill(vec2 n, float depth, float height, vec2 dir, float offset, float spreadCos, float curve,
              float softness, float aa, float opposite) {
    float z      = depth - offset;
    float ramp   = 1.0 - clamp(z / max(height, 1.0e-4), 0.0, 1.0);
    float core   = mix(ramp > 0.0 ? 1.0 : 0.0, ramp, softness);
    float radial = core * clamp((height - z) / aa + 0.5, 0.0, 1.0) * clamp(z / aa + 0.5, 0.0, 1.0);
    float facing = dot(n, dir);
    vec2  lobes  = clamp((vec2(facing, -facing) - vec2(spreadCos)) / max(1.0 - spreadCos, 1.0e-6), 0.0, 1.0) * radial;
    vec2  shaped = lobes / (1.0 + curve * (1.0 - lobes));
    return shaped.x + opposite * shaped.y;
}

// The bright layer's values an NSGlassEffectView sets on macOS 27.0, through the
// host's conversions: key angle 0 (up) and fill pi (down), here light_dir; the
// sharp lobe a point deep, CA amount 0.5 (curve 0), spread 80 degrees; the soft
// one eight times as deep, amount 0.5 x 0.15 (curve 11 1/3), spread 0.65 of the
// sharp's.
const float SHARP_HEIGHT = 1.0;
const float SHARP_CURVE  = 0.0;
const float SHARP_SPREAD = 0.17364818;   // cos(80 degrees)
const float SOFT_HEIGHT  = 8.0;
const float SOFT_CURVE   = 11.333333;
const float SOFT_SPREAD  = 0.6156615;    // cos(52 degrees)
// Vetro's, not Apple's: the edges across the key light catch side_light of it
// in the sharp lobe (a half for the older kinds), so that the edge reads as lit
// glass all round, and never as a dark line between a lit top and bottom.
// Vetro's hairline: the outline itself, a point wide, lit by how squarely it
// faces hairline_dir (both ends of that axis, as Kyant0's highlight is: the top
// left for the older kinds), and never below a third on the edges across it.
const float HAIRLINE_WIDTH = 1.0;
const float HAIRLINE_FLOOR = 0.34;
// A lens is never deeper than this share of the pane's shorter half: on a
// strip as thin as the bar a full bezel would bend all of it.
const float LENS_SHARE = 0.45;
// The body bend at its fullest: at the edge, the face reads this share of its
// half size in toward the centre (a gentle magnifier), on a window's glass only.
const float BODY_PUSH = 0.12;

// vibrant_color_matrix_sover: the face under the mask goes to M * face + b,
// which lifts dark glass more than light (black to 0.147, grey 0.5 to 0.603).
vec3 brightLayer(vec3 color, float mask) {
    vec3 target = vec3(dot(color, vec3(1.1201999, -0.1894, -0.019)),
                       dot(color, vec3(-0.0563, 0.98709995, -0.0191)),
                       dot(color, vec3(-0.0563, -0.1893, 1.1574))) + vec3(0.1471);
    return mix(color, clamp(target, 0.0, 1.0), clamp(mask, 0.0, 1.0));
}

// A rectangle with Hyprland's corners: within a corner's square the outline is
// |x|^n + |y|^n = r^n. Signed distance (negative inside) and outward normal.
void roundedRect(vec2 p, vec2 halfSize, float radius, float power, out float dist, out vec2 nrm) {
    float r = clamp(radius, 0.0, min(halfSize.x, halfSize.y));
    float n = max(power, 1.01);
    vec2  q = abs(p) - halfSize + vec2(r);
    vec2  m = max(q, vec2(0.0));
    float corner = pow(pow(m.x, n) + pow(m.y, n), 1.0 / n);
    dist = corner + min(max(q.x, q.y), 0.0) - r;
    vec2 g = (m.x > 0.0 || m.y > 0.0) ? vec2(pow(m.x, n - 1.0), pow(m.y, n - 1.0))
                                      : (q.x > q.y ? vec2(1.0, 0.0) : vec2(0.0, 1.0));
    float gl = length(g);
    nrm = gl > TINY ? sign(p + vec2(TINY)) * g / gl : vec2(0.0, 1.0);
}

// The layer's own alpha at a point of its box, 0 outside it.
float alphaAt(vec2 p) {
    if (p.x < 0.0 || p.y < 0.0 || p.x >= boxSize.x || p.y >= boxSize.y)
        return 0.0;
    return texture(maskTex, p / boxSize * maskUvScale).a;
}

// Signed distance from the outline (negative inside) and the outward normal at
// p: the nearest point of the outline the flood found, which lies half a field
// texel inside it, and near the outline the field's smoother normal.
void outline(vec2 p, bool inside, out float dist, out vec2 nrm) {
    ivec2 at   = clamp(ivec2(floor(p + seedMargin)), ivec2(0), seedHigh);
    vec2  seed = unpackSeed(texelFetch(seedTex, at, 0));
    if (seed.x >= NONE) {
        dist = inside ? -1.0e4 : 1.0e4;
        nrm  = vec2(0.0, 1.0);
        return;
    }
    vec2  d     = p - (seed + 0.5 - seedMargin);
    float L     = length(d);
    float halfT = 0.5;
    dist = inside ? -(L + halfT) : (L - halfT);
    vec2 outward = L > TINY ? (inside ? -d : d) / L : vec2(0.0);
    vec2 inward  = texture(fieldTex, (p + fieldMargin) * invFieldSize).rg * 2.0 - 1.0;
    float l2     = dot(inward, inward);
    float near   = l2 > 1.0e-4 ? 1.0 - smoothstep(1.0, 3.0, L) : 0.0;
    vec2 n = mix(outward, l2 > 1.0e-4 ? -inward * inversesqrt(l2) : outward, near);
    float nl = length(n);
    nrm = nl > TINY ? n / nl : vec2(0.0, 1.0);
}

void main() {
    vec2  p     = quadOrigin + v_texcoord * quadSize;
    float S     = pointScale;
    vec2  texel = 1.0 / pageSize;
    vec2  uv    = (p - pageOrigin) * texel;

    // How much of this pixel the layer drew as pane: the share of the way from
    // half the cut-off to the cut-off past it, so that a dim around a card
    // (under half the cut-off) is never glass.
    float drawn;
    float dist; vec2 nrm;
    if (analytic == 1) {
        roundedRect(p - rectCenter, rectHalf, rectRadius, rectPower, dist, nrm);
        drawn = clamp(0.5 - dist, 0.0, 1.0);
    } else {
        float own = alphaAt(p);
        drawn = clamp((own - 0.5 * maskThreshold) / max(maskThreshold, EPS), 0.0, 1.0);
        outline(p, own >= maskThreshold, dist, nrm);
    }

    vec2 rot  = vec2(dot(nrm, vec2(1.0, 0.0)), dot(nrm, vec2(0.0, 1.0)));
    vec2 disp = vec2(dot(rot, displacement_mat.xy), dot(rot, displacement_mat.zw));

    vec3  outRGB = vec3(0.0);
    float cover  = 0.0;

    // 1. shadow, then ring shadow: macOS 27 draws these first, and the face's
    //    colour matrix then works on a surface that carries them
    if (shadow_opacity > 0.0) {
        float smag = refractLobe(dist, shadow_amount * S, shadow_inv_height / S, shadow_dist_offset);
        vec4  scol = tap(uv + (smag * disp + shadow_offset) * texel, shadow_blur_radius * S);
        vec3  sh   = grade(scol, shadow_cm0, shadow_cm1, shadow_cm2) * shadow_contribution;
        float smask = erfcHalf(dist, shadow_inv_radius / S) * shadow_opacity * step(0.0, dist);
        outRGB = mix(outRGB, sh, smask);
        cover  = max(cover, smask * shadow_face_opacity);
    }
    if (ring_shadow_opacity > 0.0) {
        float invR  = 1.0 / max(ring_shadow_radius * S, TINY);
        float d     = -dist - ring_shadow_offset.x;
        float inner = d * invR;
        float outer = inner + (ring_shadow_stroke_width * S) * invR;
        const float INV_SQRT2 = 0.70710678;
        float ring = clamp(erfcHalf(inner * INV_SQRT2, 1.0) - erfcHalf(outer * INV_SQRT2, 1.0), 0.0, 1.0)
                   * ring_shadow_opacity * clamp(ring_shadow_mask, 0.0, 1.0);
        outRGB *= (1.0 - ring);
    }

    // 2. the face: inner refraction and the blur ramp. The lens is kept within
    //    the pane: its depth (and its push with it) shrinks on a thin pane.
    float halfMin   = analytic == 1 ? min(rectHalf.x, rectHalf.y) : 0.5 * min(boxSize.x, boxSize.y);
    float lensDepth = 1.0 / max(inner_refraction_inv_height, TINY);
    float lensScale = clamp(LENS_SHARE * halfMin / max(lensDepth * S, TINY), 0.05, 1.0);
    float innerMag = edgeless == 1 ? 0.0 : refractLobe(dist, inner_refraction_amount * lensScale * S, inner_refraction_inv_height / (lensScale * S), 0.0);
    // 2a. Vetro's body bend: a window's whole face as a thick sheet curved
    //     gently over it, flat at the centre and bending more toward each edge
    //     (q * |q| along each axis), so that a big window is glass all through
    //     and not a blur inside a band of lens. The read moves toward the
    //     centre, as through a magnifier, and never past the outline.
    vec2 bodyShift = vec2(0.0);
    if (body_bend > 0.0 && analytic == 1 && edgeless == 0) {
        vec2 q = clamp((p - rectCenter) / max(rectHalf, vec2(1.0)), -1.0, 1.0);
        bodyShift = -body_bend * BODY_PUSH * rectHalf * q * abs(q);
    }
    vec2  faceUV   = uv + (innerMag * disp + bodyShift) * texel;
    float faceLod  = blurRamp(innerMag + dist, S);
    vec4  faceCol  = faceTap(faceUV, faceLod, texel);

    // 3. chromatic aberration: 7 taps, 3 forward feeding red and green, 4 back
    //    feeding green and blue, weights normalised to exactly 1
    // Outside its band the offset is nothing and the seven reads are one: they
    // are made only where they would differ by half a pixel or more.
    vec2 offA = vec2(0.0);
    if (aberration_amount > 0.0 && edgeless == 0) {
        vec2 rotA  = vec2(dot(nrm, vec2(aberration_dir.x, -aberration_dir.y)),
                          dot(nrm, vec2(aberration_dir.y,  aberration_dir.x)));
        vec2 dispA = vec2(dot(rotA, displacement_mat.zw), dot(rotA, displacement_mat.xy));
        offA = dispA * refractLobe(dist, aberration_amount * lensScale * S, inner_refraction_inv_height / (lensScale * S), 0.0);
    }
    if (dot(offA, offA) >= 0.25) {
        vec3 acc = vec3(0.0); float aSum = 0.0;
        float w = 1.0;
        for (int i = 0; i < 3; ++i) {
            vec4 s = faceTap(faceUV + offA * w * texel, faceLod, texel);
            float a = max(s.a, TINY);
            acc.r += (s.r / a) * w;
            acc.g += (s.g / a) * (1.0 - w);
            aSum  += s.a;  w -= 1.0 / 3.0;
        }
        float st = 0.0;
        for (int i = 0; i < 4; ++i) {
            vec4 s = faceTap(faceUV - offA * st * texel, faceLod, texel);
            float a = max(s.a, TINY);
            acc.g += (s.g / a) * (1.0 - st);
            acc.b += (s.b / a) * st;
            aSum  += s.a;  st += 1.0 / 3.0;
        }
        acc *= vec3(0.5, 1.0 / 3.0, 0.5);
        faceCol = vec4(acc, aSum * (1.0 / 7.0));
    }

    // 4. blur fill (new in 27): lighten, darken and base partition unity, then
    //    a lerp toward the plain fill
    if (blur_fill_normal_opacity > 0.0 || blur_fill_lighten_opacity > 0.0 || blur_fill_darken_opacity > 0.0) {
        vec3 base = grade(faceCol, face_cm0, face_cm1, face_cm2);
        vec3 fill = grade(tap(uv, blur_fill_blur_radius * pointScale), face_cm0, face_cm1, face_cm2);
        float wBase = 1.0 - blur_fill_lighten_opacity - blur_fill_darken_opacity;
        vec3 mixed  = blur_fill_lighten_opacity * max(base, fill)
                    + blur_fill_darken_opacity  * min(base, fill)
                    + wBase * base;
        faceCol.rgb = mix(mixed, fill, clamp(blur_fill_normal_opacity, 0.0, 1.0));
        faceCol.a = 1.0;
    }

    // 5. outer refraction, cross-faded across the threshold band
    float outerMag = refraction_opacity > 0.0
        ? refractLobe(dist, outer_refraction_amount * S, outer_refraction_inv_height / S, 0.0) : 0.0;
    // deep inside both lobes are nothing, and the outer read would be the face's
    if (refraction_opacity > 0.0 && abs(outerMag - innerMag) >= 0.25) {
        vec4 outerCol = tap(uv + outerMag * disp * texel, blurRamp(outerMag + dist, S));
        float span = refraction_threshold1 - refraction_threshold0;
        float t = clamp((dist - refraction_threshold0) / (abs(span) < TINY ? TINY : span), 0.0, 1.0);
        faceCol = mix(faceCol, outerCol, t * refraction_opacity);
    }

    vec3 face = grade(faceCol, face_cm0, face_cm1, face_cm2) * face_opacity;

    // 6. the background's key and fill (macOS 27, decoded): a rim of two
    //    opposing lobes a point outside the face, which darkens what is there
    //    by the colour bias through 1 + bias * H * (3 - 2 * colour). It lies
    //    beyond the outline, so it is worked out here and laid down in 10.
    float aaWidth = max(fwidth(dist), 0.5);
    float rimDark = 0.0;
    if (key_fill_highlight_color_bias < 0.0 && dist > -aaWidth) {
        float H = keyFill(nrm, -dist, key_fill_highlight_height * pointScale, key_fill_highlight_dir,
                          key_fill_highlight_effect_offset * pointScale, key_fill_highlight_spread,
                          key_fill_highlight_amount, 0.75, aaWidth, 1.0);
        float under = dot(textureLod(tex, uv, 0.0).rgb, LUMA709);
        rimDark = clamp(-key_fill_highlight_color_bias * H * (3.0 - 2.0 * under), 0.0, 1.0);
    }

    // 7. edge bleed: its band grows toward the centre
    // only within its band of the edge
    float bleedBand = edge_bleed_amount > 0.0
        ? (1.0 - smoothstep(edge_bleed_dist0 * S, edge_bleed_dist1 * S, -dist)) * edge_bleed_opacity : 0.0;
    if (bleedBand > 1.0 / 512.0) {
        float bt   = clamp(-dist * (edge_bleed_inv_height / S), 0.0, 1.0);
        float bmag = (edge_bleed_amount - lensCurve(bt) * edge_bleed_amount) * S;
        vec3 bleed = grade(tap(uv + bmag * disp * texel, edge_bleed_blur_radius * S), bleed_cm0, bleed_cm1, bleed_cm2);
        bleed = bleed * bleed_darken.x + vec3(bleed_darken.y);
        face = mix(face, bleed, bleedBand);
    }

    // 7b. Vetro's colour and tint: what is behind saturated, as Apple's glass
    //     makes it, then the face toward a dark glass colour, further over a
    //     bright backdrop, so that text on the glass reads on any wallpaper
    if (abs(saturation - 1.0) > 0.001) {
        float grey = dot(face, LUMA709);
        face = max(mix(vec3(grey), face, saturation), vec3(0.0));
    }
    if (tint_amount > 0.0 || tint_adaptive > 0.0) {
        float L = dot(face, LUMA709);
        float amount = clamp(tint_amount + tint_adaptive * max(L - 0.12, 0.0), 0.0, 0.9);
        face = mix(face, tint, amount);
    }
    // 7c. Vetro's inner light (iOS 27): the glass gives off light of its own,
    //     the bright layer's matrix over the whole face, so it lifts off a dark
    //     backdrop instead of blending into it
    if (lift > 0.0)
        face = brightLayer(face, lift);

    // 8. macOS 27's bright layer: a sharp lobe a point wide and a soft one
    //    eight wide, keyed from above and filled from below, one mask for both,
    //    and through it the face goes to vibrantColorMatrix of itself
    if (bright_layer > 0.0 && edgeless == 0 && dist < aaWidth && -dist < SOFT_HEIGHT * pointScale + aaWidth) {
        float depth = -dist;
        vec2  across = vec2(-light_dir.y, light_dir.x);
        float mask  = keyFill(nrm, depth, SHARP_HEIGHT * pointScale, light_dir, 0.0, SHARP_SPREAD, SHARP_CURVE, 0.75, aaWidth, 1.0)
                    + keyFill(nrm, depth, SOFT_HEIGHT * pointScale, light_dir, 0.0, SOFT_SPREAD, SOFT_CURVE, 1.0, aaWidth, 1.0)
                    + side_light * keyFill(nrm, depth, SHARP_HEIGHT * pointScale, across, 0.0, SHARP_SPREAD, SHARP_CURVE, 0.75, aaWidth, 1.0);
        face = brightLayer(face, mask * lightGain * bright_layer);
    }

    // 8a. Vetro's glow: light inside the edge, falling off over its depth,
    //     strongest where the edge faces the light, as on a thick pane of glass
    if (rim_glow > 0.0 && edgeless == 0 && dist < aaWidth && -dist < rim_glow_depth * pointScale + aaWidth) {
        float g      = 1.0 - smoothstep(0.0, rim_glow_depth * pointScale, -dist);
        float facing = abs(dot(nrm, hairline_dir));
        face += vec3(rim_glow * g * g * mix(0.25, 1.0, facing * facing) * lightGain);
    }

    // 8b. Vetro's hairline, white added along the outline where it meets the light
    if (hairline > 0.0 && edgeless == 0 && dist < aaWidth && -dist < HAIRLINE_WIDTH * pointScale + aaWidth) {
        float edge   = 1.0 - smoothstep(0.0, HAIRLINE_WIDTH * pointScale, -dist);
        float facing = abs(dot(nrm, hairline_dir));
        float lit    = mix(HAIRLINE_FLOOR, 1.0, facing * facing);
        face += vec3(hairline * edge * lit * lightGain);
    }

    // 9. over the shadow layer; the edge's antialiasing is the layer's own
    float aa = drawn;
    outRGB = mix(outRGB, face, aa);
    cover  = max(cover, aa);

    // 10. SDR holding tone
    if (holding_tone_opacity > 0.0) {
        float hold = erfcHalf(dist - sdr_shadow_dist0 * S, sdr_shadow_inv / S);
        float L = dot(outRGB, LUMA709);
        outRGB = mix(outRGB, vec3(L), holding_tone_opacity * hold);
    }

    // 11. the dark rim beside the face: black over what is there, as much as
    //     the bias takes away
    float rim   = rimDark * (1.0 - cover);
    float alpha = (cover + rim) * opacity;
    if (alpha < TINY)
        discard;
    // outRGB is the face weighted by its coverage already (it was mixed over black)
    fragColor = vec4(clamp(outRGB, 0.0, 1.0) * opacity, alpha);
}
