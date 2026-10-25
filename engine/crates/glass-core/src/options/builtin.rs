//! Defaults and the built-in presets, as data.
//!
//! A preset is three layers of settings: `shared`, then `dark` and `light` for
//! what differs between the two appearances. docs/model.md gives the source of
//! each number of the lens model's presets.

use super::ids::{FloatOption as F, IntOption as I};
use super::store::Preset;
use super::values::Values;

/// One setting of a built-in preset.
#[derive(Debug, Clone, Copy)]
enum Setting {
    Float(F, f32),
    Int(I, i64),
}

use Setting::{Float as Fl, Int};

/// What a built-in preset is made of.
struct Recipe {
    name: &'static str,
    /// A built-in preset to start from, by name; copied, not inherited.
    base: Option<&'static str>,
    shared: &'static [Setting],
    dark: &'static [Setting],
    light: &'static [Setting],
}

/// What the classic shader falls back to per theme when nothing sets a value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeDefaults {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub vibrancy: f32,
    pub vibrancy_darkness: f32,
    pub adaptive_dim: f32,
    pub adaptive_boost: f32,
}

/// Fallbacks of the dark appearance.
pub const DARK_THEME_DEFAULTS: ThemeDefaults = ThemeDefaults {
    brightness: 0.82,
    contrast: 0.90,
    saturation: 0.80,
    vibrancy: 0.15,
    vibrancy_darkness: 0.0,
    adaptive_dim: 0.4,
    adaptive_boost: 0.0,
};

/// Fallbacks of the light appearance.
pub const LIGHT_THEME_DEFAULTS: ThemeDefaults = ThemeDefaults {
    brightness: 1.12,
    contrast: 0.92,
    saturation: 0.85,
    vibrancy: 0.12,
    vibrancy_darkness: 0.0,
    adaptive_dim: 0.0,
    adaptive_boost: 0.4,
};

/// What a lens-model option is when no preset and no config level sets it: the
/// regular glass of docs/model.md in its dark appearance. Lengths are logical
/// pixels.
pub mod model_defaults {
    pub const MODEL: i64 = 0;
    pub const BEZEL_WIDTH: f32 = 20.0;
    pub const GLASS_THICKNESS: f32 = 53.0;
    pub const REFRACTIVE_INDEX: f32 = 1.5;
    pub const PROFILE_EXPONENT: f32 = 2.6;
    pub const REFRACTION: f32 = 1.0;
    pub const DISPERSION: f32 = 0.0;
    pub const FROST: f32 = 6.0;
    pub const LIGHT_ANGLE: f32 = 315.0;
    pub const LIGHT_INTENSITY: f32 = 0.5;
    pub const COUNTER_LIGHT: f32 = 1.0;
    pub const RIM_WIDTH: f32 = 1.0;
    pub const RIM_GLOW: f32 = 0.1;
    pub const INNER_SHADOW: f32 = 0.04;
    pub const SHADOW_OPACITY: f32 = 0.12;
    pub const SHADOW_RADIUS: f32 = 40.0;
    pub const SHADOW_OFFSET: f32 = 8.0;
    pub const TINT_OVER_DARK: f32 = 0.76;
    pub const TINT_OVER_BRIGHT: f32 = 0.76;
}

const HIGH_CONTRAST: Recipe = Recipe {
    name: "high_contrast",
    base: None,
    shared: &[
        Fl(F::BlurStrength, 1.2),
        Int(I::BlurIterations, 2),
        Fl(F::LensDistortion, 0.5),
        Fl(F::RefractionStrength, 1.2),
        Fl(F::ChromaticAberration, 0.25),
        Fl(F::FresnelStrength, 0.3),
        Fl(F::SpecularStrength, 0.8),
        Fl(F::GlassOpacity, 1.0),
        Fl(F::EdgeThickness, 0.06),
    ],
    dark: &[
        Fl(F::Brightness, 0.82),
        Fl(F::Contrast, 1.14),
        Fl(F::Saturation, 0.92),
        Fl(F::Vibrancy, 0.5),
        Fl(F::VibrancyDarkness, 0.2),
        Fl(F::AdaptiveDim, 0.25),
        Int(I::TintColor, 0x0214_2aa9),
    ],
    light: &[
        Fl(F::Brightness, 1.0),
        Fl(F::Contrast, 0.92),
        Fl(F::Saturation, 0.8),
        Fl(F::Vibrancy, 0.12),
        Fl(F::VibrancyDarkness, 5.0),
        Fl(F::AdaptiveBoost, 0.15),
        Int(I::TintColor, 0xc2cd_db33),
    ],
};

