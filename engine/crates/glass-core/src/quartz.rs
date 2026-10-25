//! Apple's own glass shader, as transcribed: the uniform block of
//! `shaders/quartz27.frag` and the numbers it is given.
//!
//! The shader follows `liquid_glass_27.glsl` of liquid-glass-react-native
//! (<https://github.com/medfa12/liquid-glass-react-native>, MIT, Copyright (c)
//! 2026 liquid-glass contributors; see NOTICE), a transcription of macOS 27's
//! `glass_background_all_lpf` from `QuartzCore`'s `default.metallib` (build
//! 26A5388g): its 63 uniforms in Apple's order and its stages in Apple's order.
//! Its numbers come in three kinds, kept apart because they are not the same
//! kind of claim (`src/params.ts` and `src/glass.ts` there):
//!
//! * [`Kind::Regular`]: fitted by coordinate descent against a real
//!   `NSGlassEffectView` on macOS 26.5.2 (mean error 15/255, by its README).
//! * [`Kind::Recipe`]: Apple's own `platformContentGlass.materialrecipe`, read
//!   off the disk (blur 45 and its colour matrix, the same on macOS 26 and 27),
//!   with the dark appearance's bias measured against a real view.
//! * [`Kind::Clear`]: the clear style's matrix, solved by least squares against
//!   a real `NSGlassEffectView` on macOS 27.0 (residual 0.4/255), and its blur
//!   from the measured width of its edge (63).
//!
//! What the transcription does not claim, and neither does this: it is not
//! pixel identical. Its corner was a fitted superellipse (here the shape is
//! whatever the layer drew, through a distance field of its outline), and the
//! stages new in macOS 27 had modelled numbers. Those are not used. The light
//! of [`Kind::Macos27`] is instead as gpui-glass-demo
//! (<https://github.com/luciascarlet/gpui-glass-demo>, BSD 3-Clause, Copyright
//! (c) 2026 GPUI glass contributors; see NOTICE) decoded it from macOS 27.0's
//! `QuartzCore` (26A428), with the values an `NSGlassEffectView` sets in the
//! dark appearance (its `RESEARCH.md`): the background's key and fill are a
//! dark rim a point outside the face, the bright light is a separate layer of
//! two lobes, a sharp and a soft, through a fixed colour matrix, and the blur
//! fill darkens. Those lengths are points, the same at every size of control.
//!
//! The transcription's lengths were pixels of a panel 240 by 150, whose
//! shorter half is [`SCALE_REF`], and it scaled every length by a pane's
//! shorter half over that: a small bar and a large window were then two
//! different materials. Here every length is a point on every pane, so that
//! all the glass is one material whatever its size.

use crate::tuning::{Id, Tuning};

/// The shorter half of the panel the lengths were fitted on, in pixels.
pub const SCALE_REF: f32 = 75.0;

/// Floats in the uniform block, std140: Apple's 63 fields come to 392 bytes,
/// the engine's own (the bright layer, the hairline, the tint, and the light's
/// direction, the side light and the inner light) follow, and a block is a
/// whole number of 16-byte rows.
pub const BLOCK_FLOATS: usize = 116;

/// Which of the transcription's presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `regular`: the fitted panel.
    Regular,
    /// The clear style, its matrix solved against macOS 27.
    Clear,
    /// Apple's recipe with macOS 27's light as decoded: the dark rim, the
    /// bright layer and the blur fill.
    Macos27,
    /// Apple's material recipe for regular glass in the dark appearance.
    Recipe,
    /// Vetro's lens: Apple's dark colour and bright layer, with the edge of
    /// iOS 26's glass as the references draw it (a circular bezel that bends
    /// what is behind by up to its own width), a light blur, and a hairline rim
    /// lit by the angle of the edge.
    Lens,
    /// The glass as its user tuned it ([`crate::tuning`]), the Author's pick as it comes.
    Tuned,
}

