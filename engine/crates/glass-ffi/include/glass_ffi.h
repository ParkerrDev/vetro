/* glass_ffi.h: the C interface of the glass model (crates/glass-ffi).
*
* Generated from the Rust source by `cargo test -p glass-ffi --test header`
* (GLASS_UPDATE_HEADER=1 writes it again). Do not edit it by hand: that test
* fails while this file and the source disagree.
*
* Every function answers with an int32_t: zero or more for success, a
* negative GLASS_ERR_* for failure. Pointers are lent for the length of the
* call and never kept; results are written to memory the caller owns.
*/
#ifndef GLASS_FFI_H
#define GLASS_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Bumped whenever a function or a struct of this header changes. The plugin
// compares it with what the library answers at load.
#define GLASS_ABI_VERSION 9
// How many float options there are.
#define GLASS_FLOAT_OPTIONS 49
// How many int options there are.
#define GLASS_INT_OPTIONS 7
// Texels in a row of the lens table.
#define GLASS_LENS_TABLE_SIZE 1024
// Device pixels of one cell of the pane map.
#define GLASS_CELL_PIXELS 8
// Device pixels of one texel of the shadow.
#define GLASS_SHADOW_PIXELS 8
// The most a layer's field is rebuilt while its outline moves on every frame, in milliseconds.
#define GLASS_MIN_BUILD_INTERVAL_MS 30
// Bytes a block of presets can take.
#define GLASS_STORE_CAPACITY 262144
// Bytes of a message.
#define GLASS_TEXT_BYTES 256
// Floats in the uniform block of Apple's own shader.
#define GLASS_QUARTZ_FLOATS 116
// Knobs of a tuning (`glass_core::tuning`).
#define GLASS_TUNING_KNOBS 32
// Presets the tuner offers.
#define GLASS_TUNING_PRESETS 8
// The knob that says whether windows show through glass (`see_windows`).
#define GLASS_T_SEE_WINDOWS 20
// How quickly every animation runs, 1 as it comes (`pace`).
#define GLASS_T_PACE 21
// Whether a closing window pours into the genie's bottle (`genie_close`).
#define GLASS_T_GENIE_CLOSE 22
// Whether a new window pours out of it (`genie_open`).
#define GLASS_T_GENIE_OPEN 23
// Whether the genie pours toward the pointer (`genie_pointer`).
#define GLASS_T_GENIE_POINTER 24
// The genie's seconds (`genie_time`).
#define GLASS_T_GENIE_TIME 25
// Whether a new window lands like jelly (`jelly`).
#define GLASS_T_JELLY 26
// How far it squashes, 0 to 1 (`jelly_squash`).
#define GLASS_T_JELLY_SQUASH 27
// Whether a dragged window wobbles (`wobble`).
#define GLASS_T_WOBBLE 28
// Whether windows the layout moves wobble too (`wobble_moves`).
#define GLASS_T_WOBBLE_MOVES 29
// A wobble's seconds (`wobble_time`).
#define GLASS_T_WOBBLE_TIME 30
// How far a wobble swings past rest, 0 to 1 (`wobble_bounce`).
#define GLASS_T_WOBBLE_BOUNCE 31
// The call went through.
#define GLASS_OK 0
// A pointer that must not be null was.
#define GLASS_ERR_NULL (-1)
// An argument is out of range, or a value does not mean anything.
#define GLASS_ERR_ARG (-2)
// A buffer is too small; the counts say how large it has to be.
#define GLASS_ERR_CAPACITY (-3)
// A block of presets is not one, or is full.
#define GLASS_ERR_STORE (-4)
// A line of a preset does not read; the message says why.
#define GLASS_ERR_PARSE (-5)
// Something in the library panicked and was stopped before it reached the caller.
#define GLASS_ERR_PANIC (-6)
// A name or option that does not exist.
#define GLASS_ERR_UNKNOWN (-7)
// The float option `blur_strength`.
#define GLASS_F_BLUR_STRENGTH 0
// The float option `noise_strength`.
#define GLASS_F_NOISE_STRENGTH 1
// The float option `refraction_strength`.
#define GLASS_F_REFRACTION_STRENGTH 2
// The float option `chromatic_aberration`.
#define GLASS_F_CHROMATIC_ABERRATION 3
// The float option `fresnel_strength`.
#define GLASS_F_FRESNEL_STRENGTH 4
// The float option `specular_strength`.
#define GLASS_F_SPECULAR_STRENGTH 5
// The float option `specular_angle`.
#define GLASS_F_SPECULAR_ANGLE 6
// The float option `glass_opacity`.
#define GLASS_F_GLASS_OPACITY 7
// The float option `edge_thickness`.
#define GLASS_F_EDGE_THICKNESS 8
// The float option `lens_distortion`.
#define GLASS_F_LENS_DISTORTION 9
// The float option `brightness`.
#define GLASS_F_BRIGHTNESS 10
// The float option `contrast`.
#define GLASS_F_CONTRAST 11
// The float option `saturation`.
#define GLASS_F_SATURATION 12
// The float option `vibrancy`.
#define GLASS_F_VIBRANCY 13
// The float option `vibrancy_darkness`.
#define GLASS_F_VIBRANCY_DARKNESS 14
// The float option `adaptive_dim`.
#define GLASS_F_ADAPTIVE_DIM 15
// The float option `adaptive_boost`.
#define GLASS_F_ADAPTIVE_BOOST 16
// The float option `refraction_flow`.
#define GLASS_F_REFRACTION_FLOW 17
// The float option `refraction_spread`.
#define GLASS_F_REFRACTION_SPREAD 18
// The float option `fresnel_tint`.
#define GLASS_F_FRESNEL_TINT 19
// The float option `bevel_strength`.
#define GLASS_F_BEVEL_STRENGTH 20
// The float option `bevel_size`.
#define GLASS_F_BEVEL_SIZE 21
// The float option `bevel_tint`.
#define GLASS_F_BEVEL_TINT 22
// The float option `bevel_angle`.
#define GLASS_F_BEVEL_ANGLE 23
// The float option `bevel_shadow`.
#define GLASS_F_BEVEL_SHADOW 24
// The float option `self_sample`.
#define GLASS_F_SELF_SAMPLE 25
// The float option `bezel_width`.
#define GLASS_F_BEZEL_WIDTH 26
// The float option `glass_thickness`.
#define GLASS_F_GLASS_THICKNESS 27
// The float option `refractive_index`.
#define GLASS_F_REFRACTIVE_INDEX 28
// The float option `profile_exponent`.
#define GLASS_F_PROFILE_EXPONENT 29
// The float option `refraction`.
#define GLASS_F_REFRACTION 30
// The float option `dispersion`.
#define GLASS_F_DISPERSION 31
// The float option `frost`.
#define GLASS_F_FROST 32
// The float option `light_angle`.
#define GLASS_F_LIGHT_ANGLE 33
// The float option `light_intensity`.
#define GLASS_F_LIGHT_INTENSITY 34
// The float option `counter_light`.
#define GLASS_F_COUNTER_LIGHT 35
// The float option `rim_width`.
#define GLASS_F_RIM_WIDTH 36
// The float option `rim_glow`.
#define GLASS_F_RIM_GLOW 37
// The float option `inner_shadow`.
#define GLASS_F_INNER_SHADOW 38
// The float option `shadow_opacity`.
#define GLASS_F_SHADOW_OPACITY 39
// The float option `shadow_radius`.
#define GLASS_F_SHADOW_RADIUS 40
// The float option `shadow_offset`.
#define GLASS_F_SHADOW_OFFSET 41
// The float option `tint_over_dark`.
#define GLASS_F_TINT_OVER_DARK 42
// The float option `tint_over_bright`.
#define GLASS_F_TINT_OVER_BRIGHT 43
// The float option `frost_wide`.
#define GLASS_F_FROST_WIDE 44
// The float option `frost_cloud`.
#define GLASS_F_FROST_CLOUD 45
// The float option `body_shade`.
#define GLASS_F_BODY_SHADE 46
// The float option `rim_light`.
#define GLASS_F_RIM_LIGHT 47
// The float option `rim_shade`.
#define GLASS_F_RIM_SHADE 48
// The int option `blur_iterations`.
#define GLASS_I_BLUR_ITERATIONS 0
// The int option `tint_color`.
#define GLASS_I_TINT_COLOR 1
// The int option `fresnel_color`.
#define GLASS_I_FRESNEL_COLOR 2
// The int option `bevel_color`.
#define GLASS_I_BEVEL_COLOR 3
// The int option `xray`.
#define GLASS_I_XRAY 4
// The int option `model`.
#define GLASS_I_MODEL 5
// The int option `material`.
#define GLASS_I_MATERIAL 6
// Points a side of a wobbling window's sheet.
#define GLASS_WOBBLE_NODES 5
// Numbers in a sheet's offsets or speeds: an x and a y for each point.
#define GLASS_WOBBLE_FLOATS 50