const SUBTLE: Recipe = Recipe {
    name: "subtle",
    base: None,
    shared: &[
        Fl(F::BlurStrength, 1.0),
        Fl(F::RefractionStrength, 0.3),
        Fl(F::ChromaticAberration, 0.2),
        Fl(F::FresnelStrength, 0.3),
        Fl(F::SpecularStrength, 0.4),
    ],
    dark: &[],
    light: &[],
};

/// The classic shader's own clear preset. It was called "clear" before the
/// lens model took that name for Apple's clear variant.
const CLASSIC_CLEAR: Recipe = Recipe {
    name: "classic_clear",
    base: None,
    shared: &[
        Fl(F::BlurStrength, 0.0),
        Fl(F::NoiseStrength, 0.0),
        Fl(F::RefractionStrength, 0.3),
        Fl(F::ChromaticAberration, 0.2),
        Fl(F::FresnelStrength, 0.3),
        Fl(F::SpecularStrength, 0.4),
    ],
    dark: &[],
    light: &[],
};

const GLASS: Recipe = Recipe {
    name: "glass",
    base: None,
    shared: &[
        Fl(F::BlurStrength, 1.0),
        Int(I::BlurIterations, 2),
        Fl(F::LensDistortion, 0.3),
        Fl(F::RefractionStrength, 8.0),
        Fl(F::ChromaticAberration, 0.5),
        Fl(F::FresnelStrength, 0.4),
        Fl(F::SpecularStrength, 0.8),
        Fl(F::GlassOpacity, 1.0),
        Fl(F::EdgeThickness, 0.06),
        Int(I::TintColor, 0xffff_ff00),
    ],
    dark: &[Fl(F::AdaptiveDim, 0.3)],
    light: &[Fl(F::AdaptiveBoost, 0.3)],
};

/// Apple Liquid Glass look: thin edge lensing along the edges, soft top
/// light, faint bevel.
const POMME: Recipe = Recipe {
    name: "pomme",
    base: None,
    shared: &[
        Fl(F::BlurStrength, 1.0),
        Int(I::BlurIterations, 2),
        Fl(F::RefractionStrength, 2.0),
        Fl(F::RefractionFlow, 1.0),
        Fl(F::RefractionSpread, 1.0),
        Fl(F::ChromaticAberration, 0.0),
        Fl(F::FresnelStrength, 0.1),
        Fl(F::FresnelTint, 0.0),
        Int(I::FresnelColor, 0xffff_ff00),
        Fl(F::SpecularStrength, 0.4),
        Fl(F::SpecularAngle, 0.0),
        Fl(F::BevelStrength, 0.5),
        Fl(F::BevelSize, 4.0),
        Fl(F::BevelTint, 0.5),
        Fl(F::BevelAngle, 315.0),
        Fl(F::BevelShadow, 0.0),
        Fl(F::GlassOpacity, 1.0),
        Fl(F::EdgeThickness, 0.024),
        Fl(F::LensDistortion, 0.0),
    ],
    dark: &[
        Fl(F::Brightness, 1.1),
        Fl(F::Contrast, 0.9),
        Fl(F::Saturation, 1.0),
        Fl(F::Vibrancy, 0.0),
        Fl(F::VibrancyDarkness, 0.0),
        Fl(F::AdaptiveDim, 0.0),
        Fl(F::AdaptiveBoost, 0.0),
        Int(I::TintColor, 0x1a1a_1a4a),
    ],
    light: &[
        Fl(F::Brightness, 0.8),
        Fl(F::Contrast, 1.0),
        Fl(F::Saturation, 1.0),
        Fl(F::Vibrancy, 0.0),
        Fl(F::VibrancyDarkness, 0.0),
        Fl(F::AdaptiveDim, 0.0),
        Fl(F::AdaptiveBoost, 0.0),
        Int(I::TintColor, 0xffff_ff6a),
    ],
};