/// The value of the `material` option that names each kind (the current
/// material's are 1 and 2, `crate::material`).
pub const OPTION_REGULAR: i64 = 3;
/// See [`OPTION_REGULAR`].
pub const OPTION_CLEAR: i64 = 4;
/// See [`OPTION_REGULAR`].
pub const OPTION_MACOS27: i64 = 5;
/// See [`OPTION_REGULAR`].
pub const OPTION_RECIPE: i64 = 6;
/// See [`OPTION_REGULAR`].
pub const OPTION_LENS: i64 = 7;
/// See [`OPTION_REGULAR`].
pub const OPTION_TUNED: i64 = 8;

/// The lens's blur, in points: enough to soften what is behind, not to frost it.
pub const LENS_BLUR: f32 = 7.0;
/// How far the lens's face goes toward its dark tint, before the adaptive part.
pub const LENS_TINT: f32 = 0.30;
/// How much the lens saturates what is behind it. 1.7 until 2026-10-18 evening,
/// when "the colors look weird and too different" from the wallpaper behind;
/// the user picked 1.3 of 1.7, 1.3 and 1.0.
pub const LENS_SATURATION: f32 = 1.3;
/// How bright the soft light inside the lens's lit edges is.
pub const LENS_GLOW: f32 = 0.16;

impl Kind {
    /// The kind a `material` option names, if it names one of these.
    #[must_use]
    pub fn by_option(value: i64) -> Option<Self> {
        match value {
            OPTION_REGULAR => Some(Self::Regular),
            OPTION_CLEAR => Some(Self::Clear),
            OPTION_MACOS27 => Some(Self::Macos27),
            OPTION_RECIPE => Some(Self::Recipe),
            OPTION_LENS => Some(Self::Lens),
            OPTION_TUNED => Some(Self::Tuned),
            _ => None,
        }
    }

    /// The number the parameters carry across the C boundary; 0 is none.
    #[must_use]
    pub fn code(self) -> u32 {
        match self {
            Self::Regular => 1,
            Self::Clear => 2,
            Self::Macos27 => 3,
            Self::Recipe => 4,
            Self::Lens => 5,
            Self::Tuned => 6,
        }
    }

    /// The kind a code names.
    #[must_use]
    pub fn from_code(code: u32) -> Option<Self> {
        match code {
            1 => Some(Self::Regular),
            2 => Some(Self::Clear),
            3 => Some(Self::Macos27),
            4 => Some(Self::Recipe),
            5 => Some(Self::Lens),
            6 => Some(Self::Tuned),
            _ => None,
        }
    }
}