// A rectangle: top left corner and size.
typedef struct GlassRect {
    double x;
    double y;
    double w;
    double h;
} GlassRect;

// The frame round a solid window: the strips to draw, in order top, bottom,
// left, right. A strip of no size is not drawn.
typedef struct GlassFrame {
    GlassRect strips[4];
} GlassFrame;

// The slab of glass; lengths in device pixels.
typedef struct GlassLens {
    double bezel_width;
    double thickness;
    double refractive_index;
    double profile_exponent;
    double refraction;
    double dispersion;
    // 1 when the rim bends by the thin-prism law of the current material.
    uint32_t paraxial;
} GlassLens;

// What the current material adds to a pane's glass; each term is off at 0.
typedef struct GlassLayered {
    // Deviation of the wide copy of the page, device pixels; 0 for a frost of one copy.
    float frost_wide;
    // Share of the wide copy laid over the held narrow one.
    float frost_cloud;
    // The narrow copy is held within this of the wide one: above when positive, below when negative.
    float frost_hold;
    // How many times more white weighs than black in the wide copy's mean.
    float cloud_weight;
    // The same in the narrow copy's.
    float ghost_weight;
    // Bright content pulled down: `rgb *= 1 - this * luma^2`.
    float body_shade;
    // Saturation is this share more over black and less over white.
    float adaptive_saturation;
    // Strength of the lobes of light on the rim.
    float rim_light;
    // Strength of the dark hairline.
    float rim_shade;
    // Share of the hairline left at the ends of the light's axis.
    float rim_shade_ends;
    // How much the rim's glass darkens what it shows.
    float edge_absorption;
    // Device pixels in a third of a point, the unit of the rim's lengths.
    float third;
} GlassLayered;

