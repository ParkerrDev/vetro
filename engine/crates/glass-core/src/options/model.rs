//! The lens model's parameters: what a preset resolves to once the monitor's
//! scale is known, with every length in device pixels.

use super::builtin::model_defaults as defaults;
use super::ids::{FloatOption as F, IntOption as I};
use super::values::Values;
use crate::lens::Lens;
use crate::material::{self, Appearance, LensLaw, Material};
use crate::num::to_f32;

/// The set of things a pane's glass is made of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelParams {
    /// The slab.
    pub lens: Lens,
    /// Deviation of the frosting Gaussian.
    pub frost: f32,
    /// Degrees clockwise from the top.
    pub light_angle: f32,
    pub light_intensity: f32,
    pub counter_light: f32,
    pub rim_width: f32,
    pub rim_glow: f32,
    pub inner_shadow: f32,
    pub shadow_opacity: f32,
    pub shadow_radius: f32,
    pub shadow_offset: f32,
    pub tint_over_dark: f32,
    pub tint_over_bright: f32,
    pub saturation: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub noise_strength: f32,
    pub glass_opacity: f32,
    /// Red, green, blue of the tint, each 0 to 1.
    pub tint_color: [f32; 3],
    /// What the current material adds; all of it nothing where it is not chosen.
    pub layered: Layered,
    /// Apple's own shader as transcribed (`crate::quartz`), when the
    /// `material` setting names one of its presets.
    pub quartz: Option<crate::quartz::Kind>,
}

/// What Apple's current material adds to the model (`crate::material`): a
/// frost of two copies of the page, a shade over bright content, and a rim of
/// two lobes of light over a dark hairline. Each term is off at 0.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Layered {
    /// Deviation of the wide copy of the page, the cloud, in device pixels; 0
    /// where the frost is the one copy of [`ModelParams::frost`]. With a
    /// cloud, that one is the narrow copy, the ghost.
    pub frost_wide: f32,
    /// Share of the cloud laid over the held ghost.
    pub frost_cloud: f32,
    /// The ghost is held within this of the cloud: no darker than the cloud
    /// less this when positive (light glass), no lighter than the cloud plus
    /// its size when negative (dark glass); 0 leaves it free.
    pub frost_hold: f32,
    /// How many times more white weighs than black in the cloud's mean.
    pub cloud_weight: f32,
    /// The same in the ghost's.
    pub ghost_weight: f32,
    /// `rgb *= 1 - this * luma^2`: bright content pulled down.
    pub body_shade: f32,
    /// Saturation is this share more over black and less over white.
    pub adaptive_saturation: f32,
    /// Strength of the two lobes of light on the rim.
    pub rim_light: f32,
    /// Strength of the dark hairline.
    pub rim_shade: f32,
    /// Share of the hairline left at the two ends of the light's axis.
    pub rim_shade_ends: f32,
    /// How much the rim's glass darkens what it shows.
    pub edge_absorption: f32,
    /// Device pixels in a third of a point: the unit the rim's hairline and
    /// its lobes of light are measured in (pixels at 3x). Never less than
    /// [`MIN_THIRD`]: the line and the core of its light are a fraction of a
    /// point across, and on a coarse screen they would fall between pixels.
    pub third: f32,
}

/// The least a third of a point is taken to be, in device pixels. The rim was
/// measured on a screen of three pixels to the point, where its hairline is
/// two pixels and the core of its light one and a half. At three quarters of
/// a pixel to the third (a screen of 2.25 or less) the hairline is still a
/// whole pixel and the light beside it lands on the next: below that the rim
/// is drawn as if the screen were that fine.
pub const MIN_THIRD: f32 = 0.75;

/// What changed between two sets of parameters, as the rebuilds it asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Changes {
    /// The field, and the shadow built alongside it, depend on something that changed.
    pub field: bool,
    /// The lens table depends on something that changed.
    pub lens: bool,
    /// A pane's frost depends on something that changed.
    pub frost: bool,
}

/// The parameters for a monitor of this scale, from options already resolved.
///
/// An option nothing sets takes the model's default. Lengths are multiplied
/// by `scale`, and values with a range are held to it.
#[must_use]
pub fn model_params(options: &Values, scale: f32) -> ModelParams {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let get = |option: F, fallback: f32| options.float_or(option, fallback);
    let unit = |option: F, fallback: f32| get(option, fallback).clamp(0.0, 1.0);
    let chosen = material::by_option(options.int_or(I::Material, 0));
    let lens = Lens {
        paraxial: chosen
            .is_some_and(|(material, _)| matches!(material.lens, LensLaw::Paraxial { .. })),
        bezel_width: f64::from(get(F::BezelWidth, defaults::BEZEL_WIDTH) * scale),
        thickness: f64::from(get(F::GlassThickness, defaults::GLASS_THICKNESS) * scale),
        refractive_index: f64::from(
            get(F::RefractiveIndex, defaults::REFRACTIVE_INDEX).clamp(1.0, 3.0),
        ),
        profile_exponent: f64::from(
            get(F::ProfileExponent, defaults::PROFILE_EXPONENT).clamp(1.0, 16.0),
        ),
        refraction: f64::from(get(F::Refraction, defaults::REFRACTION)),
        dispersion: f64::from(get(F::Dispersion, defaults::DISPERSION).clamp(0.0, 1.0)),
    };
    ModelParams {
        lens,
        frost: get(F::Frost, defaults::FROST) * scale,
        light_angle: get(F::LightAngle, defaults::LIGHT_ANGLE),
        light_intensity: get(F::LightIntensity, defaults::LIGHT_INTENSITY),
        counter_light: get(F::CounterLight, defaults::COUNTER_LIGHT),
        rim_width: (get(F::RimWidth, defaults::RIM_WIDTH) * scale).max(1.0),
        rim_glow: get(F::RimGlow, defaults::RIM_GLOW),
        inner_shadow: get(F::InnerShadow, defaults::INNER_SHADOW),
        shadow_opacity: unit(F::ShadowOpacity, defaults::SHADOW_OPACITY),
        shadow_radius: get(F::ShadowRadius, defaults::SHADOW_RADIUS) * scale,
        shadow_offset: get(F::ShadowOffset, defaults::SHADOW_OFFSET) * scale,
        tint_over_dark: unit(F::TintOverDark, defaults::TINT_OVER_DARK),
        tint_over_bright: unit(F::TintOverBright, defaults::TINT_OVER_BRIGHT),
        // Shared with the classic shader by name. Its per-theme fallbacks belong to
        // its own look, so here an unset value leaves the colour alone.
        saturation: get(F::Saturation, 1.0),
        brightness: get(F::Brightness, 1.0),
        contrast: get(F::Contrast, 1.0),
        noise_strength: get(F::NoiseStrength, 0.0),
        glass_opacity: unit(F::GlassOpacity, 1.0),
        tint_color: tint_rgb(options.int_or(I::TintColor, 0)),
        layered: layered(options, chosen, scale),
        quartz: crate::quartz::Kind::by_option(options.int_or(I::Material, 0)),
    }
}