/// The shader's uniforms, by Apple's names (`params.ts`'s values, renamed to
/// the 27 fields as `params27.generated.ts` lays them out).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    pub displacement_mat: [f32; 4],
    pub inner_refraction_amount: f32,
    pub inner_refraction_inv_height: f32,
    pub outer_refraction_amount: f32,
    pub outer_refraction_inv_height: f32,
    pub refraction_threshold: [f32; 2],
    pub blur_radius: f32,
    pub edge_bleed_blur_radius: f32,
    pub edge_bleed_amount: f32,
    pub edge_bleed_inv_height: f32,
    pub shadow_amount: f32,
    pub shadow_inv_height: f32,
    pub shadow_offset: [f32; 2],
    pub shadow_blur_radius: f32,
    pub shadow_inv_radius: f32,
    pub face_cm: [[f32; 4]; 3],
    pub bleed_cm: [[f32; 4]; 3],
    pub shadow_cm: [[f32; 4]; 3],
    pub shadow_contribution: f32,
    pub shadow_face_opacity: f32,
    pub blur_alpha: [f32; 4],
    pub blur_dist: [f32; 4],
    pub edge_bleed_dist: [f32; 2],
    pub edge_bleed_opacity: f32,
    pub face_opacity: f32,
    pub bleed_darken: [f32; 2],
    pub shadow_dist_offset: f32,
    pub shadow_opacity: f32,
    pub refraction_opacity: f32,
    pub holding_tone_opacity: f32,
    pub sdr_shadow_dist0: f32,
    pub sdr_shadow_inv: f32,
    pub ring_shadow_offset: [f32; 2],
    pub ring_shadow_stroke_width: f32,
    pub ring_shadow_radius: f32,
    pub ring_shadow_opacity: f32,
    pub ring_shadow_mask: f32,
    pub key_fill_highlight_dir: [f32; 2],
    pub key_fill_highlight_height: f32,
    pub key_fill_highlight_spread: f32,
    pub key_fill_highlight_amount: f32,
    pub key_fill_highlight_effect_offset: f32,
    pub key_fill_highlight_color_bias: f32,
    pub blur_fill_blur_radius: f32,
    pub blur_fill_lighten_opacity: f32,
    pub blur_fill_darken_opacity: f32,
    pub blur_fill_normal_opacity: f32,
    pub aberration_amount: f32,
    pub aberration_dir: [f32; 2],
    /// Not Apple's: 1 where the bright layer of macOS 27 is drawn over the face
    /// (the sharp and soft key and fill lobes through `vibrantColorMatrix`).
    pub bright_layer: f32,
    /// Not Apple's: how much white the outline's hairline adds where it faces
    /// the light, 0 for none.
    pub hairline: f32,
    /// Not Apple's: a dark glass colour the face is taken toward, so that text
    /// on it reads, by `tint_amount`, and by `tint_adaptive` more for each step
    /// of brightness behind it.
    pub tint: [f32; 3],
    pub tint_amount: f32,
    pub tint_adaptive: f32,
    /// Not Apple's: how much the colours behind are saturated before the tint
    /// (1 keeps them; 0 is read as 1, for a block that does not set it).
    pub saturation: f32,
    /// Not Apple's: a soft light just inside the edge, brightest where the
    /// edge faces the light, and how deep it reaches, in points.
    pub rim_glow: f32,
    pub rim_glow_depth: f32,
    /// Not Apple's: where the bright layer's key light comes from, a unit vector
    /// in the pane's own space, y down (macOS 27: straight above, `[0, -1]`).
    pub light_dir: [f32; 2],
    /// Not Apple's: the light the hairline and the glow face (Vetro's lens: the
    /// top left).
    pub hairline_dir: [f32; 2],
    /// Not Apple's: how much of the key light the edges across it catch.
    pub side_light: f32,
    /// Not Apple's: the bright layer's colour matrix over the whole face by this
    /// much, the light glass gives off over a dark backdrop (iOS 27).
    pub lift: f32,
    /// Not Apple's: how far the whole of a window's face bends what is behind
    /// it, as a thick sheet curved over its face: nothing at the centre, and at
    /// the edge a share of the half size (`BODY_PUSH` in the shader) times this.
    pub body_bend: f32,
}

const IDENTITY: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

