//! The plain data that crosses the C boundary, and its conversions to and
//! from the core crate's own types.
//!
//! Everything here is `#[repr(C)]` with fields of a fixed size: numbers,
//! arrays of numbers, and pointers paired with a length. Flags are `u32`, not
//! `bool`, because C may put any bit pattern in them. Nothing owns what it
//! points to: the caller allocates, fills and frees.

use glass_core::geometry::Rect;
use glass_core::lens::Lens;
use glass_core::motion::genie::Mouth;
use glass_core::motion::wobble::{COUNT, Wobble, WobbleSpec};
use glass_core::motion::{Spring, SpringSpec, Stretch};
use glass_core::options::{FLOAT_COUNT, INT_COUNT, Layered, Levels, ModelParams, Values};
use glass_core::panes::Pane;
use glass_core::schedule::{DrawConstants, FrostPlan, Layout, Reaches, SampleQuery};
use glass_core::shape::{ContinuousRect, Sample};

/// Bumped whenever a function or a struct of this header changes. The plugin
/// compares it with what the library answers at load.
pub const GLASS_ABI_VERSION: u32 = 9;
/// How many float options there are.
pub const GLASS_FLOAT_OPTIONS: usize = 49;
/// How many int options there are.
pub const GLASS_INT_OPTIONS: usize = 7;
/// Texels in a row of the lens table.
pub const GLASS_LENS_TABLE_SIZE: usize = 1024;
/// Device pixels of one cell of the pane map.
pub const GLASS_CELL_PIXELS: usize = 8;
/// Device pixels of one texel of the shadow.
pub const GLASS_SHADOW_PIXELS: usize = 8;
/// The most a layer's field is rebuilt while its outline moves on every frame, in milliseconds.
pub const GLASS_MIN_BUILD_INTERVAL_MS: u32 = 30;
/// Bytes a block of presets can take.
pub const GLASS_STORE_CAPACITY: usize = 262_144;
/// Bytes of a message.
pub const GLASS_TEXT_BYTES: usize = 256;
/// Floats in the uniform block of Apple's own shader.
pub const GLASS_QUARTZ_FLOATS: usize = 116;
/// Knobs of a tuning (`glass_core::tuning`).
pub const GLASS_TUNING_KNOBS: usize = 32;
/// Presets the tuner offers.
pub const GLASS_TUNING_PRESETS: usize = 8;
/// The knob that says whether windows show through glass (`see_windows`).
pub const GLASS_T_SEE_WINDOWS: u32 = 20;
/// How quickly every animation runs, 1 as it comes (`pace`).
pub const GLASS_T_PACE: u32 = 21;
/// Whether a closing window pours into the genie's bottle (`genie_close`).
pub const GLASS_T_GENIE_CLOSE: u32 = 22;
/// Whether a new window pours out of it (`genie_open`).
pub const GLASS_T_GENIE_OPEN: u32 = 23;
/// Whether the genie pours toward the pointer (`genie_pointer`).
pub const GLASS_T_GENIE_POINTER: u32 = 24;
/// The genie's seconds (`genie_time`).
pub const GLASS_T_GENIE_TIME: u32 = 25;
/// Whether a new window lands like jelly (`jelly`).
pub const GLASS_T_JELLY: u32 = 26;
/// How far it squashes, 0 to 1 (`jelly_squash`).
pub const GLASS_T_JELLY_SQUASH: u32 = 27;
/// Whether a dragged window wobbles (`wobble`).
pub const GLASS_T_WOBBLE: u32 = 28;
/// Whether windows the layout moves wobble too (`wobble_moves`).
pub const GLASS_T_WOBBLE_MOVES: u32 = 29;
/// A wobble's seconds (`wobble_time`).
pub const GLASS_T_WOBBLE_TIME: u32 = 30;
/// How far a wobble swings past rest, 0 to 1 (`wobble_bounce`).
pub const GLASS_T_WOBBLE_BOUNCE: u32 = 31;