/// Regular: frosted enough that text behind it cannot be read while shapes
/// and colours come through, with light and dark behind it pressed into a
/// narrow band so that what is written on it keeps its contrast. The default
/// material. The face: `out = (1 - a) * saturated(behind) + a * tint`,
/// measured on iOS 26 as scale 0.238, offset 28.4 / 255 (dark) and scale
/// 0.303, offset 174.3 / 255 (light). The classic settings at the end of each
/// layer are what windows and subsurface items get from the same name: the
/// lens model draws layer surfaces only.
const REGULAR: Recipe = Recipe {
    name: "regular",
    base: None,
    shared: &[
        Int(I::Model, 1),
        Fl(F::BezelWidth, model_defaults::BEZEL_WIDTH),
        Fl(F::GlassThickness, model_defaults::GLASS_THICKNESS),
        Fl(F::RefractiveIndex, model_defaults::REFRACTIVE_INDEX),
        Fl(F::ProfileExponent, model_defaults::PROFILE_EXPONENT),
        Fl(F::Refraction, model_defaults::REFRACTION),
        Fl(F::Dispersion, model_defaults::DISPERSION),
        Fl(F::Frost, model_defaults::FROST),
        Fl(F::LightAngle, model_defaults::LIGHT_ANGLE),
        Fl(F::LightIntensity, model_defaults::LIGHT_INTENSITY),
        Fl(F::CounterLight, model_defaults::COUNTER_LIGHT),
        Fl(F::RimWidth, model_defaults::RIM_WIDTH),
        Fl(F::RimGlow, model_defaults::RIM_GLOW),
        Fl(F::InnerShadow, model_defaults::INNER_SHADOW),
        Fl(F::ShadowOpacity, model_defaults::SHADOW_OPACITY),
        Fl(F::ShadowRadius, model_defaults::SHADOW_RADIUS),
        Fl(F::ShadowOffset, model_defaults::SHADOW_OFFSET),
        Fl(F::Brightness, 1.0),
        Fl(F::Contrast, 1.0),
        Fl(F::NoiseStrength, 0.0),
        Fl(F::GlassOpacity, 1.0),
        // classic equivalents
        Fl(F::BlurStrength, 1.0),
        Int(I::BlurIterations, 2),
        Fl(F::RefractionStrength, 1.2),
        Fl(F::RefractionSpread, 0.1),
        Fl(F::ChromaticAberration, 0.0),
        Fl(F::FresnelStrength, 0.2),
        Fl(F::SpecularStrength, 0.5),
        Fl(F::SpecularAngle, 315.0),
        Fl(F::BevelStrength, 0.6),
        Fl(F::BevelSize, 1.5),
        Fl(F::EdgeThickness, 0.04),
        Fl(F::LensDistortion, 0.0),
    ],
    dark: &[
        Int(I::TintColor, 0x2525_25ff),
        Fl(F::TintOverDark, 0.76),
        Fl(F::TintOverBright, 0.76),
        Fl(F::Saturation, 2.5),
        Fl(F::AdaptiveDim, 0.4),
    ],
    light: &[
        Int(I::TintColor, 0xfafa_faff),
        Fl(F::TintOverDark, 0.70),
        Fl(F::TintOverBright, 0.70),
        Fl(F::Saturation, 1.95),
        Fl(F::AdaptiveBoost, 0.3),
    ],
};

