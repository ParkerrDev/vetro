//! The two materials as data: the numbers the glass is made of, by generation,
//! with where each comes from. The `material` option picks the current one
//! ([`by_option`]); `crate::options::model_params` hands its numbers to the
//! passes, and `crate::shaders` writes the constants of its rim into the glass
//! shader.
//!
//! * [`G26`], the engine's current numbers: Apple's Liquid Glass as measured on
//!   iOS 26 and reconstructed in docs/model.md (the same values as the built-in
//!   `regular` and `clear` presets, which a test holds equal).
//! * [`G27`], the iOS 27 material as `liquid_glass_widgets` measured it against
//!   a `SwiftUI` `glassEffect(.regular)` control on the same screen
//!   (`liquid_glass_settings.dart` and `liquid_glass_render.frag`; MIT,
//!   Copyright (c) 2024-2026 Sebastian Degenaar, with the notices of
//!   `THIRD_PARTY_NOTICES`; see NOTICE). Its raw captures are not published, so
//!   treat it as strong but unconfirmed.
//!
//! The numbers of kube.io, spoti.pw and ybouane are not used: their licences
//! allow ideas only. Lengths are points (logical pixels).

use crate::options::builtin::model_defaults;

/// Which generation of Apple's material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    /// iOS 26: the engine's own numbers.
    G26,
    /// iOS 27: `liquid_glass_widgets`.
    G27,
}

/// The tint a face is mixed toward, and how saturated what shows through is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    /// Tint colour, 8 bits a channel.
    pub tint: [u8; 3],
    /// Share of the tint over a black page.
    pub tint_over_dark: f32,
    /// Share of the tint over a white page.
    pub tint_over_bright: f32,
    /// Saturation of what shows through.
    pub saturation: f32,
    /// `rgb *= 1 - k * luma^2`: dark glass pulls a bright backdrop down, so
    /// that a white page comes out at about 184/255 (`bodyShade`,
    /// `liquid_glass_render.frag:684-695`, `liquid_glass_settings.dart:703-706`).
    pub body_shade: Option<f32>,
}

/// How a wide, soft copy of the page is combined with the narrow one: G27's
/// "cloud" and "ghost" (`liquid_glass_render.frag:234-304`, `frostAt`;
/// `liquid_glass_settings.dart:360-440`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WideFrost {
    /// Deviation of the wide copy, the cloud.
    pub sigma: f64,
    /// Whether the held copy is the larger of the ghost and the cloud less an
    /// offset (light glass, `frostClamp > 0`) or the smaller of the ghost and
    /// the cloud plus one (dark glass).
    pub held_is_max: bool,
    /// The offset: 0.4 for light glass, 0.45 for dark.
    pub held_offset: f64,
    /// `frost = mix(held, cloud, this)`: 0.73 light, 0.85 dark.
    pub cloud_opacity: f64,
    /// How much more white weighs than black in the cloud's mean: 2.0 light, 0.5 dark.
    pub cloud_white_weight: f64,
    /// The weight of the ghost: 0.8 light, 2.5 dark.
    pub ghost_weight: f64,
}

/// What is behind the glass, blurred.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frost {
    /// Deviation of the narrow copy (the engine's single blur, or G27's ghost).
    pub narrow_sigma: f64,
    /// The wide copy and how it is combined, where there is one.
    pub wide: Option<WideFrost>,
}

/// The dark outline across the light's axis (`liquid_glass_render.frag:441-460, 870-910`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hairline {
    /// `0.314`: 80 of 255, subtracted, so the step is the same over any backdrop.
    pub shade: f64,
    /// Share of it that stays at the ends of the axis: 0.2 light, 0 dark.
    pub ends: f64,
    /// `across = (1 - along^2)^this`: 1.75.
    pub across_power: f64,
    /// Where the line starts and ends, in thirds of the scale (pixels at 3x): 0.75 and 2.0.
    pub width: [f64; 2],
    /// Where the page is read for it, outward along the normal, in thirds of the scale: 2.5.
    pub read_offset: f64,
}