const _: () = assert!(GLASS_FLOAT_OPTIONS == FLOAT_COUNT);
const _: () = assert!(GLASS_QUARTZ_FLOATS == glass_core::quartz::BLOCK_FLOATS);
const _: () = assert!(GLASS_TUNING_KNOBS == glass_core::tuning::COUNT);
const _: () = {
    use glass_core::tuning::Id;
    assert!(GLASS_T_SEE_WINDOWS == Id::SeeWindows as u32);
    assert!(GLASS_T_PACE == Id::Pace as u32);
    assert!(GLASS_T_GENIE_CLOSE == Id::GenieClose as u32);
    assert!(GLASS_T_GENIE_OPEN == Id::GenieOpen as u32);
    assert!(GLASS_T_GENIE_POINTER == Id::GeniePointer as u32);
    assert!(GLASS_T_GENIE_TIME == Id::GenieTime as u32);
    assert!(GLASS_T_JELLY == Id::Jelly as u32);
    assert!(GLASS_T_JELLY_SQUASH == Id::JellySquash as u32);
    assert!(GLASS_T_WOBBLE == Id::Wobble as u32);
    assert!(GLASS_T_WOBBLE_MOVES == Id::WobbleMoves as u32);
    assert!(GLASS_T_WOBBLE_TIME == Id::WobbleTime as u32);
    assert!(GLASS_T_WOBBLE_BOUNCE == Id::WobbleBounce as u32);
};
const _: () = assert!(GLASS_TUNING_PRESETS == glass_core::tuning::PRESETS.len());
const _: () = assert!(GLASS_INT_OPTIONS == INT_COUNT);
const _: () = assert!(GLASS_LENS_TABLE_SIZE == glass_core::schedule::LENS_TABLE_SIZE);
const _: () = assert!(GLASS_CELL_PIXELS == glass_core::panes::CELL_PIXELS);
const _: () = assert!(GLASS_MIN_BUILD_INTERVAL_MS == glass_core::schedule::MIN_BUILD_INTERVAL_MS);

/// The call went through.
pub const GLASS_OK: i32 = 0;
/// A pointer that must not be null was.
pub const GLASS_ERR_NULL: i32 = -1;
/// An argument is out of range, or a value does not mean anything.
pub const GLASS_ERR_ARG: i32 = -2;
/// A buffer is too small; the counts say how large it has to be.
pub const GLASS_ERR_CAPACITY: i32 = -3;
/// A block of presets is not one, or is full.
pub const GLASS_ERR_STORE: i32 = -4;
/// A line of a preset does not read; the message says why.
pub const GLASS_ERR_PARSE: i32 = -5;
/// Something in the library panicked and was stopped before it reached the caller.
pub const GLASS_ERR_PANIC: i32 = -6;
/// A name or option that does not exist.
pub const GLASS_ERR_UNKNOWN: i32 = -7;

