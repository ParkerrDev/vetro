#version 300 es
precision highp float;

uniform sampler2D tex;        // frosted copy of what lies behind this pane; or, layered, its cloud
uniform sampler2D fieldTex;   // rg inward normal, b blurred coverage
uniform sampler2D maskTex;    // the layer's surface, premultiplied
uniform sampler2D lensTable;  // LensModel::buildLensTable
uniform sampler2D fitTex;     // per cell of the pane map: its pane's short side over four bezel widths, at most 1

uniform vec2 quadOrigin;      // box-local device pixels of the quad's top left
uniform vec2 quadSize;
uniform vec2 boxSize;         // layer box, device pixels
uniform vec2 maskUvScale;     // box uv to surface texture uv
uniform vec2 fieldMargin;     // device pixels of field left and above the box
uniform vec2 invFieldSize;    // 1 / device pixels the field spans
uniform vec2 invFitSize;      // 1 / device pixels the fit texture spans
uniform vec2 frostOrigin;     // box-local device pixels of the frost texture's top left
uniform vec2 invFrostSize;    // 1 / device pixels the frost texture spans
uniform vec2 frostUvLow;      // centre of the first and last frost texel that hold data
uniform vec2 frostUvHigh;

uniform float maskThreshold;  // alpha from which the layer's pixels are pane
uniform float maskFeather;    // alpha range above it over which a faint pane gets its glass
uniform float interiorCoverage; // blurred coverage above which the lens has ended
uniform float solidCoverage;    // blurred coverage above which a pixel is whole pane, well clear of the outline
uniform float outerCoverage;    // blurred coverage of a point a pixel and a half outside the outline
uniform float rimCoverage;      // blurred coverage below which a pixel can be on the rim line

uniform float bezelWidth;       // rim width of the whole lens, device pixels
uniform float thickness;        // slab thickness of the whole lens, device pixels
uniform float thicknessPerFit;  // slab thickness that brings the bend to the centre line of a pane of fit 1
uniform float deviationScale;   // the table's scale times the refraction multiplier
uniform float tableTexels;      // texels in a row of the lens table
uniform int dispersive;         // 1 when red and blue bend differently from green
uniform float opacity;          // glass_opacity times the layer's fade

uniform int faceApplied;        // 1 when the frost already holds the face (it was blurred)
uniform int layered;            // 1 when the frost is the current material's two copies
uniform float noiseStrength;

uniform vec2 lightDirection;    // unit vector from the pane toward the light, y down
uniform float lightIntensity;
uniform float counterLight;
uniform float rimWidth;         // device pixels
uniform float rimGlow;
uniform float innerShadow;

// The current material's rim (crate::material): each term is off at 0.
uniform float rimLight;         // strength of the lobes of light at the two ends of the light's axis
uniform float rimShade;         // strength of the dark hairline on the outline
uniform float rimShadeEnds;     // share of the hairline left at the ends of the axis
uniform float edgeAbsorption;   // how much the rim's glass darkens what it shows
uniform float third;            // device pixels in a third of a point, which the rim's lengths are counted in

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

// The layer's own alpha at a point of its box, 0 outside it.
float alphaAt(vec2 p) {
    if (p.x < 0.0 || p.y < 0.0 || p.x >= boxSize.x || p.y >= boxSize.y)
        return 0.0;
    return texture(maskTex, p / boxSize * maskUvScale).a;
}

vec3 frostAt(vec2 p) {
    vec3 frost = texture(tex, clamp((p - frostOrigin) * invFrostSize, frostUvLow, frostUvHigh)).rgb;
    return layered == 1 ? layeredFrost(p, frost) : frost;
}