/// The light on the rim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rim {
    /// Where the light is, degrees clockwise from the top (0 the top, 90 the right, 315 the upper left).
    pub light_angle: f64,
    /// Strength of the lobes: 1.0 light, 1.15 dark. Where this is the engine's
    /// own line, its `light_intensity`.
    pub strength: f64,
    /// `lobe = lit^3 + this * opposite^3`: 0.85. In the engine the far wall
    /// has `counter_light`, 1.
    pub opposite: f64,
    /// `(1 - luma)^this`: the lobe fades over bright bodies, 0.3.
    pub luma_power: f64,
    /// `0.175 exp(-inset / 1.5)`: the core's amplitude, and its decay in thirds
    /// of a point (pixels at 3x, which is what the source measures in).
    pub core: [f64; 2],
    /// `0.035 exp(-inset / 8)`: the tail's amplitude and decay, in the same thirds.
    pub tail: [f64; 2],
    /// Points past the outline before the lobes begin: 0.75.
    pub inset: f64,
    /// The rim's glass darkens what it shows by up to this, 0.6 of it on the
    /// lit side and 1.4 on the far one (`edgeAbsorption`,
    /// `liquid_glass_render.frag:720-731`): 0.035 in both of G27's appearances.
    pub absorption: f64,
    /// The engine's soft light spilling inward and the darkening inside the
    /// outline, which have no measured source; G27 drops them.
    pub glow_and_inner_shadow: Option<[f64; 2]>,
    /// The dark hairline, where the material has one.
    pub hairline: Option<Hairline>,
}

/// The drop shadow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub opacity: f64,
    /// Blur radius, points.
    pub blur: f64,
    /// How far below the pane it falls, points.
    pub offset: f64,
}

/// One appearance of a material.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Appearance {
    pub tone: Tone,
    pub frost: Frost,
    pub rim: Rim,
    pub shadow: Option<Shadow>,
}

/// The thickness, index and law of the lens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LensLaw {
    /// Snell's law on a rim of superellipse profile (docs/model.md): the
    /// engine's lens, which `crate::lens` computes.
    Snell {
        bezel_width: f64,
        thickness: f64,
        refractive_index: f64,
        profile_exponent: f64,
    },
    /// The thin-prism law measured on iOS 27 (`liquid_glass_render.frag:388-415`):
    /// `D(t) = A s^2` with `s = 1 - t` and `A = 8 thickness (n - 1)`, over a
    /// bezel as wide as the thickness. Measured on a 56 pt control: thickness
    /// 10.7 pt, `n` 1.24, so `A` is 20.5 pt, folding from the outline to 0.74
    /// of the bezel. One control size, so it cannot say how the bezel scales.
    Paraxial {
        thickness: f64,
        refractive_index: f64,
        fold: f64,
    },
}

impl LensLaw {
    /// The width of the rim, points.
    #[must_use]
    pub fn bezel_width(&self) -> f64 {
        match *self {
            Self::Snell { bezel_width, .. } => bezel_width,
            Self::Paraxial { thickness, .. } => thickness,
        }
    }

    /// For the paraxial law, how far the page is read from the outline's
    /// ray, `A s^2` at band position `t` (0 on the outline, 1 where the glass
    /// is flat); none for Snell, which `crate::lens::displacement` computes.
    #[must_use]
    pub fn paraxial_displacement(&self, t: f64) -> Option<f64> {
        match *self {
            Self::Paraxial {
                thickness,
                refractive_index,
                ..
            } => {
                let s = 1.0 - t.clamp(0.0, 1.0);
                Some(8.0 * thickness * (refractive_index - 1.0) * s * s)
            }
            Self::Snell { .. } => None,
        }
    }
}

/// The engine's clear variant: the same lens, barely frosted, for panes over
/// photos and video. Apple's guidelines ask for a dimming layer of 35 percent
/// over bright media (Human Interface Guidelines, Materials).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clear {
    pub frost_sigma: f64,
    pub dispersion: f64,
    pub saturation: f32,
    pub light_intensity: f64,
    /// Tint over dark and over bright pages, in the dark appearance (black).
    pub dark_tint: [f32; 2],
    /// The same in the light appearance (white).
    pub light_tint: [f32; 2],
}