// Float options, by index: what glass_option_info(0, index) and GlassValues.f use.
/// The float option `blur_strength`.
pub const GLASS_F_BLUR_STRENGTH: u32 = 0;
/// The float option `noise_strength`.
pub const GLASS_F_NOISE_STRENGTH: u32 = 1;
/// The float option `refraction_strength`.
pub const GLASS_F_REFRACTION_STRENGTH: u32 = 2;
/// The float option `chromatic_aberration`.
pub const GLASS_F_CHROMATIC_ABERRATION: u32 = 3;
/// The float option `fresnel_strength`.
pub const GLASS_F_FRESNEL_STRENGTH: u32 = 4;
/// The float option `specular_strength`.
pub const GLASS_F_SPECULAR_STRENGTH: u32 = 5;
/// The float option `specular_angle`.
pub const GLASS_F_SPECULAR_ANGLE: u32 = 6;
/// The float option `glass_opacity`.
pub const GLASS_F_GLASS_OPACITY: u32 = 7;
/// The float option `edge_thickness`.
pub const GLASS_F_EDGE_THICKNESS: u32 = 8;
/// The float option `lens_distortion`.
pub const GLASS_F_LENS_DISTORTION: u32 = 9;
/// The float option `brightness`.
pub const GLASS_F_BRIGHTNESS: u32 = 10;
/// The float option `contrast`.
pub const GLASS_F_CONTRAST: u32 = 11;
/// The float option `saturation`.
pub const GLASS_F_SATURATION: u32 = 12;
/// The float option `vibrancy`.
pub const GLASS_F_VIBRANCY: u32 = 13;
/// The float option `vibrancy_darkness`.
pub const GLASS_F_VIBRANCY_DARKNESS: u32 = 14;
/// The float option `adaptive_dim`.
pub const GLASS_F_ADAPTIVE_DIM: u32 = 15;
/// The float option `adaptive_boost`.
pub const GLASS_F_ADAPTIVE_BOOST: u32 = 16;
/// The float option `refraction_flow`.
pub const GLASS_F_REFRACTION_FLOW: u32 = 17;
/// The float option `refraction_spread`.
pub const GLASS_F_REFRACTION_SPREAD: u32 = 18;
/// The float option `fresnel_tint`.
pub const GLASS_F_FRESNEL_TINT: u32 = 19;
/// The float option `bevel_strength`.
pub const GLASS_F_BEVEL_STRENGTH: u32 = 20;
/// The float option `bevel_size`.
pub const GLASS_F_BEVEL_SIZE: u32 = 21;
/// The float option `bevel_tint`.
pub const GLASS_F_BEVEL_TINT: u32 = 22;
/// The float option `bevel_angle`.
pub const GLASS_F_BEVEL_ANGLE: u32 = 23;
/// The float option `bevel_shadow`.
pub const GLASS_F_BEVEL_SHADOW: u32 = 24;
/// The float option `self_sample`.
pub const GLASS_F_SELF_SAMPLE: u32 = 25;
/// The float option `bezel_width`.
pub const GLASS_F_BEZEL_WIDTH: u32 = 26;
/// The float option `glass_thickness`.
pub const GLASS_F_GLASS_THICKNESS: u32 = 27;
/// The float option `refractive_index`.
pub const GLASS_F_REFRACTIVE_INDEX: u32 = 28;
/// The float option `profile_exponent`.
pub const GLASS_F_PROFILE_EXPONENT: u32 = 29;
/// The float option `refraction`.
pub const GLASS_F_REFRACTION: u32 = 30;
/// The float option `dispersion`.
pub const GLASS_F_DISPERSION: u32 = 31;
/// The float option `frost`.
pub const GLASS_F_FROST: u32 = 32;
/// The float option `light_angle`.
pub const GLASS_F_LIGHT_ANGLE: u32 = 33;
/// The float option `light_intensity`.
pub const GLASS_F_LIGHT_INTENSITY: u32 = 34;
/// The float option `counter_light`.
pub const GLASS_F_COUNTER_LIGHT: u32 = 35;
/// The float option `rim_width`.
pub const GLASS_F_RIM_WIDTH: u32 = 36;
/// The float option `rim_glow`.
pub const GLASS_F_RIM_GLOW: u32 = 37;
/// The float option `inner_shadow`.
pub const GLASS_F_INNER_SHADOW: u32 = 38;
/// The float option `shadow_opacity`.
pub const GLASS_F_SHADOW_OPACITY: u32 = 39;
/// The float option `shadow_radius`.
pub const GLASS_F_SHADOW_RADIUS: u32 = 40;
/// The float option `shadow_offset`.
pub const GLASS_F_SHADOW_OFFSET: u32 = 41;
/// The float option `tint_over_dark`.
pub const GLASS_F_TINT_OVER_DARK: u32 = 42;
/// The float option `tint_over_bright`.
pub const GLASS_F_TINT_OVER_BRIGHT: u32 = 43;
/// The float option `frost_wide`.
pub const GLASS_F_FROST_WIDE: u32 = 44;
/// The float option `frost_cloud`.
pub const GLASS_F_FROST_CLOUD: u32 = 45;
/// The float option `body_shade`.
pub const GLASS_F_BODY_SHADE: u32 = 46;
/// The float option `rim_light`.
pub const GLASS_F_RIM_LIGHT: u32 = 47;
/// The float option `rim_shade`.
pub const GLASS_F_RIM_SHADE: u32 = 48;

// Int options, by index: what glass_option_info(1, index) and GlassValues.i use.
/// The int option `blur_iterations`.
pub const GLASS_I_BLUR_ITERATIONS: u32 = 0;
/// The int option `tint_color`.
pub const GLASS_I_TINT_COLOR: u32 = 1;
/// The int option `fresnel_color`.
pub const GLASS_I_FRESNEL_COLOR: u32 = 2;
/// The int option `bevel_color`.
pub const GLASS_I_BEVEL_COLOR: u32 = 3;
/// The int option `xray`.
pub const GLASS_I_XRAY: u32 = 4;
/// The int option `model`.
pub const GLASS_I_MODEL: u32 = 5;
/// The int option `material`.
pub const GLASS_I_MATERIAL: u32 = 6;

/// A rectangle: top left corner and size.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GlassRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The frame round a solid window: the strips to draw, in order top, bottom,
/// left, right. A strip of no size is not drawn.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlassFrame {
    pub strips: [GlassRect; 4],
}

/// The slab of glass; lengths in device pixels.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassLens {
    pub bezel_width: f64,
    pub thickness: f64,
    pub refractive_index: f64,
    pub profile_exponent: f64,
    pub refraction: f64,
    pub dispersion: f64,
    /// 1 when the rim bends by the thin-prism law of the current material.
    pub paraxial: u32,
}