/// `defaultParams()` of `params.ts`: the fitted regular panel.
const REGULAR: Block = Block {
    displacement_mat: [1.0, 0.0, 0.0, 1.0],
    // the two lobes bend opposite ways: matching signs would make a plain lens
    inner_refraction_amount: 4.0,
    inner_refraction_inv_height: 1.0 / 18.0,
    outer_refraction_amount: -13.0,
    outer_refraction_inv_height: 1.0 / 10.0,
    refraction_threshold: [-30.0, 0.0],
    // an effective Gaussian deviation of about 8, read off the mip chain
    blur_radius: 28.0,
    edge_bleed_blur_radius: 32.0,
    edge_bleed_amount: 24.0,
    edge_bleed_inv_height: 1.0 / 12.0,
    shadow_amount: 10.0,
    shadow_inv_height: 1.0 / 20.0,
    shadow_offset: [0.0, 0.004],
    shadow_blur_radius: 0.0,
    shadow_inv_radius: 1.0 / 26.0,
    // fitted; the off-diagonals match Apple's shipped recipe to within 0.008
    face_cm: [
        [0.3545, -0.1604, -0.0150, 0.1283],
        [-0.0368, 0.2329, -0.0158, 0.1297],
        [-0.0438, -0.1498, 0.3705, 0.1263],
    ],
    bleed_cm: IDENTITY,
    shadow_cm: [
        [0.2, 0.0, 0.0, 0.0],
        [0.0, 0.2, 0.0, 0.0],
        [0.0, 0.0, 0.2, 0.0],
    ],
    shadow_contribution: 0.5,
    shadow_face_opacity: 0.0,
    blur_alpha: [1.0, 0.6, 0.3, 0.0],
    blur_dist: [0.0, 8.0, 20.0, 40.0],
    edge_bleed_dist: [0.0, 14.0],
    edge_bleed_opacity: 0.15,
    face_opacity: 1.0,
    bleed_darken: [0.92, 0.0],
    shadow_dist_offset: 6.0,
    shadow_opacity: 0.0,
    refraction_opacity: 0.65,
    holding_tone_opacity: 0.0,
    sdr_shadow_dist0: 0.0,
    sdr_shadow_inv: 0.0,
    ring_shadow_offset: [0.0, 0.0],
    ring_shadow_stroke_width: 9.0,
    ring_shadow_radius: 30.0,
    ring_shadow_opacity: 0.0,
    ring_shadow_mask: 1.0,
    key_fill_highlight_dir: [0.35, -0.94],
    key_fill_highlight_height: 70.0,
    key_fill_highlight_spread: 0.55,
    key_fill_highlight_amount: 0.0,
    key_fill_highlight_effect_offset: 4.0,
    key_fill_highlight_color_bias: 0.0,
    blur_fill_blur_radius: 48.0,
    blur_fill_lighten_opacity: 0.0,
    blur_fill_darken_opacity: 0.0,
    blur_fill_normal_opacity: 0.0,
    aberration_amount: 3.0,
    aberration_dir: [1.0, 0.0],
    bright_layer: 0.0,
    hairline: 0.0,
    tint: [0.0, 0.0, 0.0],
    tint_amount: 0.0,
    tint_adaptive: 0.0,
    saturation: 1.0,
    rim_glow: 0.0,
    rim_glow_depth: 0.0,
    light_dir: [0.0, -1.0],
    hairline_dir: [
        -core::f32::consts::FRAC_1_SQRT_2,
        -core::f32::consts::FRAC_1_SQRT_2,
    ],
    side_light: 0.5,
    lift: 0.0,
    body_bend: 0.0,
};

/// `platformContentGlass.materialrecipe`'s colour matrix for the dark
/// appearance, with the bias a real darkAqua view measured (0.235 x 0.346).
const RECIPE_DARK_CM: [[f32; 4]; 3] = [
    [0.921, -0.265, -0.027, 0.0813],
    [-0.079, 0.735, -0.027, 0.0813],
    [-0.079, -0.265, 0.973, 0.0813],
];

/// The colour bias of the dark ring macOS 27 draws a point outside its glass,
/// as an `NSGlassEffectView` sets it: a tuning's dark edge of one half.
const DARK_EDGE_BIAS: f32 = -0.1875;

/// What every tuning starts from: no stage that is not one of its knobs. The
/// dark ring is macOS 27's as decoded (a point wide, a point outside the face,
/// lobes about the horizontal with a spread of 120 degrees, so darkest on the
/// sides), with its strength the tuning's; no bleed, shadow, blur fill or outer
/// lens, so that the page read around a pane is no wider than the knobs make it.
const TUNED_BASE: Block = Block {
    outer_refraction_amount: 0.0,
    outer_refraction_inv_height: 0.0,
    refraction_opacity: 0.0,
    edge_bleed_blur_radius: 0.0,
    edge_bleed_amount: 0.0,
    edge_bleed_inv_height: 0.0,
    edge_bleed_opacity: 0.0,
    shadow_amount: 0.0,
    shadow_inv_height: 0.0,
    shadow_offset: [0.0, 0.0],
    shadow_blur_radius: 0.0,
    shadow_inv_radius: 0.0,
    shadow_opacity: 0.0,
    shadow_contribution: 0.0,
    ring_shadow_opacity: 0.0,
    key_fill_highlight_dir: [1.0, 0.0],
    key_fill_highlight_height: 1.0,
    key_fill_highlight_spread: -0.5,
    key_fill_highlight_amount: 0.0,
    key_fill_highlight_effect_offset: -1.0,
    key_fill_highlight_color_bias: 0.0,
    blur_fill_blur_radius: 0.0,
    face_cm: IDENTITY,
    ..REGULAR
};