// What a pane's glass is made of, every length in device pixels.
typedef struct GlassParams {
    GlassLens lens;
    float frost;
    float light_angle;
    float light_intensity;
    float counter_light;
    float rim_width;
    float rim_glow;
    float inner_shadow;
    float shadow_opacity;
    float shadow_radius;
    float shadow_offset;
    float tint_over_dark;
    float tint_over_bright;
    float saturation;
    float brightness;
    float contrast;
    float noise_strength;
    float glass_opacity;
    float tint_color[3];
    GlassLayered layered;
    // Apple's own shader as transcribed: 0 for none, else a preset
    // (`glass_core::quartz::Kind::code`); see `glass_quartz`.
    uint32_t quartz;
} GlassParams;

// What Apple's own shader is given (`glass_core::quartz`): its uniform block,
// std140, the panel size its lengths were fitted on, and how far beyond a
// pane it reads the page, both in the fitted panel's pixels.
typedef struct GlassQuartz {
    float block[GLASS_QUARTZ_FLOATS];
    float scale_ref;
    float reach;
    // The face's blur radius, in points, for a backdrop blurred to match.
    float blur;
} GlassQuartz;

// A value for every option; a negative one is "not set".
typedef struct GlassValues {
    float f[GLASS_FLOAT_OPTIONS];
    int64_t i[GLASS_INT_OPTIONS];
} GlassValues;

// The three levels of the config an option can be set at.
typedef struct GlassLevels {
    GlassValues global;
    GlassValues dark;
    GlassValues light;
} GlassLevels;

// Bytes the caller owns, lent for the length of a call.
typedef struct GlassBytes {
    const uint8_t* data;
    size_t len;
} GlassBytes;

// A block of presets in memory the caller owns: `capacity` bytes at `data`,
// of which the first `len` are a store. The library rewrites it in place.
typedef struct GlassStore {
    uint8_t* data;
    size_t capacity;
    size_t len;
} GlassStore;

// A message, UTF-8, `len` bytes of it.
typedef struct GlassText {
    uint8_t bytes[GLASS_TEXT_BYTES];
    uint32_t len;
} GlassText;

// A tuning of the glass: one value for every knob, in the knobs' order.
typedef struct GlassTuning {
    float values[GLASS_TUNING_KNOBS];
} GlassTuning;