/// What the current material adds to a pane's glass; each term is off at 0.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlassLayered {
    /// Deviation of the wide copy of the page, device pixels; 0 for a frost of one copy.
    pub frost_wide: f32,
    /// Share of the wide copy laid over the held narrow one.
    pub frost_cloud: f32,
    /// The narrow copy is held within this of the wide one: above when positive, below when negative.
    pub frost_hold: f32,
    /// How many times more white weighs than black in the wide copy's mean.
    pub cloud_weight: f32,
    /// The same in the narrow copy's.
    pub ghost_weight: f32,
    /// Bright content pulled down: `rgb *= 1 - this * luma^2`.
    pub body_shade: f32,
    /// Saturation is this share more over black and less over white.
    pub adaptive_saturation: f32,
    /// Strength of the lobes of light on the rim.
    pub rim_light: f32,
    /// Strength of the dark hairline.
    pub rim_shade: f32,
    /// Share of the hairline left at the ends of the light's axis.
    pub rim_shade_ends: f32,
    /// How much the rim's glass darkens what it shows.
    pub edge_absorption: f32,
    /// Device pixels in a third of a point, the unit of the rim's lengths.
    pub third: f32,
}

/// What a pane's glass is made of, every length in device pixels.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassParams {
    pub lens: GlassLens,
    pub frost: f32,
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
    pub tint_color: [f32; 3],
    pub layered: GlassLayered,
    /// Apple's own shader as transcribed: 0 for none, else a preset
    /// (`glass_core::quartz::Kind::code`); see `glass_quartz`.
    pub quartz: u32,
}

/// What Apple's own shader is given (`glass_core::quartz`): its uniform block,
/// std140, the panel size its lengths were fitted on, and how far beyond a
/// pane it reads the page, both in the fitted panel's pixels.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassQuartz {
    pub block: [f32; GLASS_QUARTZ_FLOATS],
    pub scale_ref: f32,
    pub reach: f32,
    /// The face's blur radius, in points, for a backdrop blurred to match.
    pub blur: f32,
}

/// A value for every option; a negative one is "not set".
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassValues {
    pub f: [f32; GLASS_FLOAT_OPTIONS],
    pub i: [i64; GLASS_INT_OPTIONS],
}

/// The three levels of the config an option can be set at.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassLevels {
    pub global: GlassValues,
    pub dark: GlassValues,
    pub light: GlassValues,
}

/// Bytes the caller owns, lent for the length of a call.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlassBytes {
    pub data: *const u8,
    pub len: usize,
}

/// A block of presets in memory the caller owns: `capacity` bytes at `data`,
/// of which the first `len` are a store. The library rewrites it in place.
#[repr(C)]
#[derive(Debug)]
pub struct GlassStore {
    pub data: *mut u8,
    pub capacity: usize,
    pub len: usize,
}

/// A message, UTF-8, `len` bytes of it.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlassText {
    pub bytes: [u8; GLASS_TEXT_BYTES],
    pub len: u32,
}

/// A tuning of the glass: one value for every knob, in the knobs' order.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassTuning {
    pub values: [f32; GLASS_TUNING_KNOBS],
}

/// One knob of a tuning, as the tuner shows it: its key in the file, its label,
/// a sentence on what it does, its unit, where it sits (group: 0 glass, 1 edge,
/// 2 light, 3 behind, 4 motion) and how (shown: 0 slider, 1 switch, 2 colour,
/// 3 part of the colour before it), and its range.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlassKnob {
    pub key: GlassText,
    pub label: GlassText,
    pub help: GlassText,
    pub unit: GlassText,
    pub group: u32,
    pub shown: u32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub default_value: f32,
}

/// One preset of the tuner: its name and what the tuner calls it.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlassTuningPreset {
    pub name: GlassText,
    pub label: GlassText,
}

/// What to resolve the options of.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlassResolveRequest {
    /// The committed presets, as a block.
    pub store: GlassBytes,
    /// The name of the preset to start from.
    pub preset: GlassBytes,
    /// The config's levels.
    pub levels: GlassLevels,
    /// 1 for the dark appearance, 0 for the light.
    pub dark: u32,
}

/// One setting of one layer of a preset, as text.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlassFieldEdit {
    pub name: GlassBytes,
    /// 0 shared, 1 dark, 2 light.
    pub variant: u32,
    pub key: GlassBytes,
    pub value: GlassBytes,
}

/// What a config option is called and what it is registered with.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlassOptionInfo {
    /// The key, UTF-8 without a terminator, in memory that lives as long as the library.
    pub key: *const u8,
    pub key_len: usize,
    /// The value the global level is registered with; negative when it is not set.
    pub global_default: f64,
}

