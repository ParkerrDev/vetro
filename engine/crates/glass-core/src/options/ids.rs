//! Every option the plugin knows, by kind and by key, listed once.
//!
//! The first 26 float options and the first five int options belong to the
//! classic shader; the rest are the lens model's (docs/model.md), the last
//! five floats and the last int those of Apple's current material
//! (`crate::material`). All of them exist at three levels (global, `dark:`,
//! `light:`) and in presets.

/// Value of an option that is not set at a level: every option is zero or more
/// when set, so a negative number means "inherit".
pub const UNSET_FLOAT: f32 = -1.0;
/// The same for an int option.
pub const UNSET_INT: i64 = -1;

/// How many float options there are.
pub const FLOAT_COUNT: usize = 49;
/// How many int options there are.
pub const INT_COUNT: usize = 7;

/// An option whose value is a number with a fraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FloatOption {
    BlurStrength = 0,
    NoiseStrength,
    RefractionStrength,
    ChromaticAberration,
    FresnelStrength,
    SpecularStrength,
    SpecularAngle,
    GlassOpacity,
    EdgeThickness,
    LensDistortion,
    Brightness,
    Contrast,
    Saturation,
    Vibrancy,
    VibrancyDarkness,
    AdaptiveDim,
    AdaptiveBoost,
    RefractionFlow,
    RefractionSpread,
    FresnelTint,
    BevelStrength,
    BevelSize,
    BevelTint,
    BevelAngle,
    BevelShadow,
    SelfSample,
    BezelWidth,
    GlassThickness,
    RefractiveIndex,
    ProfileExponent,
    Refraction,
    Dispersion,
    Frost,
    LightAngle,
    LightIntensity,
    CounterLight,
    RimWidth,
    RimGlow,
    InnerShadow,
    ShadowOpacity,
    ShadowRadius,
    ShadowOffset,
    TintOverDark,
    TintOverBright,
    FrostWide,
    FrostCloud,
    BodyShade,
    RimLight,
    RimShade,
}

/// An option whose value is a whole number (a colour is one: `0xRRGGBBAA`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IntOption {
    BlurIterations = 0,
    TintColor,
    FresnelColor,
    BevelColor,
    Xray,
    Model,
    Material,
}

impl FloatOption {
    /// Every float option, in the order of its index.
    pub const ALL: [Self; FLOAT_COUNT] = [
        Self::BlurStrength,
        Self::NoiseStrength,
        Self::RefractionStrength,
        Self::ChromaticAberration,
        Self::FresnelStrength,
        Self::SpecularStrength,
        Self::SpecularAngle,
        Self::GlassOpacity,
        Self::EdgeThickness,
        Self::LensDistortion,
        Self::Brightness,
        Self::Contrast,
        Self::Saturation,
        Self::Vibrancy,
        Self::VibrancyDarkness,
        Self::AdaptiveDim,
        Self::AdaptiveBoost,
        Self::RefractionFlow,
        Self::RefractionSpread,
        Self::FresnelTint,
        Self::BevelStrength,
        Self::BevelSize,
        Self::BevelTint,
        Self::BevelAngle,
        Self::BevelShadow,
        Self::SelfSample,
        Self::BezelWidth,
        Self::GlassThickness,
        Self::RefractiveIndex,
        Self::ProfileExponent,
        Self::Refraction,
        Self::Dispersion,
        Self::Frost,
        Self::LightAngle,
        Self::LightIntensity,
        Self::CounterLight,
        Self::RimWidth,
        Self::RimGlow,
        Self::InnerShadow,
        Self::ShadowOpacity,
        Self::ShadowRadius,
        Self::ShadowOffset,
        Self::TintOverDark,
        Self::TintOverBright,
        Self::FrostWide,
        Self::FrostCloud,
        Self::BodyShade,
        Self::RimLight,
        Self::RimShade,
    ];