// One knob of a tuning, as the tuner shows it: its key in the file, its label,
// a sentence on what it does, its unit, where it sits (group: 0 glass, 1 edge,
// 2 light, 3 behind, 4 motion) and how (shown: 0 slider, 1 switch, 2 colour,
// 3 part of the colour before it), and its range.
typedef struct GlassKnob {
    GlassText key;
    GlassText label;
    GlassText help;
    GlassText unit;
    uint32_t group;
    uint32_t shown;
    float min;
    float max;
    float step;
    float default_value;
} GlassKnob;

// One preset of the tuner: its name and what the tuner calls it.
typedef struct GlassTuningPreset {
    GlassText name;
    GlassText label;
} GlassTuningPreset;

// What to resolve the options of.
typedef struct GlassResolveRequest {
    // The committed presets, as a block.
    GlassBytes store;
    // The name of the preset to start from.
    GlassBytes preset;
    // The config's levels.
    GlassLevels levels;
    // 1 for the dark appearance, 0 for the light.
    uint32_t dark;
} GlassResolveRequest;

// One setting of one layer of a preset, as text.
typedef struct GlassFieldEdit {
    GlassBytes name;
    // 0 shared, 1 dark, 2 light.
    uint32_t variant;
    GlassBytes key;
    GlassBytes value;
} GlassFieldEdit;

// What a config option is called and what it is registered with.
typedef struct GlassOptionInfo {
    // The key, UTF-8 without a terminator, in memory that lives as long as the library.
    const uint8_t* key;
    size_t key_len;
    // The value the global level is registered with; negative when it is not set.
    double global_default;
} GlassOptionInfo;

// What the classic shader falls back to for a theme.
typedef struct GlassThemeDefaults {
    float brightness;
    float contrast;
    float saturation;
    float vibrancy;
    float vibrancy_darkness;
    float adaptive_dim;
    float adaptive_boost;
} GlassThemeDefaults;

// Where the classic shader's tone curve first stops rising, and how high it
// is there (`glass_core::tone`): past `at` nothing behind the glass comes out
// darker than `value`.
typedef struct GlassTonePeak {
    float at;
    float value;
} GlassTonePeak;

// Which rebuilds a change of parameters asks for; each 1 or 0.
typedef struct GlassChanges {
    uint32_t field;
    uint32_t lens;
    uint32_t frost;
} GlassChanges;

// How a layer's field, its shadow and its map of cells are laid out.
typedef struct GlassLayout {
    double field_sigma;
    int32_t field_scale;
    int32_t margin;
    int32_t shadow_reach;
    int32_t field_texels[2];
    int32_t shadow_texels[2];
    int32_t cells[2];
    uint32_t initial_fit;
    float blur_sigma;
    int32_t blur_radius;
    uint32_t shadow_on;
    float shadow_sigma;
    int32_t shadow_taps;
    uint32_t small;
} GlassLayout;

// How the frost of a pane is made from the page behind it.
typedef struct GlassFrostPlan {
    int32_t reduction;
    float sigma;
    uint32_t face_applied;
    int32_t passes;
    float pass_sigma;
    int32_t pairs;
} GlassFrostPlan;

// The part of the screen a pane's frost is read from.
typedef struct GlassSampleQuery {
    GlassRect pane;
    double box_x;
    double box_y;
    double source_w;
    double source_h;
    float frost;
    int32_t reach;
} GlassSampleQuery;

// How far glass reaches beyond a pane, in device pixels.
typedef struct GlassReaches {
    int32_t frost;
    int32_t shadow;
    int32_t damage;
    int32_t padding;
    double repaint;
} GlassReaches;

// What the glass shader is given besides the textures and the geometry of a quad.
typedef struct GlassDrawConstants {
    float thickness_per_fit;
    float interior_coverage;
    float solid_coverage;
    float outer_coverage;
    float rim_coverage;
    float deviation_scale;
    uint32_t dispersive;
    float light_direction[2];
} GlassDrawConstants;

// The opacity of the glass and of its shadow in one frame.
typedef struct GlassOpacities {
    float glass;
    float shadow;
} GlassOpacities;

// The measured constants of the current material's rim, for a shader that is
// not made in the core (the one that draws a window's pane). Lengths are
// thirds of a point.
typedef struct GlassRimConstants {
    // Share of the lit lobe's strength the far lobe has.
    float opposite;
    // `(1 - luma)^this`: the lobes fade over a bright body.
    float luma_power;
    // The core of a lobe: its strength, and the length it falls to 1/e in.
    float core[2];
    // The tail of a lobe, likewise.
    float tail[2];
    // How far in the lobes begin.
    float inset;
    // How much darker than the page beside the pane the hairline is.
    float hairline_shade;
    // `across = (1 - along^2)^this`.
    float hairline_power;
    // Where the hairline starts to fade and where it has ended.
    float hairline_width[2];
    // How far outward the page is read for it.
    float hairline_read;
} GlassRimConstants;