/// What the classic shader falls back to for a theme.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassThemeDefaults {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub vibrancy: f32,
    pub vibrancy_darkness: f32,
    pub adaptive_dim: f32,
    pub adaptive_boost: f32,
}

/// Where the classic shader's tone curve first stops rising, and how high it
/// is there (`glass_core::tone`): past `at` nothing behind the glass comes out
/// darker than `value`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassTonePeak {
    pub at: f32,
    pub value: f32,
}

/// Which rebuilds a change of parameters asks for; each 1 or 0.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlassChanges {
    pub field: u32,
    pub lens: u32,
    pub frost: u32,
}

/// How a layer's field, its shadow and its map of cells are laid out.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassLayout {
    pub field_sigma: f64,
    pub field_scale: i32,
    pub margin: i32,
    pub shadow_reach: i32,
    pub field_texels: [i32; 2],
    pub shadow_texels: [i32; 2],
    pub cells: [i32; 2],
    pub initial_fit: u32,
    pub blur_sigma: f32,
    pub blur_radius: i32,
    pub shadow_on: u32,
    pub shadow_sigma: f32,
    pub shadow_taps: i32,
    pub small: u32,
}

/// How the frost of a pane is made from the page behind it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassFrostPlan {
    pub reduction: i32,
    pub sigma: f32,
    pub face_applied: u32,
    pub passes: i32,
    pub pass_sigma: f32,
    pub pairs: i32,
}

/// The part of the screen a pane's frost is read from.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassSampleQuery {
    pub pane: GlassRect,
    pub box_x: f64,
    pub box_y: f64,
    pub source_w: f64,
    pub source_h: f64,
    pub frost: f32,
    pub reach: i32,
}

/// How far glass reaches beyond a pane, in device pixels.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassReaches {
    pub frost: i32,
    pub shadow: i32,
    pub damage: i32,
    pub padding: i32,
    pub repaint: f64,
}

/// What the glass shader is given besides the textures and the geometry of a quad.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassDrawConstants {
    pub thickness_per_fit: f32,
    pub interior_coverage: f32,
    pub solid_coverage: f32,
    pub outer_coverage: f32,
    pub rim_coverage: f32,
    pub deviation_scale: f32,
    pub dispersive: u32,
    pub light_direction: [f32; 2],
}

/// The opacity of the glass and of its shadow in one frame.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassOpacities {
    pub glass: f32,
    pub shadow: f32,
}

/// The measured constants of the current material's rim, for a shader that is
/// not made in the core (the one that draws a window's pane). Lengths are
/// thirds of a point.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlassRimConstants {
    /// Share of the lit lobe's strength the far lobe has.
    pub opposite: f32,
    /// `(1 - luma)^this`: the lobes fade over a bright body.
    pub luma_power: f32,
    /// The core of a lobe: its strength, and the length it falls to 1/e in.
    pub core: [f32; 2],
    /// The tail of a lobe, likewise.
    pub tail: [f32; 2],
    /// How far in the lobes begin.
    pub inset: f32,
    /// How much darker than the page beside the pane the hairline is.
    pub hairline_shade: f32,
    /// `across = (1 - along^2)^this`.
    pub hairline_power: f32,
    /// Where the hairline starts to fade and where it has ended.
    pub hairline_width: [f32; 2],
    /// How far outward the page is read for it.
    pub hairline_read: f32,
}

/// A piece of glass; its face rectangles are `interior_count` of the list
/// that comes with it, from `first_interior`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassPane {
    pub rect: GlassRect,
    pub short_side: f64,
    pub first_interior: u32,
    pub interior_count: u32,
}

/// Panes lent to the library for a call, with their face rectangles.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlassPaneList {
    pub panes: *const GlassPane,
    pub pane_count: usize,
    pub interiors: *const GlassRect,
    pub interior_count: usize,
}

/// Room for panes: the library writes into it and says how many it found.
/// When there is not enough room it still counts, and the call fails with
/// `GLASS_ERR_CAPACITY`, so that the caller can try again with enough.
#[repr(C)]
#[derive(Debug)]
pub struct GlassPaneBuffers {
    pub panes: *mut GlassPane,
    pub pane_capacity: usize,
    pub interiors: *mut GlassRect,
    pub interior_capacity: usize,
    /// One byte per cell of the map, if the call makes any.
    pub fit: *mut u8,
    pub fit_capacity: usize,
    pub pane_count: usize,
    pub interior_count: usize,
    pub fit_count: usize,
}