/// Each entry of `from` taken toward `to` by `t`.
fn blend_cm(from: &[[f32; 4]; 3], to: &[[f32; 4]; 3], t: f32) -> [[f32; 4]; 3] {
    let mut out = *from;
    for (row, (a, b)) in out.iter_mut().zip(from.iter().zip(to.iter())) {
        for (slot, (x, y)) in row.iter_mut().zip(a.iter().zip(b.iter())) {
            *slot = x + (y - x) * t;
        }
    }
    out
}

impl Block {
    /// The block of a tuning: the glass as its user set it. Lengths stay points,
    /// as every length of the shader is. Without a lens there is no colour
    /// fringe either: the fringe follows the lens's own curve.
    #[must_use]
    pub fn tuned(tuning: &Tuning) -> Self {
        let depth = tuning.get(Id::Lens);
        let lensed = depth >= 0.5;
        let angle = tuning.get(Id::LightAngle).to_radians();
        let light = [libm::sinf(angle), -libm::cosf(angle)];
        Self {
            inner_refraction_amount: if lensed {
                depth * tuning.get(Id::Bend)
            } else {
                0.0
            },
            inner_refraction_inv_height: if lensed { 1.0 / depth } else { 0.0 },
            blur_radius: tuning.get(Id::Frost),
            face_cm: blend_cm(&IDENTITY, &RECIPE_DARK_CM, tuning.get(Id::AppleColour)),
            key_fill_highlight_color_bias: DARK_EDGE_BIAS * 2.0 * tuning.get(Id::DarkEdge),
            aberration_amount: if lensed {
                tuning.get(Id::Dispersion)
            } else {
                0.0
            },
            bright_layer: tuning.get(Id::RimLight),
            hairline: tuning.get(Id::Hairline),
            tint: tuning.tint_colour(),
            tint_amount: tuning.get(Id::Tint),
            tint_adaptive: tuning.get(Id::Adaptive),
            saturation: tuning.get(Id::Saturation),
            rim_glow: tuning.get(Id::Glow),
            rim_glow_depth: tuning.get(Id::GlowDepth),
            light_dir: light,
            hairline_dir: light,
            side_light: tuning.get(Id::SideLight),
            lift: tuning.get(Id::Lift),
            body_bend: tuning.get(Id::BodyBend),
            ..TUNED_BASE
        }
    }