// A piece of glass; its face rectangles are `interior_count` of the list
// that comes with it, from `first_interior`.
typedef struct GlassPane {
    GlassRect rect;
    double short_side;
    uint32_t first_interior;
    uint32_t interior_count;
} GlassPane;

// Panes lent to the library for a call, with their face rectangles.
typedef struct GlassPaneList {
    const GlassPane* panes;
    size_t pane_count;
    const GlassRect* interiors;
    size_t interior_count;
} GlassPaneList;

// Room for panes: the library writes into it and says how many it found.
// When there is not enough room it still counts, and the call fails with
// `GLASS_ERR_CAPACITY`, so that the caller can try again with enough.
typedef struct GlassPaneBuffers {
    GlassPane* panes;
    size_t pane_capacity;
    GlassRect* interiors;
    size_t interior_capacity;
    // One byte per cell of the map, if the call makes any.
    uint8_t* fit;
    size_t fit_capacity;
    size_t pane_count;
    size_t interior_count;
    size_t fit_count;
} GlassPaneBuffers;

// The GPU's coarse map of where panes are, and what its cells stand for.
typedef struct GlassMapRequest {
    // `width * height` RGBA texels.
    GlassBytes cells;
    size_t width;
    size_t height;
    int32_t margin;
    int32_t field_scale;
    double sigma;
    double bezel_width;
    double box_w;
    double box_h;
} GlassMapRequest;

// The panes that stand now.
typedef struct GlassWantedRequest {
    // The panes of the last map, if there is one.
    GlassPaneList known;
    uint32_t has_known;
    // Part of the layer redrawn since the map, if any.
    GlassRect unknown;
    uint32_t has_unknown;
    // The layer's box.
    GlassRect whole;
    // 1 when the layer is about the size of what it shows.
    uint32_t small;
} GlassWantedRequest;

// A value moving to a target under a spring, as the library steps it (not
// used by the plugin yet): duration and bounce say how it swings.
typedef struct GlassSpring {
    double position;
    double velocity;
    double target;
    // Seconds of one swing.
    double duration;
    // 0 critical damping, up to 1 no damping, down to -1 creeping.
    double bounce;
} GlassSpring;

// How a pane stretches along its motion and squashes across it.
typedef struct GlassStretch {
    double along;
    double across;
} GlassStretch;

// Where a closing window goes into, or an opening one comes out of: a point
// and the width of the mouth there.
typedef struct GlassMouth {
    double x;
    double y;
    double width;
} GlassMouth;

// Room for a mesh of `cols` by `rows` cells: an x and a y for each of its
// `(cols + 1) * (rows + 1)` points, row by row from the top left.
typedef struct GlassMesh {
    float* points;
    size_t capacity;
    uint32_t cols;
    uint32_t rows;
} GlassMesh;

// How a wobbling window swings: the spring of each point to its place
// (seconds of a swing, bounce), how strongly it is held to its neighbours as
// a share of that, and how far it may trail as a share of the window.
typedef struct GlassWobbleSpec {
    double duration;
    double bounce;
    double coupling;
    double reach;
} GlassWobbleSpec;

// A wobbling window's sheet, kept by the caller: each point's offset from
// its place and its speed, x and y, row by row; the point held, or -1.
// All zeros and -1 is a sheet at rest.
typedef struct GlassWobble {
    double offset[GLASS_WOBBLE_FLOATS];
    double velocity[GLASS_WOBBLE_FLOATS];
    int32_t held;
} GlassWobble;

// A rectangle with continuous corners, centred on the origin, y down.
typedef struct GlassDeclaredRect {
    double width;
    double height;
    // Radii: top left, top right, bottom right, bottom left.
    double radii[4];
} GlassDeclaredRect;

// A shape at a point: its signed distance (negative inside) and outward unit normal.
typedef struct GlassSample {
    double distance;
    double normal[2];
} GlassSample;

// The version of this interface; the header says which one it was written for.
uint32_t glass_abi_version(void);

// How many bytes of `text` are left when one trailing `*` is taken off: Hyprland
// stores dynamic tags with it, and presets are registered without it.
int32_t glass_strip_dynamic_tag_marker(GlassBytes text, size_t* length);