/// The GPU's coarse map of where panes are, and what its cells stand for.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlassMapRequest {
    /// `width * height` RGBA texels.
    pub cells: GlassBytes,
    pub width: usize,
    pub height: usize,
    pub margin: i32,
    pub field_scale: i32,
    pub sigma: f64,
    pub bezel_width: f64,
    pub box_w: f64,
    pub box_h: f64,
}

/// The panes that stand now.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlassWantedRequest {
    /// The panes of the last map, if there is one.
    pub known: GlassPaneList,
    pub has_known: u32,
    /// Part of the layer redrawn since the map, if any.
    pub unknown: GlassRect,
    pub has_unknown: u32,
    /// The layer's box.
    pub whole: GlassRect,
    /// 1 when the layer is about the size of what it shows.
    pub small: u32,
}

impl From<Rect> for GlassRect {
    fn from(r: Rect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        }
    }
}

impl From<GlassRect> for Rect {
    fn from(r: GlassRect) -> Self {
        Self::new(r.x, r.y, r.w, r.h)
    }
}

impl From<GlassLens> for Lens {
    fn from(l: GlassLens) -> Self {
        Self {
            bezel_width: l.bezel_width,
            thickness: l.thickness,
            refractive_index: l.refractive_index,
            profile_exponent: l.profile_exponent,
            refraction: l.refraction,
            dispersion: l.dispersion,
            paraxial: l.paraxial != 0,
        }
    }
}

impl From<Lens> for GlassLens {
    fn from(l: Lens) -> Self {
        Self {
            bezel_width: l.bezel_width,
            thickness: l.thickness,
            refractive_index: l.refractive_index,
            profile_exponent: l.profile_exponent,
            refraction: l.refraction,
            dispersion: l.dispersion,
            paraxial: u32::from(l.paraxial),
        }
    }
}

impl From<Layered> for GlassLayered {
    fn from(l: Layered) -> Self {
        Self {
            frost_wide: l.frost_wide,
            frost_cloud: l.frost_cloud,
            frost_hold: l.frost_hold,
            cloud_weight: l.cloud_weight,
            ghost_weight: l.ghost_weight,
            body_shade: l.body_shade,
            adaptive_saturation: l.adaptive_saturation,
            rim_light: l.rim_light,
            rim_shade: l.rim_shade,
            rim_shade_ends: l.rim_shade_ends,
            edge_absorption: l.edge_absorption,
            third: l.third,
        }
    }
}

impl From<GlassLayered> for Layered {
    fn from(l: GlassLayered) -> Self {
        Self {
            frost_wide: l.frost_wide,
            frost_cloud: l.frost_cloud,
            frost_hold: l.frost_hold,
            cloud_weight: l.cloud_weight,
            ghost_weight: l.ghost_weight,
            body_shade: l.body_shade,
            adaptive_saturation: l.adaptive_saturation,
            rim_light: l.rim_light,
            rim_shade: l.rim_shade,
            rim_shade_ends: l.rim_shade_ends,
            edge_absorption: l.edge_absorption,
            third: l.third,
        }
    }
}

impl From<ModelParams> for GlassParams {
    fn from(p: ModelParams) -> Self {
        Self {
            lens: p.lens.into(),
            frost: p.frost,
            light_angle: p.light_angle,
            light_intensity: p.light_intensity,
            counter_light: p.counter_light,
            rim_width: p.rim_width,
            rim_glow: p.rim_glow,
            inner_shadow: p.inner_shadow,
            shadow_opacity: p.shadow_opacity,
            shadow_radius: p.shadow_radius,
            shadow_offset: p.shadow_offset,
            tint_over_dark: p.tint_over_dark,
            tint_over_bright: p.tint_over_bright,
            saturation: p.saturation,
            brightness: p.brightness,
            contrast: p.contrast,
            noise_strength: p.noise_strength,
            glass_opacity: p.glass_opacity,
            tint_color: p.tint_color,
            layered: p.layered.into(),
            quartz: p.quartz.map_or(0, glass_core::quartz::Kind::code),
        }
    }
}

impl From<GlassParams> for ModelParams {
    fn from(p: GlassParams) -> Self {
        Self {
            lens: p.lens.into(),
            frost: p.frost,
            light_angle: p.light_angle,
            light_intensity: p.light_intensity,
            counter_light: p.counter_light,
            rim_width: p.rim_width,
            rim_glow: p.rim_glow,
            inner_shadow: p.inner_shadow,
            shadow_opacity: p.shadow_opacity,
            shadow_radius: p.shadow_radius,
            shadow_offset: p.shadow_offset,
            tint_over_dark: p.tint_over_dark,
            tint_over_bright: p.tint_over_bright,
            saturation: p.saturation,
            brightness: p.brightness,
            contrast: p.contrast,
            noise_strength: p.noise_strength,
            glass_opacity: p.glass_opacity,
            tint_color: p.tint_color,
            layered: p.layered.into(),
            quartz: glass_core::quartz::Kind::from_code(p.quartz),
        }
    }
}