/// Clear: the same lens, barely frosted and barely tinted, for panes that
/// float over photos and video. It does not adapt, except for the dimming
/// Apple asks for behind it where the content is bright (35 percent, Human
/// Interface Guidelines, Materials). Not a default anywhere: text on it is
/// only readable over media that is dark or dimmed.
const CLEAR: Recipe = Recipe {
    name: "clear",
    base: Some("regular"),
    shared: &[
        Fl(F::Frost, 2.0),
        Fl(F::Dispersion, 0.1),
        Fl(F::LightIntensity, 0.4),
        // classic equivalents
        Fl(F::BlurStrength, 0.1),
        Int(I::BlurIterations, 1),
    ],
    dark: &[
        Int(I::TintColor, 0x0000_00ff),
        Fl(F::TintOverDark, 0.18),
        Fl(F::TintOverBright, 0.35),
        Fl(F::Saturation, 1.2),
    ],
    light: &[
        Int(I::TintColor, 0xffff_ffff),
        Fl(F::TintOverDark, 0.07),
        Fl(F::TintOverBright, 0.07),
        Fl(F::Saturation, 1.2),
    ],
};

/// Apple's current regular glass (`crate::material::G27`, measured on iOS 27
/// by `liquid_glass_widgets`; see NOTICE), where `regular` is the first
/// generation. What is behind it comes through twice, nearly sharp and as a
/// wide soft cloud laid over that, so that shapes behind it can still be told
/// while nothing behind it competes with what is written on it. It is far
/// lighter than `regular`: the dark appearance lifts the page by an eighth of
/// white and only pulls down what is bright. Its rim is a dark hairline with
/// a lobe of light at the top and at the bottom, and it bends by the thin-prism
/// law over a narrower rim. The `material` setting hands the engine the numbers
/// of the material that no setting names.
const REGULAR27: Recipe = Recipe {
    name: "regular27",
    base: Some("regular"),
    shared: &[
        // the thin prism: the rim as wide as the slab is thick
        Fl(F::BezelWidth, 10.7),
        Fl(F::GlassThickness, 10.7),
        Fl(F::RefractiveIndex, 1.24),
        Fl(F::Dispersion, 0.0),
        // the ghost; the cloud is frost_wide
        Fl(F::Frost, 0.6),
        Fl(F::FrostWide, 14.0),
        // the first generation's diagonal light, its glow and its inner shadow are not in this one
        Fl(F::LightIntensity, 0.0),
        Fl(F::RimGlow, 0.0),
        Fl(F::InnerShadow, 0.0),
        // the hairline ends two thirds of a point inside the outline
        Fl(F::RimWidth, 0.67),
        Fl(F::RimShade, 1.0),
    ],
    dark: &[
        Int(I::Material, crate::material::OPTION_CURRENT_DARK),
        Int(I::TintColor, 0xffff_ffff),
        Fl(F::TintOverDark, 0.12),
        Fl(F::TintOverBright, 0.12),
        Fl(F::Saturation, 1.4),
        Fl(F::FrostCloud, 0.85),
        Fl(F::BodyShade, 0.28),
        // lit from below
        Fl(F::LightAngle, 180.0),
        Fl(F::RimLight, 1.15),
        Fl(F::ShadowOpacity, 0.0),
    ],
    light: &[
        Int(I::Material, crate::material::OPTION_CURRENT_LIGHT),
        Int(I::TintColor, 0xf8f8_f8ff),
        Fl(F::TintOverDark, 0.53),
        Fl(F::TintOverBright, 0.53),
        Fl(F::Saturation, 2.1),
        Fl(F::FrostCloud, 0.73),
        Fl(F::BodyShade, 0.0),
        // lit from above
        Fl(F::LightAngle, 0.0),
        Fl(F::RimLight, 1.0),
        Fl(F::ShadowOpacity, 0.016),
        Fl(F::ShadowRadius, 10.0),
        Fl(F::ShadowOffset, 7.0),
    ],
};