// Name and registered default of option `index` of `kind` (0 float, 1 int).
int32_t glass_option_info(uint32_t kind, uint32_t index, GlassOptionInfo* out);

// What the classic shader falls back to in the dark (1) or light (0) theme.
int32_t glass_theme_defaults(uint32_t dark, GlassThemeDefaults* out);

// Where the classic shader's tone curve, for its dim, boost and brightness,
// first stops rising: the shader holds whatever is brighter behind the glass
// to that height, so that nothing brighter comes out darker.
int32_t glass_tone_peak(float dim, float boost, float brightness, GlassTonePeak* out);

// Every option, resolved through preset, theme and global level; a negative
// value where nothing sets it.
int32_t glass_resolve_options(const GlassResolveRequest* request, GlassValues* out);

// The lens model's parameters for a monitor of this scale, from options already resolved.
int32_t glass_model_params(const GlassValues* options, float scale, GlassParams* out);

// 1 when the options ask for the lens model rather than the classic shader, else 0.
int32_t glass_uses_model(const GlassValues* options);

// Which rebuilds a change from `before` to `after` asks for.
int32_t glass_params_changes(const GlassParams* before, const GlassParams* after, GlassChanges* out);

// Makes the block an empty store.
int32_t glass_store_clear(GlassStore* block);

// Adds one line of the legacy preset form (`name:clear:dark, frost:4`). On a
// line that does not read, answers `GLASS_ERR_PARSE` and says why in `error`.
int32_t glass_store_add_line(GlassStore* block, GlassBytes line, GlassText* error);

// Sets one option of one layer of a preset from its text, making the preset
// if it is not there. Answers 1 when the setting took, 0 when the key or the
// value is not one.
int32_t glass_store_set_field(GlassStore* block, const GlassFieldEdit* edit);

// Says which preset `name` inherits from, making it if it is not there.
int32_t glass_store_set_inherits(GlassStore* block, GlassBytes name, GlassBytes inherits);

// Makes sure a preset called `name` exists.
int32_t glass_store_define(GlassStore* block, GlassBytes name);

// Writes into `committed` the presets a config ends up with: the built-in ones, changed by `user`'s.
int32_t glass_store_commit(const GlassStore* user, GlassStore* committed);

// 1 when a preset is called `name`, else 0.
int32_t glass_store_contains(const GlassStore* block, GlassBytes name);

// 1 when any preset sets the float option `option` to more than zero, else 0.
int32_t glass_store_any_positive(const GlassStore* block, uint32_t option);

// Writes the lookup table the glass shader reads into `out`, which must hold
// `2 * GLASS_LENS_TABLE_SIZE * 4` bytes, and the scale its lean is a fraction of into `scale`.
int32_t glass_lens_table(const GlassLens* lens, uint8_t* out, size_t out_len, double* scale);

// Lays out the field, the shadow and the pane map of a layer of this size.
int32_t glass_layout(const GlassParams* params, double box_w, double box_h, GlassLayout* out);

// Plans the frost of a pane for this deviation, in device pixels.
int32_t glass_frost_plan(float frost, GlassFrostPlan* out);

// The monitor rectangle a pane's frost is read from; of no size where there is none.
int32_t glass_sample_rect(const GlassSampleQuery* query, GlassRect* out);

// The strips of the frame round a solid window whose pane is `outer`: `band`
// is how far the pane reaches beyond the window and `corner` the pane's own
// corner radius, in device pixels.
int32_t glass_frame_strips(const GlassRect* outer, double band, double corner, GlassFrame* out);

// How far glass reaches beyond a pane.
int32_t glass_reaches(const GlassParams* params, int32_t shadow_reach, GlassReaches* out);

// The constants of the lens and the light for the glass shader; `table_scale` is what `glass_lens_table` answered.
int32_t glass_draw_constants(const GlassParams* params, double table_scale, GlassDrawConstants* out);

// The opacity of the glass and of its shadow in a frame where the layer is drawn with this alpha.
int32_t glass_opacities(const GlassParams* params, float alpha, GlassOpacities* out);

// What Apple's own shader is given for the preset `code` names
// (`GlassParams::quartz`); `GLASS_ERR_UNKNOWN` for a code that names none.
int32_t glass_quartz(uint32_t code, GlassQuartz* out);