/// A material: both appearances and the lens they share.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub name: &'static str,
    pub generation: Generation,
    pub lens: LensLaw,
    /// Reciprocal Abbe number: 0 where the material has no visible dispersion.
    pub dispersion: f64,
    pub light: Appearance,
    pub dark: Appearance,
    /// The clear variant, where the material has one.
    pub clear: Option<Clear>,
}

impl Material {
    /// The appearance asked for.
    #[must_use]
    pub fn appearance(&self, dark: bool) -> &Appearance {
        if dark { &self.dark } else { &self.light }
    }
}

/// The engine's rim: the diagonal light of docs/model.md, both walls lit.
const fn engine_rim() -> Rim {
    Rim {
        light_angle: 315.0,
        strength: 0.5,
        opposite: 1.0,
        luma_power: 0.0,
        core: [0.0, 1.0],
        tail: [0.0, 1.0],
        inset: 0.0,
        absorption: 0.0,
        glow_and_inner_shadow: Some([0.1, 0.04]),
        hairline: None,
    }
}

const fn engine_frost() -> Frost {
    Frost {
        narrow_sigma: 6.0,
        wide: None,
    }
}

const fn engine_shadow() -> Shadow {
    Shadow {
        opacity: model_defaults::SHADOW_OPACITY as f64,
        blur: model_defaults::SHADOW_RADIUS as f64,
        offset: model_defaults::SHADOW_OFFSET as f64,
    }
}

/// iOS 26, as the engine has it. Sources, per docs/model.md: the lens from
/// `flutter_liquid_glass` (measured on iOS: refraction height 20, amount 60),
/// the tone from `CodenameOne`'s `GlassRecipe.java` (measured on iOS 26: output
/// = input * 0.238 + 28.4 dark, * 0.303 + 174.3 light), the frost from
/// `ShatteredGlass` (backdrop at scale 0.25, `inputBlurRadius` 1.5: 6 points),
/// the light and shadow from `ShatteredGlass` and Figma's kit.
pub const G26: Material = Material {
    name: "G26",
    generation: Generation::G26,
    lens: LensLaw::Snell {
        bezel_width: model_defaults::BEZEL_WIDTH as f64,
        thickness: model_defaults::GLASS_THICKNESS as f64,
        refractive_index: model_defaults::REFRACTIVE_INDEX as f64,
        profile_exponent: model_defaults::PROFILE_EXPONENT as f64,
    },
    dispersion: 0.0,
    light: Appearance {
        tone: Tone {
            tint: [0xfa, 0xfa, 0xfa],
            tint_over_dark: 0.70,
            tint_over_bright: 0.70,
            saturation: 1.95,
            body_shade: None,
        },
        frost: engine_frost(),
        rim: engine_rim(),
        shadow: Some(engine_shadow()),
    },
    dark: Appearance {
        tone: Tone {
            tint: [0x25, 0x25, 0x25],
            tint_over_dark: 0.76,
            tint_over_bright: 0.76,
            saturation: 2.5,
            body_shade: None,
        },
        frost: engine_frost(),
        rim: engine_rim(),
        shadow: Some(engine_shadow()),
    },
    clear: Some(Clear {
        frost_sigma: 2.0,
        dispersion: 0.1,
        saturation: 1.2,
        light_intensity: 0.4,
        dark_tint: [0.18, 0.35],
        light_tint: [0.07, 0.07],
    }),
};

const fn g27_rim(light: bool) -> Rim {
    Rim {
        // light material lit from the top, dark from the bottom: both lobes on the vertical axis
        // (`lightAngle = pi/2` in light appearance, `-pi/2` in dark: `liquid_glass_settings.dart:650-715`)
        light_angle: if light { 0.0 } else { 180.0 },
        strength: if light { 1.0 } else { 1.15 },
        opposite: 0.85,
        luma_power: 0.3,
        core: [0.175, 1.5],
        tail: [0.035, 8.0],
        inset: 0.75,
        absorption: 0.035,
        glow_and_inner_shadow: None,
        hairline: Some(g27_hairline(light)),
    }
}