    /// The name of each option in a config file, by index.
    const KEYS: [&'static str; FLOAT_COUNT] = [
        "blur_strength",
        "noise_strength",
        "refraction_strength",
        "chromatic_aberration",
        "fresnel_strength",
        "specular_strength",
        "specular_angle",
        "glass_opacity",
        "edge_thickness",
        "lens_distortion",
        "brightness",
        "contrast",
        "saturation",
        "vibrancy",
        "vibrancy_darkness",
        "adaptive_dim",
        "adaptive_boost",
        "refraction_flow",
        "refraction_spread",
        "fresnel_tint",
        "bevel_strength",
        "bevel_size",
        "bevel_tint",
        "bevel_angle",
        "bevel_shadow",
        "self_sample",
        "bezel_width",
        "glass_thickness",
        "refractive_index",
        "profile_exponent",
        "refraction",
        "dispersion",
        "frost",
        "light_angle",
        "light_intensity",
        "counter_light",
        "rim_width",
        "rim_glow",
        "inner_shadow",
        "shadow_opacity",
        "shadow_radius",
        "shadow_offset",
        "tint_over_dark",
        "tint_over_bright",
        "frost_wide",
        "frost_cloud",
        "body_shade",
        "rim_light",
        "rim_shade",
    ];

    /// The value the global level is registered with in Hyprland's config, by
    /// index; negative where the option is not set at the global level, so that
    /// the theme's own default (or the model's) applies.
    const GLOBAL_DEFAULTS: [f64; FLOAT_COUNT] = [
        2.0,   // blur_strength
        0.0,   // noise_strength
        0.6,   // refraction_strength
        0.5,   // chromatic_aberration
        0.6,   // fresnel_strength
        0.8,   // specular_strength
        0.0,   // specular_angle
        1.0,   // glass_opacity
        0.06,  // edge_thickness
        0.5,   // lens_distortion
        -1.0,  // brightness
        -1.0,  // contrast
        -1.0,  // saturation
        -1.0,  // vibrancy
        -1.0,  // vibrancy_darkness
        -1.0,  // adaptive_dim
        -1.0,  // adaptive_boost
        0.0,   // refraction_flow
        1.0,   // refraction_spread
        0.0,   // fresnel_tint
        0.0,   // bevel_strength
        6.0,   // bevel_size
        0.0,   // bevel_tint
        315.0, // bevel_angle
        0.0,   // bevel_shadow
        0.0,   // self_sample
        -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0,
        -1.0, -1.0, -1.0, // the lens model's options: not set at any level
        -1.0, -1.0, -1.0, -1.0, -1.0, // the current material's: the same
    ];

    /// The option at this position of [`Self::ALL`].
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    /// The option named `key` in a config file.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::KEYS
            .iter()
            .position(|k| *k == key)
            .and_then(Self::from_index)
    }

    /// Position in [`Self::ALL`] and in every array of float values.
    #[must_use]
    pub fn index(self) -> usize {
        usize::from(self as u8)
    }

    /// Name in a config file.
    #[must_use]
    pub fn key(self) -> &'static str {
        Self::KEYS.get(self.index()).copied().unwrap_or("")
    }

    /// Value the global level is registered with; negative when unset.
    #[must_use]
    pub fn global_default(self) -> f64 {
        Self::GLOBAL_DEFAULTS
            .get(self.index())
            .copied()
            .unwrap_or(-1.0)
    }
}

impl IntOption {
    /// Every int option, in the order of its index.
    pub const ALL: [Self; INT_COUNT] = [
        Self::BlurIterations,
        Self::TintColor,
        Self::FresnelColor,
        Self::BevelColor,
        Self::Xray,
        Self::Model,
        Self::Material,
    ];

    const KEYS: [&'static str; INT_COUNT] = [
        "blur_iterations",
        "tint_color",
        "fresnel_color",
        "bevel_color",
        "xray",
        "model",
        "material",
    ];

    /// Registered value of the global level, by index; negative when unset.
    const GLOBAL_DEFAULTS: [i64; INT_COUNT] = [
        3,           // blur_iterations
        0x8899_aa22, // tint_color
        0xffff_ff00, // fresnel_color
        0xffff_ff00, // bevel_color
        0,           // xray
        -1,          // model
        -1,          // material
    ];

    /// The option at this position of [`Self::ALL`].
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    /// The option named `key` in a config file.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::KEYS
            .iter()
            .position(|k| *k == key)
            .and_then(Self::from_index)
    }

    /// Position in [`Self::ALL`] and in every array of int values.
    #[must_use]
    pub fn index(self) -> usize {
        usize::from(self as u8)
    }

    /// Name in a config file.
    #[must_use]
    pub fn key(self) -> &'static str {
        Self::KEYS.get(self.index()).copied().unwrap_or("")
    }

    /// Value the global level is registered with; negative when unset.
    #[must_use]
    pub fn global_default(self) -> i64 {
        Self::GLOBAL_DEFAULTS
            .get(self.index())
            .copied()
            .unwrap_or(UNSET_INT)
    }
}