// What Apple's own shader is given for a tuning (`glass_core::tuning`): the
// glass as its user set it.
int32_t glass_quartz_tuned(const GlassTuning* tuning, GlassQuartz* out);

// Every knob at its default: the glass as it comes.
int32_t glass_tuning_default(GlassTuning* out);

// Knob `index`: its key, words, range and default.
int32_t glass_tuning_knob(uint32_t index, GlassKnob* out);

// Preset `index`: its name and label.
int32_t glass_tuning_preset_info(uint32_t index, GlassTuningPreset* out);

// The tuning of the preset named `name`, over `out`: the preset sets the
// glass, and the motion already in `out` stays.
int32_t glass_tuning_preset(GlassBytes name, GlassTuning* out);

// Reads a file's text over the defaults. Answers how many lines were skipped
// because their key or value did not read.
int32_t glass_tuning_parse(GlassBytes text, GlassTuning* out);

// The file's text for a tuning; with a null `out`, only its length.
int32_t glass_tuning_format(const GlassTuning* tuning, uint8_t* out, size_t capacity, size_t* length);

// Sets knob `index` of a tuning, held to its range, as a file would.
int32_t glass_tuning_set(GlassTuning* tuning, uint32_t index, float value);

// The measured constants of the current material's rim.
int32_t glass_rim_constants(GlassRimConstants* out);

// Reads the panes out of the GPU's map of cells.
int32_t glass_panes_from_cells(const GlassMapRequest* request, GlassPaneBuffers* out);

// The panes that stand now: the last map's, plus what was redrawn since.
int32_t glass_panes_wanted(const GlassWantedRequest* request, GlassPaneBuffers* out);

// How many fragment shaders there are.
uint32_t glass_shader_count(void);

// The name of shader `index`; with a null `out`, only its length.
int32_t glass_shader_name(uint32_t index, uint8_t* out, size_t capacity, size_t* length);

// The source of shader `index`, constants put in; with a null `out`, only its length.
int32_t glass_shader_source(uint32_t index, uint8_t* out, size_t capacity, size_t* length);

// Moves a spring forward `dt` seconds, in place: the exact solution of its
// equation, stable for any `dt`. Retargeting is setting `target`: position
// and velocity carry over.
int32_t glass_spring_step(GlassSpring* spring, double dt);

// The sliding selector's stretch for a speed in pixels a second.
int32_t glass_selector_stretch(double velocity, uint32_t dragging, GlassStretch* out);

// How far past its end a thing dragged by `offset` is shown.
int32_t glass_rubber_band(double offset, double limit, double* out);

// The distance and the outward normal of a declared rectangle at a point.
int32_t glass_declared_sample(const GlassDeclaredRect* shape, double x, double y, GlassSample* out);

// Joins up to four samples with blend distance `k`, gated by the angle between their normals.
int32_t glass_union(const GlassSample* samples, size_t count, double k, GlassSample* out);

// The points of `window` poured into `mouth` at `progress`, 0 the window as
// it is and 1 gone through, into `mesh`. Answers with the number of points.
int32_t glass_genie_mesh(const GlassRect* window, const GlassMouth* mouth, double progress, GlassMesh* mesh);

// Holds the point of the sheet nearest (`u`, `v`), each 0 to 1 across the
// window; a point outside it lets go.
int32_t glass_wobble_hold(GlassWobble* wobble, double u, double v);

// The window went from `from` to `to`: the sheet stays where it was on the
// screen but for the held point, and trails.
int32_t glass_wobble_moved(GlassWobble* wobble, const GlassWobbleSpec* spec, const GlassRect* from, const GlassRect* to);

// Squeezes a window of `width` by `height` towards its middle by `amount`, a
// share of its size, for it to spring out of.
int32_t glass_wobble_squeeze(GlassWobble* wobble, double width, double height, double amount);

// Moves the sheet forward `dt` seconds. Answers 1 when it has come to rest
// (no point a quarter of a pixel from its place), 0 while it moves.
int32_t glass_wobble_step(GlassWobble* wobble, const GlassWobbleSpec* spec, double dt);

// The points of `window` drawn on the sheet, into `mesh`. Answers with the
// number of points.
int32_t glass_wobble_mesh(const GlassWobble* wobble, const GlassRect* window, GlassMesh* mesh);

#ifdef __cplusplus
#define GLASS_STATIC_ASSERT(condition, message) static_assert(condition, message)
#define alignof_(type) alignof(type)
#else
#define GLASS_STATIC_ASSERT(condition, message) _Static_assert(condition, message)
#define alignof_(type) _Alignof(type)
#endif