/// Apple's own glass shader as transcribed (`crate::quartz`, see NOTICE):
/// macOS 27's `glass_background` with the numbers fitted against a real
/// `NSGlassEffectView`. The `material` setting picks the transcription's
/// preset; the engine's own lens, frost and rim settings do not apply to it,
/// and it draws its own shadow where its preset has one, so the engine's is off.
const APPLE: Recipe = Recipe {
    name: "apple",
    base: Some("regular"),
    shared: &[
        Int(I::Material, crate::quartz::OPTION_REGULAR),
        Fl(F::ShadowOpacity, 0.0),
    ],
    dark: &[],
    light: &[],
};

/// The clear style: its matrix solved against a real view on macOS 27.
const APPLE_CLEAR: Recipe = Recipe {
    name: "apple-clear",
    base: Some("apple"),
    shared: &[Int(I::Material, crate::quartz::OPTION_CLEAR)],
    dark: &[],
    light: &[],
};

/// Apple's recipe with macOS 27's light as decoded from `QuartzCore`: the dark
/// rim a point outside the face, the bright layer of a sharp and a soft lobe,
/// and the blur fill that darkens.
const APPLE27: Recipe = Recipe {
    name: "apple27",
    base: Some("apple"),
    shared: &[Int(I::Material, crate::quartz::OPTION_MACOS27)],
    dark: &[],
    light: &[],
};

/// Apple's own material recipe for regular glass, read off the disk
/// (`platformContentGlass.materialrecipe`), in the dark appearance.
const APPLE_RECIPE: Recipe = Recipe {
    name: "apple-recipe",
    base: Some("apple"),
    shared: &[Int(I::Material, crate::quartz::OPTION_RECIPE)],
    dark: &[],
    light: &[],
};

/// Vetro's lens on Apple's colour (`crate::quartz::Kind::Lens`): a light blur,
/// a real bezel at the edge, and a hairline rim.
const LENS: Recipe = Recipe {
    name: "lens",
    base: Some("apple"),
    shared: &[Int(I::Material, crate::quartz::OPTION_LENS)],
    dark: &[],
    light: &[],
};

/// The glass as its user tuned it (`crate::tuning`, `crate::quartz::Kind::Tuned`):
/// the Author's pick until the tuner changes it.
const TUNED: Recipe = Recipe {
    name: "tuned",
    base: Some("apple"),
    shared: &[Int(I::Material, crate::quartz::OPTION_TUNED)],
    dark: &[],
    light: &[],
};

/// Every recipe, a base before the recipes that start from it.
const RECIPES: [&Recipe; 14] = [
    &HIGH_CONTRAST,
    &SUBTLE,
    &CLASSIC_CLEAR,
    &GLASS,
    &POMME,
    &REGULAR,
    &CLEAR,
    &REGULAR27,
    &APPLE,
    &APPLE_CLEAR,
    &APPLE27,
    &APPLE_RECIPE,
    &LENS,
    &TUNED,
];

fn apply(values: &mut Values, settings: &[Setting]) {
    for setting in settings {
        match *setting {
            Setting::Float(option, value) => values.set_float(option, value),
            Setting::Int(option, value) => values.set_int(option, value),
        }
    }
}

fn build(recipe: &Recipe, made: &[Preset]) -> Preset {
    let mut preset = recipe
        .base
        .and_then(|base| made.iter().find(|p| p.name == base))
        .cloned()
        .unwrap_or_default();
    recipe.name.clone_into(&mut preset.name);
    apply(&mut preset.shared, recipe.shared);
    apply(&mut preset.dark, recipe.dark);
    apply(&mut preset.light, recipe.light);
    preset
}

/// The presets that ship with the plugin.
#[must_use]
pub fn builtin_presets() -> Vec<Preset> {
    let mut made: Vec<Preset> = Vec::with_capacity(RECIPES.len());
    for recipe in RECIPES {
        let preset = build(recipe, &made);
        made.push(preset);
    }
    made
}
