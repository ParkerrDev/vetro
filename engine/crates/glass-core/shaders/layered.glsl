// The frost of Apple's current material, and its face: put into the shaders
// that draw the pane where they name it (crate::shaders). It reads LUMA,
// which the shader it is put into declares.

uniform sampler2D sharpTex;   // the page behind this pane as it is, at full resolution
uniform vec2 sharpOrigin;     // box-local device pixels of that copy's top left
uniform vec2 invSharpSize;    // 1 / device pixels it spans
uniform vec2 sharpUvLow;      // centre of its first and last texel
uniform vec2 sharpUvHigh;
uniform float ghostSigma;     // deviation of the narrow copy, device pixels
uniform float ghostWeight;    // how many times more white weighs than black in it
uniform float frostCloud;     // share of the cloud laid over the held ghost
uniform float frostHold;      // the ghost is held within this of the cloud: above it when positive, below when negative

uniform float saturation;
uniform float brightness;
uniform float contrast;
uniform vec3 tintColor;
uniform float tintOverDark;
uniform float tintOverBright;
uniform float adaptiveSaturation; // saturation is this share more over black, less over white
uniform float bodyShade;          // bright content pulled down: rgb *= 1 - this * luma^2

// The page at p, as it is.
vec3 sharpAt(vec2 p) {
    return texture(sharpTex, clamp((p - sharpOrigin) * invSharpSize, sharpUvLow, sharpUvHigh)).rgb;
}

// What is behind the glass at p, where `cloud` is the wide soft copy of it.
// Through this material the page shows twice. The ghost is a nearly sharp
// copy: four filtered reads a deviation out along each diagonal make a
// Gaussian of that deviation, and in their mean a bright one weighs
// ghostWeight times a dark one, so that through dark glass light detail
// reads bolder and dark detail finer. It is held within frostHold of the
// cloud on one side, and the cloud is laid over it at frostCloud: what is
// written behind the glass stays as pale shapes and never competes with
// what is written on it.
vec3 layeredFrost(vec2 p, vec3 cloud) {
    float reach = max(ghostSigma, 0.25);
    vec3  sum   = vec3(0.0);
    float total = 0.0;
    for (int i = 0; i < 4; i++) {
        vec2  corner = vec2(i % 2 == 0 ? -reach : reach, i < 2 ? -reach : reach);
        vec3  texel  = sharpAt(p + corner);
        float weight = mix(1.0, ghostWeight, clamp(dot(texel, LUMA), 0.0, 1.0));
        sum   += weight * texel;
        total += weight;
    }
    vec3 ghost = sum / max(total, 1e-5);
    vec3 held  = frostHold > 0.0 ? max(ghost, cloud - frostHold) : frostHold < 0.0 ? min(ghost, cloud - frostHold) : ghost;
    return mix(held, cloud, frostCloud);
}

// The body shade alone: what the hairline, which is neither tinted nor
// saturated, still takes of the face.
vec3 shadeOf(vec3 color) {
    float luminance = clamp(dot(color, LUMA), 0.0, 1.0);
    return color * (1.0 - bodyShade * luminance * luminance);
}

// The material: colour comes through stronger than light and dark do, then a
// tint by an amount that follows the luminance behind, so that what is written
// on the pane keeps its contrast, then the shade that pulls down what is
// bright. With adaptiveSaturation and bodyShade at 0 this is the face of the
// first generation, to the bit.
vec3 faceOf(vec3 color) {
    float luminance = dot(color, LUMA);
    color = mix(vec3(luminance), color, saturation * (1.0 + adaptiveSaturation * (1.0 - 2.0 * clamp(luminance, 0.0, 1.0))));
    color = (color - 0.5) * contrast + 0.5;
    color *= brightness;
    color = mix(color, tintColor, mix(tintOverDark, tintOverBright, clamp(luminance, 0.0, 1.0)));
    return shadeOf(color);
}