GLASS_STATIC_ASSERT(sizeof(GlassRect) == 32, "GlassRect has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassRect) == 8, "GlassRect has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassFrame) == 128, "GlassFrame has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassFrame) == 8, "GlassFrame has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassLens) == 56, "GlassLens has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassLens) == 8, "GlassLens has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassLayered) == 48, "GlassLayered has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassLayered) == 4, "GlassLayered has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassRimConstants) == 48, "GlassRimConstants has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassRimConstants) == 4, "GlassRimConstants has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassParams) == 192, "GlassParams has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassParams) == 8, "GlassParams has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassValues) == 256, "GlassValues has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassValues) == 8, "GlassValues has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassLevels) == 768, "GlassLevels has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassLevels) == 8, "GlassLevels has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassBytes) == 16, "GlassBytes has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassBytes) == 8, "GlassBytes has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassStore) == 24, "GlassStore has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassStore) == 8, "GlassStore has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassText) == 260, "GlassText has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassText) == 4, "GlassText has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassResolveRequest) == 808, "GlassResolveRequest has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassResolveRequest) == 8, "GlassResolveRequest has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassFieldEdit) == 56, "GlassFieldEdit has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassFieldEdit) == 8, "GlassFieldEdit has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassOptionInfo) == 24, "GlassOptionInfo has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassOptionInfo) == 8, "GlassOptionInfo has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassThemeDefaults) == 28, "GlassThemeDefaults has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassThemeDefaults) == 4, "GlassThemeDefaults has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassTonePeak) == 8, "GlassTonePeak has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassTonePeak) == 4, "GlassTonePeak has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassChanges) == 12, "GlassChanges has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassChanges) == 4, "GlassChanges has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassLayout) == 72, "GlassLayout has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassLayout) == 8, "GlassLayout has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassFrostPlan) == 24, "GlassFrostPlan has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassFrostPlan) == 4, "GlassFrostPlan has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassSampleQuery) == 72, "GlassSampleQuery has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassSampleQuery) == 8, "GlassSampleQuery has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassReaches) == 24, "GlassReaches has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassReaches) == 8, "GlassReaches has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassDrawConstants) == 36, "GlassDrawConstants has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassDrawConstants) == 4, "GlassDrawConstants has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassOpacities) == 8, "GlassOpacities has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassOpacities) == 4, "GlassOpacities has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassQuartz) == 476, "GlassQuartz has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassQuartz) == 4, "GlassQuartz has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassTuning) == 128, "GlassTuning has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassTuning) == 4, "GlassTuning has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassKnob) == 1064, "GlassKnob has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassKnob) == 4, "GlassKnob has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassTuningPreset) == 520, "GlassTuningPreset has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassTuningPreset) == 4, "GlassTuningPreset has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassPane) == 48, "GlassPane has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassPane) == 8, "GlassPane has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassPaneList) == 32, "GlassPaneList has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassPaneList) == 8, "GlassPaneList has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassPaneBuffers) == 72, "GlassPaneBuffers has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassPaneBuffers) == 8, "GlassPaneBuffers has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassMapRequest) == 72, "GlassMapRequest has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassMapRequest) == 8, "GlassMapRequest has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassWantedRequest) == 120, "GlassWantedRequest has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassWantedRequest) == 8, "GlassWantedRequest has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassSpring) == 40, "GlassSpring has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassSpring) == 8, "GlassSpring has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassStretch) == 16, "GlassStretch has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassStretch) == 8, "GlassStretch has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassMouth) == 24, "GlassMouth has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassMouth) == 8, "GlassMouth has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassMesh) == 24, "GlassMesh has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassMesh) == 8, "GlassMesh has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassWobbleSpec) == 32, "GlassWobbleSpec has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassWobbleSpec) == 8, "GlassWobbleSpec has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassWobble) == 808, "GlassWobble has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassWobble) == 8, "GlassWobble has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassDeclaredRect) == 48, "GlassDeclaredRect has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassDeclaredRect) == 8, "GlassDeclaredRect has a different alignment in Rust");
GLASS_STATIC_ASSERT(sizeof(GlassSample) == 24, "GlassSample has a different size in Rust");
GLASS_STATIC_ASSERT(alignof_(GlassSample) == 8, "GlassSample has a different alignment in Rust");

#ifdef __cplusplus
}
#endif

#endif /* GLASS_FFI_H */