    /// The numbers of a preset.
    #[must_use]
    pub fn of(kind: Kind) -> Self {
        match kind {
            Kind::Regular => REGULAR,
            // `Glass.clear`, dark: CLEAR_CM_DARK, decoded on macOS 27.0 (26A5388g);
            // the blur is regular's times the measured ratio of their edges,
            // 19.6 to 13.9 pixels of deviation, 45 x 1.41
            Kind::Clear => Self {
                face_cm: [
                    [0.8297, -0.0967, -0.0183, 0.0967],
                    [-0.0493, 0.7821, -0.0179, 0.0965],
                    [-0.0496, -0.0967, 0.8611, 0.0967],
                ],
                blur_radius: 63.0,
                ..REGULAR
            },
            // `GLASS_RECIPE_DARK`: platformContentGlass.materialrecipe's blur and
            // matrix, with the bias a real darkAqua view measured (0.235 x 0.346)
            Kind::Recipe => Self {
                face_cm: RECIPE_DARK_CM,
                blur_radius: 45.0,
                ..REGULAR
            },
            // The recipe, with what an NSGlassEffectView sets on macOS 27.0 in the
            // dark appearance, but for the dark rim (below). The background's key and fill: height 1 point, offset
            // -1, spread 120 degrees (cos -0.5), CA amount 0.5 (rational amount
            // 1/0.5 - 2 = 0), colour bias -0.1875, lit along the vertical. Here the
            // key and fill fields hold those decoded values: lengths in points, the
            // spread as its cosine, the amount as the shader's. The blur fill: radius
            // 8 points, darken 0.675. The bright layer's own values are the shader's
            // constants (quartz27.frag, brightLayer).
            Kind::Macos27 => Self {
                key_fill_highlight_dir: [0.0, -1.0],
                key_fill_highlight_height: 1.0,
                key_fill_highlight_spread: -0.5,
                key_fill_highlight_amount: 0.0,
                key_fill_highlight_effect_offset: -1.0,
                // The decoded bias is -0.1875: a dark line a point outside every
                // edge. Lit only at the top and the bottom, the sides were left
                // with that line alone, which read as a dark outline (2026-10-18:
                // "they look weird and kinda have like a dark outline"), so the
                // rim is off and the edge is the bright layer's.
                key_fill_highlight_color_bias: 0.0,
                blur_fill_blur_radius: 8.0,
                blur_fill_darken_opacity: 0.675,
                bright_layer: 1.0,
                ..Self::of(Kind::Recipe)
            },
            // 2026-10-18: the glass was "too blurry or frosted", had "no real lens",
            // and its "edges and rim look off". The bezel is a quarter circle 28
            // points deep that moves what is behind outward by up to 28 points at
            // the very edge, so the backdrop beyond the pane bends into its rim, as
            // Kyant0's AndroidLiquidGlass and kube.io's bezel do (the shader keeps
            // it within the pane's own size). One lens, not two crossed ones, no
            // bleed band and no blur fill: those were the frost.
            Kind::Lens => Self {
                inner_refraction_amount: 28.0,
                inner_refraction_inv_height: 1.0 / 28.0,
                refraction_opacity: 0.0,
                blur_radius: LENS_BLUR,
                edge_bleed_opacity: 0.0,
                aberration_amount: 4.0,
                bright_layer: 1.0,
                hairline: 0.6,
                // 2026-10-18, of the lens without it: "i have trouble seeing text, the
                // contrast is bad ... maybe a dark tint?"
                tint: [0.045, 0.05, 0.06],
                tint_amount: LENS_TINT,
                tint_adaptive: 0.5,
                // 2026-10-18, of the tinted lens: "still doesn't quite look enough like
                // liquid glass". What the calibrated iOS 26 and 27 materials have and it
                // did not (liquid_glass_widgets' presets): colour, saturated about 1.5
                // times, and light on the glass, a soft glow inside the lit edges.
                saturation: LENS_SATURATION,
                rim_glow: LENS_GLOW,
                rim_glow_depth: 10.0,
                ..Self::of(Kind::Recipe)
            },
            Kind::Tuned => Self::tuned(&crate::tuning::Tuning::DEFAULT),
        }
    }

    /// The block as the shader's std140 layout has it, offset for offset
    /// (`OFFSETS27` of `params27.generated.ts`, in floats).
    #[must_use]
    pub fn std140(&self) -> [f32; BLOCK_FLOATS] {
        let mut out = [0.0_f32; BLOCK_FLOATS];
        for (at, values) in self.head().iter().chain(self.matrices().iter()) {
            put(&mut out, *at, values);
        }
        for (at, values) in self.tail().into_iter().chain(self.engine()) {
            put(&mut out, at, &values);
        }
        out
    }

    /// Refraction, blur, bleed and shadow lengths: floats 0 to 19.
    fn head(&self) -> [(usize, Vec<f32>); 4] {
        [
            (0, self.displacement_mat.to_vec()),
            (
                4,
                vec![
                    self.inner_refraction_amount,
                    self.inner_refraction_inv_height,
                    self.outer_refraction_amount,
                    self.outer_refraction_inv_height,
                    self.refraction_threshold[0],
                    self.refraction_threshold[1],
                    self.blur_radius,
                    self.edge_bleed_blur_radius,
                    self.edge_bleed_amount,
                    self.edge_bleed_inv_height,
                    self.shadow_amount,
                    self.shadow_inv_height,
                ],
            ),
            (16, self.shadow_offset.to_vec()),
            (18, vec![self.shadow_blur_radius, self.shadow_inv_radius]),
        ]
    }