void main() {
    vec2  p     = quadOrigin + v_texcoord * quadSize;
    vec4  field = texture(fieldTex, (p + fieldMargin) * invFieldSize);
    float v     = field.b;

    vec2  inward = field.rg * 2.0 - 1.0;
    float length2 = dot(inward, inward);
    inward = length2 > 1e-6 ? inward * inversesqrt(length2) : vec2(0.0);

    // How much of this pixel is pane. Well inside a pane all of it, and far
    // outside none: the layer's own alpha is read only near the outline. There
    // the pixel's alpha over the alpha two pixels further in is the share of the
    // pixel the layer covered when it drew its edge, whatever the alpha of its
    // fill, so the glass ends as smoothly as the layer does. A stroke or a glyph
    // much thinner than the blur stays under a tenth of full coverage, where even
    // the tip of a sharp corner has more: it is drawn on the layer, not a pane.
    //
    // A layer may draw something fainter than a pane around it: a dim over the
    // whole screen behind a menu, or its own soft shadow. That is not pane, so
    // the alpha found beyond the rim line, when it is under the threshold, is
    // taken off both before the share is worked out: the glass then ends on the
    // card's outline, and the dim beside it lies on the bare page.
    float coverage = 0.0;
    float solid    = 1.0;   // the layer's alpha just inside this pixel, for the rim line
    float beside   = 0.0;   // the layer's alpha just outside the pane
    if (v >= solidCoverage) {
        coverage = 1.0;
    } else if (v > 0.09) {
        float own = alphaAt(p);
        if (own >= 0.5 * maskThreshold) {
            solid = max(alphaAt(p + inward * 2.0), own);
            float outside = alphaAt(p - inward * (rimWidth + 1.5));
            beside = outside < maskThreshold ? min(outside, own) : 0.0;
            float share = clamp((own - beside) / max(solid - beside, @ALPHA_SPAN_FLOOR@), 0.0, 1.0);
            if (own >= maskThreshold)
                coverage = smoothstep(0.10, 0.18, v) * share;
            else if (v >= outerCoverage && solid >= maskThreshold)
                coverage = share;
            // a pane whose own fill is faint gets glass in proportion
            if (maskFeather > 0.0)
                coverage *= clamp((solid - maskThreshold) / maskFeather, 0.0, 1.0);
        }
    }

    // Beside the pane there is only its shadow, which shadow.frag has drawn already.
    if (coverage <= 0.0)
        discard;

    // Refraction. In the rim the ray that reaches the eye left the page further
    // inside the pane, by (B h(t) + T) times the tangent of its lean (LensModel.hpp).
    // A pane too small for the whole lens refracts with one that fits it: the rim
    // at most a quarter of its short side, the bend at most to its centre line.
    vec3 color;
    float band = 1.0;
    float fit  = 1.0;
    if (v < interiorCoverage) {
        // how far inside the outline, in bezel widths of the whole lens
        float depth = texture(lensTable, vec2(sqrt(clamp(2.0 * (1.0 - v), 0.0, 1.0)), @LENS_ROW_DEPTH@)).r;
        fit  = max(texture(fitTex, (p + fieldMargin) * invFitSize).r, @FIT_FLOOR@);
        band = clamp(depth / fit, 0.0, 1.0);
    }
    if (band < 1.0) {
        vec4 lens = texture(lensTable, vec2((band * (tableTexels - 1.0) + 0.5) / tableTexels, @LENS_ROW_LEAN@));
        vec3 bend = (bezelWidth * fit * lens.a + min(thickness, thicknessPerFit * fit)) * deviationScale * lens.rgb;
        if (dispersive == 1) {
            color.r = frostAt(p + inward * bend.r).r;
            color.g = frostAt(p + inward * bend.g).g;
            color.b = frostAt(p + inward * bend.b).b;
        } else {
            color = frostAt(p + inward * bend.g);
        }
    } else {
        color = frostAt(p);
    }

    // The face, where the frost pass has not applied it already (frostblur.frag).
    if (faceApplied == 0)
        color = faceOf(color);

    if (noiseStrength > 0.001)
        color += (pixelHash(floor(p)) - 0.5) * noiseStrength * 0.1;

    // The rim. edge is 1 on the outline and falls to 0 a bezel inside it.
    float edge = clamp(2.0 * (1.0 - v), 0.0, 1.0);
    if (edge > @EDGE_FLOOR@) {
        // both walls along the light's axis catch it, the far one by counterLight
        float facing = dot(-inward, lightDirection);
        float lit    = facing * facing * (facing > 0.0 ? 1.0 : counterLight);
        float glow   = edge * edge * edge;
        // a line rimWidth wide: pixels whose neighbour that far outward is no longer pane
        float line   = v < rimCoverage
            ? 1.0 - clamp((alphaAt(p - inward * rimWidth) - beside) / max(max(solid, maskThreshold) - beside, @ALPHA_SPAN_FLOOR@), 0.0, 1.0)
            : 0.0;
        color *= 1.0 - innerShadow * glow;
        color += lightIntensity * lit * (line + rimGlow * glow);

        // The current material's rim, measured against the system's own control
        // (liquid_glass_render.frag of liquid_glass_widgets, see NOTICE). The line is
        // a hairline: the page as it is just outside the pane, neither frosted nor
        // tinted, a fixed step darker across the light's axis, which is how the
        // system's outline stays the same step under any page. Just inside it, at
        // the two ends of the axis, a lobe of light: a sharp core and a soft tail a
        // few points into the body, fainter over a bright body. And the rim's glass
        // itself darkens a little what it shows, more on the far side than the lit.
        if (rimShade > 0.0 || rimLight > 0.0 || edgeAbsorption > 0.0) {
            // the same axis seen from the wall: 1 where the wall faces the light
            float along = facing;
            if (edgeAbsorption > 0.0 && band < 1.0) {
                // the glass of a quarter-circle rim, thick at the outline and gone where the pane is flat
                float rise  = sqrt(max(0.0, 1.0 - (1.0 - band) * (1.0 - band)));
                float glass = 1.0 - sqrt(max(0.0, 1.0 - (1.0 - rise) * (1.0 - rise)));
                color *= max(0.0, 1.0 - sqrt(glass) * edgeAbsorption * mix(1.4, 0.6, along * 0.5 + 0.5));
            }
            float hairline = rimShade > 0.0 ? line : 0.0;
            if (hairline > 0.0)
                color = mix(color, shadeOf(sharpAt(p - inward * (@HAIRLINE_READ@ * third))), hairline);
            if (rimLight > 0.0) {
                float lobe  = pow(max(along, 0.0), 3.0) + @RIM_OPPOSITE@ * pow(max(-along, 0.0), 3.0);
                // device pixels inside the outline, then thirds of a point past the inset.
                // The lobes begin where the hairline has ended: three quarters of a point
                // in, and on a coarse screen no nearer than a pixel past the line's width,
                // so that the brightest of the light is not spent under the line.
                float inside = band * fit * bezelWidth;
                float inset  = max(0.0, inside - max(@RIM_INSET@ * third, rimWidth + 1.0)) / max(third, 1e-3);
                float ease   = pow(1.0 - clamp(dot(color, LUMA), 0.0, 1.0), @RIM_LUMA_POWER@);
                color += rimLight * lobe * ease * (@RIM_CORE@.x * exp(-inset / @RIM_CORE@.y) + @RIM_TAIL@.x * exp(-inset / @RIM_TAIL@.y)) * (1.0 - hairline);
            }
            if (hairline > 0.0) {
                float across = pow(max(1.0 - along * along, 0.0), @HAIRLINE_POWER@);
                color = max(color - rimShade * @HAIRLINE_SHADE@ * mix(rimShadeEnds, 1.0, across) * hairline, 0.0);
            }
        }
    }

    color = clamp(color, 0.0, 1.0);

    // Premultiplied, for Hyprland's (ONE, ONE_MINUS_SRC_ALPHA) blend: where the pane
    // covers a pixel only in part, the shadow drawn before shows through the rest.
    float glassAlpha = coverage * opacity;
    fragColor        = vec4(color * glassAlpha, glassAlpha);
}