/// The current material's terms: each from its option where one is set, and
/// otherwise from the material chosen, or off where none is.
fn layered(options: &Values, chosen: Option<(&Material, &Appearance)>, scale: f32) -> Layered {
    let get = |option: F, fallback: f64| options.float_or(option, to_f32(fallback));
    let appearance = chosen.map(|(_, appearance)| appearance);
    let wide = appearance.and_then(|a| a.frost.wide);
    let rim = appearance.map(|a| a.rim);
    let hairline = rim.and_then(|r| r.hairline);
    Layered {
        frost_wide: get(F::FrostWide, wide.map_or(0.0, |w| w.sigma)).max(0.0) * scale,
        frost_cloud: get(F::FrostCloud, wide.map_or(0.0, |w| w.cloud_opacity)).clamp(0.0, 1.0),
        frost_hold: wide.map_or(0.0, |w| {
            to_f32(if w.held_is_max {
                w.held_offset
            } else {
                -w.held_offset
            })
        }),
        cloud_weight: wide.map_or(1.0, |w| to_f32(w.cloud_white_weight)),
        ghost_weight: wide.map_or(1.0, |w| to_f32(w.ghost_weight)),
        body_shade: get(
            F::BodyShade,
            appearance
                .and_then(|a| a.tone.body_shade)
                .map_or(0.0, f64::from),
        )
        .clamp(0.0, 1.0),
        adaptive_saturation: if chosen.is_some() {
            material::ADAPTIVE_SATURATION
        } else {
            0.0
        },
        rim_light: get(
            F::RimLight,
            if hairline.is_some() {
                rim.map_or(0.0, |r| r.strength)
            } else {
                0.0
            },
        )
        .max(0.0),
        rim_shade: get(F::RimShade, if hairline.is_some() { 1.0 } else { 0.0 }).clamp(0.0, 1.0),
        rim_shade_ends: hairline.map_or(0.0, |h| to_f32(h.ends)),
        edge_absorption: rim.map_or(0.0, |r| to_f32(r.absorption)),
        third: (scale / 3.0).max(MIN_THIRD),
    }
}

/// The red, green and blue of a colour written `0xRRGGBBAA`, each 0 to 1.
fn tint_rgb(tint: i64) -> [f32; 3] {
    let channel = |shift: u32| {
        let byte = tint.checked_shr(shift).unwrap_or(0) & 0xFF;
        f32::from(u8::try_from(byte).unwrap_or(0)) / 255.0
    };
    [channel(24), channel(16), channel(8)]
}

/// True when the options ask for the lens model rather than the classic shader.
#[must_use]
pub fn uses_model(options: &Values) -> bool {
    options.int_or(I::Model, defaults::MODEL) > 0
}

/// Which rebuilds a change from `old` to `new` asks for.
#[must_use]
#[expect(
    clippy::float_cmp,
    reason = "a change of any bit of a setting rebuilds what depends on it: exact comparison is the point"
)]
pub fn changes(old: &ModelParams, new: &ModelParams) -> Changes {
    Changes {
        // the field's blur follows the bezel width; the shadow is built alongside it
        field: old.lens.bezel_width != new.lens.bezel_width
            || old.shadow_radius != new.shadow_radius
            || old.shadow_offset != new.shadow_offset
            || (old.shadow_opacity > 0.0) != (new.shadow_opacity > 0.0),
        lens: old.lens.refractive_index != new.lens.refractive_index
            || old.lens.profile_exponent != new.lens.profile_exponent
            || old.lens.dispersion != new.lens.dispersion
            || old.lens.paraxial != new.lens.paraxial,
        frost: old.frost != new.frost
            || old.layered != new.layered
            || old.saturation != new.saturation
            || old.brightness != new.brightness
            || old.contrast != new.contrast
            || old.tint_color != new.tint_color
            || old.tint_over_dark != new.tint_over_dark
            || old.tint_over_bright != new.tint_over_bright
            || old.quartz != new.quartz,
    }
}