impl From<&Values> for GlassValues {
    fn from(v: &Values) -> Self {
        Self { f: v.f, i: v.i }
    }
}

impl From<&GlassValues> for Values {
    fn from(v: &GlassValues) -> Self {
        Self { f: v.f, i: v.i }
    }
}

impl From<&GlassLevels> for Levels {
    fn from(l: &GlassLevels) -> Self {
        Self {
            global: (&l.global).into(),
            dark: (&l.dark).into(),
            light: (&l.light).into(),
        }
    }
}

fn flag(on: bool) -> u32 {
    u32::from(on)
}

impl From<Layout> for GlassLayout {
    fn from(l: Layout) -> Self {
        Self {
            field_sigma: l.field_sigma,
            field_scale: l.field_scale,
            margin: l.margin,
            shadow_reach: l.shadow_reach,
            field_texels: l.field_texels,
            shadow_texels: l.shadow_texels,
            cells: l.cells,
            initial_fit: u32::from(l.initial_fit),
            blur_sigma: l.blur_sigma,
            blur_radius: l.blur_radius,
            shadow_on: flag(l.shadow_on),
            shadow_sigma: l.shadow_sigma,
            shadow_taps: l.shadow_taps,
            small: flag(l.small),
        }
    }
}

impl From<FrostPlan> for GlassFrostPlan {
    fn from(p: FrostPlan) -> Self {
        Self {
            reduction: p.reduction,
            sigma: p.sigma,
            face_applied: flag(p.face_applied),
            passes: p.passes,
            pass_sigma: p.pass_sigma,
            pairs: p.pairs,
        }
    }
}

impl From<&GlassSampleQuery> for SampleQuery {
    fn from(q: &GlassSampleQuery) -> Self {
        Self {
            pane: q.pane.into(),
            box_position: [q.box_x, q.box_y],
            source_size: [q.source_w, q.source_h],
            frost: q.frost,
            reach: q.reach,
        }
    }
}

impl From<Reaches> for GlassReaches {
    fn from(r: Reaches) -> Self {
        Self {
            frost: r.frost,
            shadow: r.shadow,
            damage: r.damage,
            padding: r.padding,
            repaint: r.repaint,
        }
    }
}

impl From<DrawConstants> for GlassDrawConstants {
    fn from(c: DrawConstants) -> Self {
        Self {
            thickness_per_fit: c.thickness_per_fit,
            interior_coverage: c.interior_coverage,
            solid_coverage: c.solid_coverage,
            outer_coverage: c.outer_coverage,
            rim_coverage: c.rim_coverage,
            deviation_scale: c.deviation_scale,
            dispersive: flag(c.dispersive),
            light_direction: c.light_direction,
        }
    }
}

impl GlassPane {
    /// The pane as the caller sees it, with its face rectangles appended to `interiors`.
    pub fn from_pane(pane: &Pane, interiors: &mut Vec<GlassRect>) -> Self {
        let first = u32::try_from(interiors.len()).unwrap_or(u32::MAX);
        interiors.extend(pane.interiors.iter().map(|r| GlassRect::from(*r)));
        Self {
            rect: pane.rect.into(),
            short_side: pane.short_side,
            first_interior: first,
            interior_count: u32::try_from(pane.interiors.len()).unwrap_or(u32::MAX),
        }
    }
}

/// A value moving to a target under a spring, as the library steps it (not
/// used by the plugin yet): duration and bounce say how it swings.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassSpring {
    pub position: f64,
    pub velocity: f64,
    pub target: f64,
    /// Seconds of one swing.
    pub duration: f64,
    /// 0 critical damping, up to 1 no damping, down to -1 creeping.
    pub bounce: f64,
}

impl GlassSpring {
    pub(crate) fn to_spring(self) -> Spring {
        let mut spring =
            Spring::at_rest(self.position, SpringSpec::new(self.duration, self.bounce));
        spring.velocity = self.velocity;
        spring.retarget(self.target);
        spring
    }
}

/// How a pane stretches along its motion and squashes across it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassStretch {
    pub along: f64,
    pub across: f64,
}

impl From<Stretch> for GlassStretch {
    fn from(s: Stretch) -> Self {
        Self {
            along: s.along,
            across: s.across,
        }
    }
}