/// G27's dark outline: the same in both appearances but for what is left of
/// it at the ends of the light's axis.
#[must_use]
pub const fn g27_hairline(light: bool) -> Hairline {
    Hairline {
        shade: 0.314,
        ends: if light { 0.2 } else { 0.0 },
        across_power: 1.75,
        width: [0.75, 2.0],
        read_offset: 2.5,
    }
}

const fn g27_frost(light: bool) -> Frost {
    Frost {
        narrow_sigma: 0.6,
        wide: Some(WideFrost {
            sigma: 14.0,
            held_is_max: light,
            held_offset: if light { 0.4 } else { 0.45 },
            cloud_opacity: if light { 0.73 } else { 0.85 },
            cloud_white_weight: if light { 2.0 } else { 0.5 },
            ghost_weight: if light { 0.8 } else { 2.5 },
        }),
    }
}

/// iOS 27, as `liquid_glass_widgets` measured it. Frost: a cloud and a ghost
/// (`liquid_glass_settings.dart:360-440`). Tone: light `#F8F8F8` at 0x87
/// (0.53), saturation 2.1; dark white at 0x1F (0.12), saturation 1.4 with the
/// body shade. Shadow: light one black at 0x04 (1.6 percent), blur 10, offset 7;
/// dark none (`shadowElevation 0`). No dispersion: the iOS 27 presets set
/// `chromaticAberration: 0`. No clear variant in the iOS 27 presets.
pub const G27: Material = Material {
    name: "G27",
    generation: Generation::G27,
    lens: LensLaw::Paraxial {
        thickness: 10.7,
        refractive_index: 1.24,
        fold: 0.74,
    },
    dispersion: 0.0,
    light: Appearance {
        tone: Tone {
            tint: [0xf8, 0xf8, 0xf8],
            tint_over_dark: 0.53,
            tint_over_bright: 0.53,
            saturation: 2.1,
            body_shade: None,
        },
        frost: g27_frost(true),
        rim: g27_rim(true),
        shadow: Some(Shadow {
            opacity: 0.016,
            blur: 10.0,
            offset: 7.0,
        }),
    },
    dark: Appearance {
        tone: Tone {
            tint: [0xff, 0xff, 0xff],
            tint_over_dark: 0.12,
            tint_over_bright: 0.12,
            saturation: 1.4,
            body_shade: Some(0.28),
        },
        frost: g27_frost(false),
        rim: g27_rim(false),
        shadow: None,
    },
    clear: None,
};

/// A material by name (`"G26"` or `"G27"`).
#[must_use]
pub fn by_name(name: &str) -> Option<&'static Material> {
    [&G26, &G27].into_iter().find(|m| m.name == name)
}

/// The `material` option's value for the current material in its light
/// appearance.
pub const OPTION_CURRENT_LIGHT: i64 = 1;
/// The same in its dark appearance.
pub const OPTION_CURRENT_DARK: i64 = 2;

/// What the `material` option names: the current material and one of its
/// appearances, or nothing, which leaves the engine with its own numbers.
#[must_use]
pub fn by_option(value: i64) -> Option<(&'static Material, &'static Appearance)> {
    match value {
        OPTION_CURRENT_LIGHT => Some((&G27, &G27.light)),
        OPTION_CURRENT_DARK => Some((&G27, &G27.dark)),
        _ => None,
    }
}

/// How much the saturation of what shows through follows the page: by this
/// share more over black and less over white (`adaptiveStrength`, `mix(1.2,
/// 0.8, luma)`, `liquid_glass_render.frag:619-636`).
pub const ADAPTIVE_SATURATION: f32 = 0.2;

#[cfg(test)]
mod tests;