    /// The three colour matrices, a row of four floats each: floats 20 to 55.
    fn matrices(&self) -> [(usize, Vec<f32>); 9] {
        let [f0, f1, f2] = self.face_cm;
        let [b0, b1, b2] = self.bleed_cm;
        let [s0, s1, s2] = self.shadow_cm;
        [
            (20, f0.to_vec()),
            (24, f1.to_vec()),
            (28, f2.to_vec()),
            (32, b0.to_vec()),
            (36, b1.to_vec()),
            (40, b2.to_vec()),
            (44, s0.to_vec()),
            (48, s1.to_vec()),
            (52, s2.to_vec()),
        ]
    }

    /// Apple's fields after the matrices: floats 56 to 97.
    fn tail(&self) -> [(usize, Vec<f32>); 10] {
        [
            (56, vec![self.shadow_contribution, self.shadow_face_opacity]),
            (58, self.blur_alpha.to_vec()),
            (62, self.blur_dist.to_vec()),
            (66, self.edge_bleed_dist.to_vec()),
            (68, vec![self.edge_bleed_opacity, self.face_opacity]),
            (70, self.bleed_darken.to_vec()),
            (
                72,
                vec![
                    self.shadow_dist_offset,
                    self.shadow_opacity,
                    self.refraction_opacity,
                    self.holding_tone_opacity,
                    self.sdr_shadow_dist0,
                    self.sdr_shadow_inv,
                    self.ring_shadow_offset[0],
                    self.ring_shadow_offset[1],
                    self.ring_shadow_stroke_width,
                    self.ring_shadow_radius,
                    self.ring_shadow_opacity,
                    self.ring_shadow_mask,
                ],
            ),
            (84, self.key_fill_highlight_dir.to_vec()),
            (
                86,
                vec![
                    self.key_fill_highlight_height,
                    self.key_fill_highlight_spread,
                    self.key_fill_highlight_amount,
                    self.key_fill_highlight_effect_offset,
                    self.key_fill_highlight_color_bias,
                    self.blur_fill_blur_radius,
                    self.blur_fill_lighten_opacity,
                    self.blur_fill_darken_opacity,
                    self.blur_fill_normal_opacity,
                    self.aberration_amount,
                ],
            ),
            (96, self.aberration_dir.to_vec()),
        ]
    }

    /// The engine's own fields: floats 98 to 115.
    fn engine(&self) -> [(usize, Vec<f32>); 5] {
        [
            (98, vec![self.bright_layer, self.hairline]),
            (
                100,
                vec![self.tint[0], self.tint[1], self.tint[2], self.tint_amount],
            ),
            (
                104,
                vec![
                    self.tint_adaptive,
                    self.saturation,
                    self.rim_glow,
                    self.rim_glow_depth,
                ],
            ),
            (
                108,
                vec![
                    self.light_dir[0],
                    self.light_dir[1],
                    self.hairline_dir[0],
                    self.hairline_dir[1],
                ],
            ),
            (112, vec![self.side_light, self.lift, self.body_bend, 0.0]),
        ]
    }
}

/// Writes `values` into `out` from float `at` on, as far as there is room.
fn put(out: &mut [f32; BLOCK_FLOATS], at: usize, values: &[f32]) {
    for (slot, value) in out.iter_mut().skip(at).zip(values) {
        *slot = *value;
    }
}

/// The furthest a pane's glass reads the page beyond its own outline, in the
/// fitted panel's pixels: the largest displacement any stage makes, and the
/// reach of the widest blur (a mip read spreads about a radius each way).
#[must_use]
pub fn reach(block: &Block) -> f32 {
    let shifts = [
        block.inner_refraction_amount.abs(),
        block.outer_refraction_amount.abs(),
        block.edge_bleed_amount.abs(),
        block.shadow_amount.abs(),
        block.aberration_amount.abs(),
    ];
    let blurs = [
        block.blur_radius,
        block.edge_bleed_blur_radius,
        block.blur_fill_blur_radius,
    ];
    let shift = shifts.iter().fold(0.0_f32, |a, b| a.max(*b));
    let blur = blurs.iter().fold(0.0_f32, |a, b| a.max(*b));
    shift + blur
}

#[cfg(test)]
mod tests;