/// Where a closing window goes into, or an opening one comes out of: a point
/// and the width of the mouth there.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlassMouth {
    pub x: f64,
    pub y: f64,
    pub width: f64,
}

impl From<GlassMouth> for Mouth {
    fn from(m: GlassMouth) -> Self {
        Self {
            x: m.x,
            y: m.y,
            width: m.width,
        }
    }
}

/// Room for a mesh of `cols` by `rows` cells: an x and a y for each of its
/// `(cols + 1) * (rows + 1)` points, row by row from the top left.
#[repr(C)]
#[derive(Debug)]
pub struct GlassMesh {
    pub points: *mut f32,
    pub capacity: usize,
    pub cols: u32,
    pub rows: u32,
}

/// Points a side of a wobbling window's sheet.
pub const GLASS_WOBBLE_NODES: usize = 5;
/// Numbers in a sheet's offsets or speeds: an x and a y for each point.
pub const GLASS_WOBBLE_FLOATS: usize = 50;
const _: () = assert!(GLASS_WOBBLE_FLOATS == 2 * COUNT);
const _: () = assert!(GLASS_WOBBLE_NODES * GLASS_WOBBLE_NODES == COUNT);

/// How a wobbling window swings: the spring of each point to its place
/// (seconds of a swing, bounce), how strongly it is held to its neighbours as
/// a share of that, and how far it may trail as a share of the window.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassWobbleSpec {
    pub duration: f64,
    pub bounce: f64,
    pub coupling: f64,
    pub reach: f64,
}

impl From<GlassWobbleSpec> for WobbleSpec {
    fn from(s: GlassWobbleSpec) -> Self {
        Self::new(s.duration, s.bounce, s.coupling, s.reach)
    }
}

/// A wobbling window's sheet, kept by the caller: each point's offset from
/// its place and its speed, x and y, row by row; the point held, or -1.
/// All zeros and -1 is a sheet at rest.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassWobble {
    pub offset: [f64; GLASS_WOBBLE_FLOATS],
    pub velocity: [f64; GLASS_WOBBLE_FLOATS],
    pub held: i32,
}

impl GlassWobble {
    pub(crate) fn sheet(&self) -> Wobble {
        let mut w = Wobble::still();
        let pairs = |flat: &[f64; GLASS_WOBBLE_FLOATS], into: &mut [[f64; 2]; COUNT]| {
            for (pair, from) in into.iter_mut().zip(flat.chunks_exact(2)) {
                if let [x, y] = from {
                    *pair = [*x, *y];
                }
            }
        };
        pairs(&self.offset, &mut w.offset);
        pairs(&self.velocity, &mut w.velocity);
        w.held = usize::try_from(self.held).ok().filter(|&k| k < COUNT);
        w
    }

    pub(crate) fn keep(&mut self, w: &Wobble) {
        let flat = |from: &[[f64; 2]; COUNT], into: &mut [f64; GLASS_WOBBLE_FLOATS]| {
            for (to, pair) in into.chunks_exact_mut(2).zip(from) {
                to.copy_from_slice(pair);
            }
        };
        flat(&w.offset, &mut self.offset);
        flat(&w.velocity, &mut self.velocity);
        self.held = w.held.and_then(|k| i32::try_from(k).ok()).unwrap_or(-1);
    }
}

/// A rectangle with continuous corners, centred on the origin, y down.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassDeclaredRect {
    pub width: f64,
    pub height: f64,
    /// Radii: top left, top right, bottom right, bottom left.
    pub radii: [f64; 4],
}

impl GlassDeclaredRect {
    pub(crate) fn to_shape(self) -> ContinuousRect {
        ContinuousRect::with_radii(self.width, self.height, self.radii)
    }
}

/// A shape at a point: its signed distance (negative inside) and outward unit normal.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassSample {
    pub distance: f64,
    pub normal: [f64; 2],
}

impl From<Sample> for GlassSample {
    fn from(s: Sample) -> Self {
        Self {
            distance: s.distance,
            normal: s.normal,
        }
    }
}

impl From<GlassSample> for Sample {
    fn from(s: GlassSample) -> Self {
        Self {
            distance: s.distance,
            normal: s.normal,
        }
    }
}

impl From<&glass_core::tuning::Tuning> for GlassTuning {
    fn from(tuning: &glass_core::tuning::Tuning) -> Self {
        Self {
            values: tuning.values(),
        }
    }
}

impl From<&GlassTuning> for glass_core::tuning::Tuning {
    fn from(tuning: &GlassTuning) -> Self {
        Self::from_values(&tuning.values)
    }
}
