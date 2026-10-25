//! The functions the C++ plugin calls.
//!
//! Every one answers with an `int32_t`: zero or a positive number for success
//! (some say yes or no with 1 and 0), a negative `GLASS_ERR_*` for failure.
//! Results go to memory the caller owns, through a pointer it passes. Single
//! objects are passed by pointer, and a null one is answered with
//! `GLASS_ERR_NULL`; memory lent as a pointer and a length is read through
//! [`crate::boundary`]. No function keeps a pointer after it returns.
//!
//! This file holds no `unsafe` block. The `no_mangle` attribute that gives
//! each function its C name is what the lint calls unsafe, so the module
//! says so once.

#![expect(
    unsafe_code,
    reason = "the no_mangle attribute that names each function for C is the only unsafe thing here"
)]

use glass_core::geometry::{Rect, frame_strips};
use glass_core::lens::{Lens, build_lens_table};
use glass_core::motion::genie::genie_mesh;
use glass_core::motion::wobble::wobble_mesh;
use glass_core::motion::{rubber_band, selector_stretch};
use glass_core::options::builtin::{DARK_THEME_DEFAULTS, LIGHT_THEME_DEFAULTS};
use glass_core::options::resolve::{Request, resolve};
use glass_core::options::{
    FloatOption, IntOption, ModelParams, Store, StoreView, Variant, changes, model_params,
    uses_model,
};
use glass_core::panes::{CellGrid, MapGeometry, Pane, PaneMap, map_from_cells, wanted_panes};
use glass_core::schedule::{
    LENS_TABLE_SIZE, draw_constants, frost_plan, layout, opacities, reaches, sample_rect,
    widest_frost,
};
use glass_core::shaders;
use glass_core::shape::{self, union_of};
use glass_core::tone;

use crate::boundary::{read_slice, write_slice};
use crate::status::{DONE, Failure, Outcome, guarded, present};
use crate::types::{
    GLASS_ABI_VERSION, GLASS_TEXT_BYTES, GlassBytes, GlassChanges, GlassDeclaredRect,
    GlassDrawConstants, GlassFieldEdit, GlassFrame, GlassFrostPlan, GlassKnob, GlassLayout,
    GlassLens, GlassMapRequest, GlassMesh, GlassMouth, GlassOpacities, GlassOptionInfo, GlassPane,
    GlassPaneBuffers, GlassPaneList, GlassParams, GlassQuartz, GlassReaches, GlassRect,
    GlassResolveRequest, GlassRimConstants, GlassSample, GlassSampleQuery, GlassSpring, GlassStore,
    GlassStretch, GlassText, GlassThemeDefaults, GlassTonePeak, GlassTuning, GlassTuningPreset,
    GlassValues, GlassWantedRequest, GlassWobble, GlassWobbleSpec,
};
use glass_core::tuning::{self, Id, KNOBS, PRESETS, Preset, Tuning};

fn text_of<'a>(bytes: &GlassBytes) -> Result<&'a str, Failure> {
    std::str::from_utf8(read_slice(bytes.data, bytes.len)?).map_err(|_| Failure::Argument)
}

/// The version of this interface; the header says which one it was written for.
#[unsafe(no_mangle)]
pub extern "C" fn glass_abi_version() -> u32 {
    GLASS_ABI_VERSION
}

// ── options ──────────────────────────────────────────────────────────────────

/// How many bytes of `text` are left when one trailing `*` is taken off: Hyprland
/// stores dynamic tags with it, and presets are registered without it.
#[unsafe(no_mangle)]
pub extern "C" fn glass_strip_dynamic_tag_marker(
    text: GlassBytes,
    length: Option<&mut usize>,
) -> i32 {
    guarded(|| {
        let name = text_of(&text)?;
        *present(length)? = glass_core::options::parse::strip_dynamic_tag_marker(name).len();
        DONE
    })
}

/// Name and registered default of option `index` of `kind` (0 float, 1 int).
#[unsafe(no_mangle)]
pub extern "C" fn glass_option_info(
    kind: u32,
    index: u32,
    out: Option<&mut GlassOptionInfo>,
) -> i32 {
    guarded(|| {
        let out = present(out)?;
        let at = usize::try_from(index).map_err(|_| Failure::Argument)?;
        let (key, default) = match kind {
            0 => FloatOption::from_index(at).map(|o| (o.key(), o.global_default())),
            1 => IntOption::from_index(at).map(|o| (o.key(), crate::exports::int_default(o))),
            _ => None,
        }
        .ok_or(Failure::Unknown)?;
        *out = GlassOptionInfo {
            key: key.as_ptr(),
            key_len: key.len(),
            global_default: default,
        };
        DONE
    })
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the colours registered are 32 bits: exact in a double"
)]
fn int_default(option: IntOption) -> f64 {
    option.global_default() as f64
}

/// What the classic shader falls back to in the dark (1) or light (0) theme.
#[unsafe(no_mangle)]
pub extern "C" fn glass_theme_defaults(dark: u32, out: Option<&mut GlassThemeDefaults>) -> i32 {
    guarded(|| {
        let d = if dark != 0 {
            DARK_THEME_DEFAULTS
        } else {
            LIGHT_THEME_DEFAULTS
        };
        *present(out)? = GlassThemeDefaults {
            brightness: d.brightness,
            contrast: d.contrast,
            saturation: d.saturation,
            vibrancy: d.vibrancy,
            vibrancy_darkness: d.vibrancy_darkness,
            adaptive_dim: d.adaptive_dim,
            adaptive_boost: d.adaptive_boost,
        };
        DONE
    })
}

/// Where the classic shader's tone curve, for its dim, boost and brightness,
/// first stops rising: the shader holds whatever is brighter behind the glass
/// to that height, so that nothing brighter comes out darker.
#[unsafe(no_mangle)]
pub extern "C" fn glass_tone_peak(
    dim: f32,
    boost: f32,
    brightness: f32,
    out: Option<&mut GlassTonePeak>,
) -> i32 {
    guarded(|| {
        let peak = tone::peak(dim, boost, brightness);
        *present(out)? = GlassTonePeak {
            at: peak.at,
            value: peak.value,
        };
        DONE
    })
}

/// Every option, resolved through preset, theme and global level; a negative
/// value where nothing sets it.
#[unsafe(no_mangle)]
pub extern "C" fn glass_resolve_options(
    request: Option<&GlassResolveRequest>,
    out: Option<&mut GlassValues>,
) -> i32 {
    guarded(|| {
        let (request, out) = (present(request)?, present(out)?);
        let bytes = read_slice(request.store.data, request.store.len)?;
        let empty = Store::new().to_bytes().map_err(|_| Failure::Store)?;
        let view = StoreView::new(if bytes.is_empty() { &empty } else { bytes })
            .map_err(|_| Failure::Store)?;
        let levels = (&request.levels).into();
        let resolved = resolve(&Request {
            store: view,
            preset: text_of(&request.preset)?,
            dark: request.dark != 0,
            levels: &levels,
        });
        *out = (&resolved).into();
        DONE
    })
}

/// The lens model's parameters for a monitor of this scale, from options already resolved.
#[unsafe(no_mangle)]
pub extern "C" fn glass_model_params(
    options: Option<&GlassValues>,
    scale: f32,
    out: Option<&mut GlassParams>,
) -> i32 {
    guarded(|| {
        *present(out)? = model_params(&present(options)?.into(), scale).into();
        DONE
    })
}

/// 1 when the options ask for the lens model rather than the classic shader, else 0.
#[unsafe(no_mangle)]
pub extern "C" fn glass_uses_model(options: Option<&GlassValues>) -> i32 {
    guarded(|| Ok(i32::from(uses_model(&present(options)?.into()))))
}

/// Which rebuilds a change from `before` to `after` asks for.
#[unsafe(no_mangle)]
pub extern "C" fn glass_params_changes(
    before: Option<&GlassParams>,
    after: Option<&GlassParams>,
    out: Option<&mut GlassChanges>,
) -> i32 {
    guarded(|| {
        let c = changes(&(*present(before)?).into(), &(*present(after)?).into());
        *present(out)? = GlassChanges {
            field: u32::from(c.field),
            lens: u32::from(c.lens),
            frost: u32::from(c.frost),
        };
        DONE
    })
}

// ── presets ──────────────────────────────────────────────────────────────────

/// Reads the store out of the block, runs `edit` on it and writes it back.
fn edit_store(block: Option<&mut GlassStore>, edit: impl FnOnce(&mut Store) -> Outcome) -> Outcome {
    let block = present(block)?;
    let mut store = if block.len == 0 {
        Store::new()
    } else {
        Store::from_bytes(read_slice(block.data, block.len)?).map_err(|_| Failure::Store)?
    };
    let answer = edit(&mut store)?;
    write_store(&store, block)?;
    Ok(answer)
}

fn write_store(store: &Store, block: &mut GlassStore) -> Result<(), Failure> {
    let bytes = store.to_bytes().map_err(|_| Failure::Store)?;
    if bytes.len() > block.capacity {
        return Err(Failure::Capacity);
    }
    write_slice(block.data, block.capacity)?
        .get_mut(..bytes.len())
        .ok_or(Failure::Capacity)?
        .copy_from_slice(&bytes);
    block.len = bytes.len();
    Ok(())
}

/// Makes the block an empty store.
#[unsafe(no_mangle)]
pub extern "C" fn glass_store_clear(block: Option<&mut GlassStore>) -> i32 {
    guarded(|| {
        edit_store(block, |store| {
            *store = Store::new();
            DONE
        })
    })
}

/// Adds one line of the legacy preset form (`name:clear:dark, frost:4`). On a
/// line that does not read, answers `GLASS_ERR_PARSE` and says why in `error`.
#[unsafe(no_mangle)]
pub extern "C" fn glass_store_add_line(
    block: Option<&mut GlassStore>,
    line: GlassBytes,
    error: Option<&mut GlassText>,
) -> i32 {
    guarded(|| {
        let line = text_of(&line)?;
        edit_store(block, |store| match store.add_line(line) {
            Ok(()) => DONE,
            Err(problem) => {
                if let Some(slot) = error {
                    write_message(&problem.to_string(), slot);
                }
                Err(Failure::Parse)
            }
        })
    })
}

fn write_message(message: &str, out: &mut GlassText) {
    let take = message.len().min(GLASS_TEXT_BYTES);
    out.bytes = [0; GLASS_TEXT_BYTES];
    for (slot, byte) in out.bytes.iter_mut().zip(message.bytes().take(take)) {
        *slot = byte;
    }
    out.len = u32::try_from(take).unwrap_or(0);
}

/// Sets one option of one layer of a preset from its text, making the preset
/// if it is not there. Answers 1 when the setting took, 0 when the key or the
/// value is not one.
#[unsafe(no_mangle)]
pub extern "C" fn glass_store_set_field(
    block: Option<&mut GlassStore>,
    edit: Option<&GlassFieldEdit>,
) -> i32 {
    guarded(|| {
        let edit = present(edit)?;
        let variant = match edit.variant {
            0 => Variant::Shared,
            1 => Variant::Dark,
            2 => Variant::Light,
            _ => return Err(Failure::Argument),
        };
        let (name, key, value) = (
            text_of(&edit.name)?,
            text_of(&edit.key)?,
            text_of(&edit.value)?,
        );
        edit_store(block, |store| {
            Ok(i32::from(store.set_field(name, variant, key, value)))
        })
    })
}

/// Says which preset `name` inherits from, making it if it is not there.
#[unsafe(no_mangle)]
pub extern "C" fn glass_store_set_inherits(
    block: Option<&mut GlassStore>,
    name: GlassBytes,
    inherits: GlassBytes,
) -> i32 {
    guarded(|| {
        let (name, inherits) = (text_of(&name)?, text_of(&inherits)?);
        edit_store(block, |store| {
            store.set_inherits(name, inherits);
            DONE
        })
    })
}

/// Makes sure a preset called `name` exists.
#[unsafe(no_mangle)]
pub extern "C" fn glass_store_define(block: Option<&mut GlassStore>, name: GlassBytes) -> i32 {
    guarded(|| {
        let name = text_of(&name)?;
        edit_store(block, |store| {
            store.define(name);
            DONE
        })
    })
}

/// Writes into `committed` the presets a config ends up with: the built-in ones, changed by `user`'s.
#[unsafe(no_mangle)]
pub extern "C" fn glass_store_commit(
    user: Option<&GlassStore>,
    committed: Option<&mut GlassStore>,
) -> i32 {
    guarded(|| {
        let user = present(user)?;
        let user = if user.len == 0 {
            Store::new()
        } else {
            Store::from_bytes(read_slice(user.data, user.len)?).map_err(|_| Failure::Store)?
        };
        write_store(&Store::committed(&user), present(committed)?)?;
        DONE
    })
}

fn view_of<'a>(block: &'a GlassStore, empty: &'a [u8]) -> Result<StoreView<'a>, Failure> {
    let bytes = read_slice(block.data, block.len)?;
    StoreView::new(if bytes.is_empty() { empty } else { bytes }).map_err(|_| Failure::Store)
}

/// 1 when a preset is called `name`, else 0.
#[unsafe(no_mangle)]
pub extern "C" fn glass_store_contains(block: Option<&GlassStore>, name: GlassBytes) -> i32 {
    guarded(|| {
        let empty = Store::new().to_bytes().map_err(|_| Failure::Store)?;
        Ok(i32::from(
            view_of(present(block)?, &empty)?.contains(text_of(&name)?),
        ))
    })
}

/// 1 when any preset sets the float option `option` to more than zero, else 0.
#[unsafe(no_mangle)]
pub extern "C" fn glass_store_any_positive(block: Option<&GlassStore>, option: u32) -> i32 {
    guarded(|| {
        let option = usize::try_from(option)
            .ok()
            .and_then(FloatOption::from_index)
            .ok_or(Failure::Unknown)?;
        let empty = Store::new().to_bytes().map_err(|_| Failure::Store)?;
        Ok(i32::from(
            view_of(present(block)?, &empty)?.any_float_positive(option),
        ))
    })
}

// ── lens ─────────────────────────────────────────────────────────────────────

/// Writes the lookup table the glass shader reads into `out`, which must hold
/// `2 * GLASS_LENS_TABLE_SIZE * 4` bytes, and the scale its lean is a fraction of into `scale`.
#[unsafe(no_mangle)]
pub extern "C" fn glass_lens_table(
    lens: Option<&GlassLens>,
    out: *mut u8,
    out_len: usize,
    scale: Option<&mut f64>,
) -> i32 {
    guarded(|| {
        let lens = Lens::from(*present(lens)?);
        let table = build_lens_table(&lens, LENS_TABLE_SIZE).map_err(|_| Failure::Argument)?;
        let target = write_slice(out, out_len)?;
        if target.len() < table.texels.len() {
            return Err(Failure::Capacity);
        }
        target
            .get_mut(..table.texels.len())
            .ok_or(Failure::Capacity)?
            .copy_from_slice(&table.texels);
        *present(scale)? = table.deviation_scale;
        DONE
    })
}

// ── schedule ─────────────────────────────────────────────────────────────────

/// Lays out the field, the shadow and the pane map of a layer of this size.
#[unsafe(no_mangle)]
pub extern "C" fn glass_layout(
    params: Option<&GlassParams>,
    box_w: f64,
    box_h: f64,
    out: Option<&mut GlassLayout>,
) -> i32 {
    guarded(|| {
        *present(out)? = layout(&(*present(params)?).into(), [box_w, box_h]).into();
        DONE
    })
}

/// Plans the frost of a pane for this deviation, in device pixels.
#[unsafe(no_mangle)]
pub extern "C" fn glass_frost_plan(frost: f32, out: Option<&mut GlassFrostPlan>) -> i32 {
    guarded(|| {
        *present(out)? = frost_plan(frost).into();
        DONE
    })
}

/// The monitor rectangle a pane's frost is read from; of no size where there is none.
#[unsafe(no_mangle)]
pub extern "C" fn glass_sample_rect(
    query: Option<&GlassSampleQuery>,
    out: Option<&mut GlassRect>,
) -> i32 {
    guarded(|| {
        *present(out)? = sample_rect(&present(query)?.into()).into();
        DONE
    })
}

/// The strips of the frame round a solid window whose pane is `outer`: `band`
/// is how far the pane reaches beyond the window and `corner` the pane's own
/// corner radius, in device pixels.
#[unsafe(no_mangle)]
pub extern "C" fn glass_frame_strips(
    outer: Option<&GlassRect>,
    band: f64,
    corner: f64,
    out: Option<&mut GlassFrame>,
) -> i32 {
    guarded(|| {
        let strips = frame_strips(&(*present(outer)?).into(), band, corner);
        *present(out)? = GlassFrame {
            strips: strips.map(Into::into),
        };
        DONE
    })
}

/// How far glass reaches beyond a pane.
#[unsafe(no_mangle)]
pub extern "C" fn glass_reaches(
    params: Option<&GlassParams>,
    shadow_reach: i32,
    out: Option<&mut GlassReaches>,
) -> i32 {
    guarded(|| {
        let params: ModelParams = (*present(params)?).into();
        *present(out)? =
            reaches(widest_frost(&params), shadow_reach, params.lens.bezel_width).into();
        DONE
    })
}

/// The constants of the lens and the light for the glass shader; `table_scale` is what `glass_lens_table` answered.
#[unsafe(no_mangle)]
pub extern "C" fn glass_draw_constants(
    params: Option<&GlassParams>,
    table_scale: f64,
    out: Option<&mut GlassDrawConstants>,
) -> i32 {
    guarded(|| {
        *present(out)? = draw_constants(&(*present(params)?).into(), table_scale).into();
        DONE
    })
}

/// The opacity of the glass and of its shadow in a frame where the layer is drawn with this alpha.
#[unsafe(no_mangle)]
pub extern "C" fn glass_opacities(
    params: Option<&GlassParams>,
    alpha: f32,
    out: Option<&mut GlassOpacities>,
) -> i32 {
    guarded(|| {
        let (glass, shadow) = opacities(&(*present(params)?).into(), alpha);
        *present(out)? = GlassOpacities { glass, shadow };
        DONE
    })
}

/// What Apple's own shader is given for the preset `code` names
/// (`GlassParams::quartz`); `GLASS_ERR_UNKNOWN` for a code that names none.
#[unsafe(no_mangle)]
pub extern "C" fn glass_quartz(code: u32, out: Option<&mut GlassQuartz>) -> i32 {
    guarded(|| {
        let kind = glass_core::quartz::Kind::from_code(code).ok_or(Failure::Unknown)?;
        let block = glass_core::quartz::Block::of(kind);
        *present(out)? = GlassQuartz {
            block: block.std140(),
            scale_ref: glass_core::quartz::SCALE_REF,
            reach: glass_core::quartz::reach(&block),
            blur: block.blur_radius,
        };
        DONE
    })
}

/// What Apple's own shader is given for a tuning (`glass_core::tuning`): the
/// glass as its user set it.
#[unsafe(no_mangle)]
pub extern "C" fn glass_quartz_tuned(
    tuning: Option<&GlassTuning>,
    out: Option<&mut GlassQuartz>,
) -> i32 {
    guarded(|| {
        let block = glass_core::quartz::Block::tuned(&Tuning::from(present(tuning)?));
        *present(out)? = GlassQuartz {
            block: block.std140(),
            scale_ref: glass_core::quartz::SCALE_REF,
            reach: glass_core::quartz::reach(&block),
            blur: block.blur_radius,
        };
        DONE
    })
}

// ── tuning ───────────────────────────────────────────────────────────────────

/// Every knob at its default: the glass as it comes.
#[unsafe(no_mangle)]
pub extern "C" fn glass_tuning_default(out: Option<&mut GlassTuning>) -> i32 {
    guarded(|| {
        *present(out)? = GlassTuning::from(&Tuning::DEFAULT);
        DONE
    })
}

/// Knob `index`: its key, words, range and default.
#[unsafe(no_mangle)]
pub extern "C" fn glass_tuning_knob(index: u32, out: Option<&mut GlassKnob>) -> i32 {
    guarded(|| {
        let knob = KNOBS
            .get(usize::try_from(index).map_err(|_| Failure::Argument)?)
            .ok_or(Failure::Unknown)?;
        let slot = present(out)?;
        write_message(knob.key, &mut slot.key);
        write_message(knob.label, &mut slot.label);
        write_message(knob.help, &mut slot.help);
        write_message(knob.unit, &mut slot.unit);
        slot.group = knob.group.code();
        slot.shown = knob.shown.code();
        slot.min = knob.min;
        slot.max = knob.max;
        slot.step = knob.step;
        slot.default_value = knob.default;
        DONE
    })
}

/// Preset `index`: its name and label.
#[unsafe(no_mangle)]
pub extern "C" fn glass_tuning_preset_info(index: u32, out: Option<&mut GlassTuningPreset>) -> i32 {
    guarded(|| {
        let preset = PRESETS
            .get(usize::try_from(index).map_err(|_| Failure::Argument)?)
            .ok_or(Failure::Unknown)?;
        let slot = present(out)?;
        write_message(preset.name, &mut slot.name);
        write_message(preset.label, &mut slot.label);
        DONE
    })
}

/// The tuning of the preset named `name`, over `out`: the preset sets the
/// glass, and the motion already in `out` stays.
#[unsafe(no_mangle)]
pub extern "C" fn glass_tuning_preset(name: GlassBytes, out: Option<&mut GlassTuning>) -> i32 {
    guarded(|| {
        let preset = Preset::named(text_of(&name)?).ok_or(Failure::Unknown)?;
        let slot = present(out)?;
        let kept = Tuning::from(&*slot);
        *slot = GlassTuning::from(&preset.tuning().with_motion_of(&kept));
        DONE
    })
}

/// Reads a file's text over the defaults. Answers how many lines were skipped
/// because their key or value did not read.
#[unsafe(no_mangle)]
pub extern "C" fn glass_tuning_parse(text: GlassBytes, out: Option<&mut GlassTuning>) -> i32 {
    guarded(|| {
        let (tuning, skipped) = tuning::parse(text_of(&text)?);
        *present(out)? = GlassTuning::from(&tuning);
        Ok(i32::try_from(skipped).unwrap_or(i32::MAX))
    })
}

/// The file's text for a tuning; with a null `out`, only its length.
#[unsafe(no_mangle)]
pub extern "C" fn glass_tuning_format(
    tuning: Option<&GlassTuning>,
    out: *mut u8,
    capacity: usize,
    length: Option<&mut usize>,
) -> i32 {
    guarded(|| {
        let text = tuning::format(&Tuning::from(present(tuning)?));
        write_text(&text, out, capacity, length)
    })
}

/// Sets knob `index` of a tuning, held to its range, as a file would.
#[unsafe(no_mangle)]
pub extern "C" fn glass_tuning_set(
    tuning: Option<&mut GlassTuning>,
    index: u32,
    value: f32,
) -> i32 {
    guarded(|| {
        let id = Id::from_index(usize::try_from(index).map_err(|_| Failure::Argument)?)
            .ok_or(Failure::Unknown)?;
        let slot = present(tuning)?;
        let mut held = Tuning::from(&*slot);
        held.set(id, value);
        *slot = GlassTuning::from(&held);
        DONE
    })
}

/// The measured constants of the current material's rim.
#[unsafe(no_mangle)]
pub extern "C" fn glass_rim_constants(out: Option<&mut GlassRimConstants>) -> i32 {
    guarded(|| {
        let rim = glass_core::material::G27.light.rim;
        let hairline = glass_core::material::g27_hairline(true);
        let pair = |values: [f64; 2]| values.map(glass_core::num::to_f32);
        *present(out)? = GlassRimConstants {
            opposite: glass_core::num::to_f32(rim.opposite),
            luma_power: glass_core::num::to_f32(rim.luma_power),
            core: pair(rim.core),
            tail: pair(rim.tail),
            inset: glass_core::num::to_f32(rim.inset * 3.0),
            hairline_shade: glass_core::num::to_f32(hairline.shade),
            hairline_power: glass_core::num::to_f32(hairline.across_power),
            hairline_width: pair(hairline.width),
            hairline_read: glass_core::num::to_f32(hairline.read_offset),
        };
        DONE
    })
}

// ── panes ────────────────────────────────────────────────────────────────────

fn panes_of(list: &GlassPaneList) -> Result<Vec<Pane>, Failure> {
    let panes = read_slice(list.panes, list.pane_count)?;
    let interiors = read_slice(list.interiors, list.interior_count)?;
    panes
        .iter()
        .map(|pane| {
            let first = usize::try_from(pane.first_interior).map_err(|_| Failure::Argument)?;
            let count = usize::try_from(pane.interior_count).map_err(|_| Failure::Argument)?;
            let rects = interiors
                .get(first..first.saturating_add(count))
                .ok_or(Failure::Argument)?;
            Ok(Pane {
                rect: pane.rect.into(),
                short_side: pane.short_side,
                interiors: rects.iter().map(|r| (*r).into()).collect(),
            })
        })
        .collect()
}

/// Counts what there is for the caller, and copies it as far as there is room.
fn write_panes(panes: &[Pane], fit: &[u8], out: &mut GlassPaneBuffers) -> Result<(), Failure> {
    let mut interiors = Vec::new();
    let list: Vec<GlassPane> = panes
        .iter()
        .map(|p| GlassPane::from_pane(p, &mut interiors))
        .collect();
    out.pane_count = list.len();
    out.interior_count = interiors.len();
    out.fit_count = fit.len();
    if list.len() > out.pane_capacity
        || interiors.len() > out.interior_capacity
        || fit.len() > out.fit_capacity
    {
        return Err(Failure::Capacity);
    }
    copy_into(write_slice(out.panes, out.pane_capacity)?, &list)?;
    copy_into(
        write_slice(out.interiors, out.interior_capacity)?,
        &interiors,
    )?;
    copy_into(write_slice(out.fit, out.fit_capacity)?, fit)
}

fn copy_into<T: Copy>(target: &mut [T], source: &[T]) -> Result<(), Failure> {
    target
        .get_mut(..source.len())
        .ok_or(Failure::Capacity)?
        .copy_from_slice(source);
    Ok(())
}

/// Reads the panes out of the GPU's map of cells.
#[unsafe(no_mangle)]
pub extern "C" fn glass_panes_from_cells(
    request: Option<&GlassMapRequest>,
    out: Option<&mut GlassPaneBuffers>,
) -> i32 {
    guarded(|| {
        let (request, out) = (present(request)?, present(out)?);
        let grid = CellGrid {
            cells: read_slice(request.cells.data, request.cells.len)?,
            width: request.width,
            height: request.height,
        };
        let geometry = MapGeometry {
            margin: request.margin,
            field_scale: request.field_scale,
            sigma: request.sigma,
            bezel_width: request.bezel_width,
            box_size: (request.box_w, request.box_h),
        };
        let PaneMap { panes, fit } =
            map_from_cells(&grid, &geometry).map_err(|_| Failure::Argument)?;
        write_panes(&panes, &fit, out)?;
        DONE
    })
}

/// The panes that stand now: the last map's, plus what was redrawn since.
#[unsafe(no_mangle)]
pub extern "C" fn glass_panes_wanted(
    request: Option<&GlassWantedRequest>,
    out: Option<&mut GlassPaneBuffers>,
) -> i32 {
    guarded(|| {
        let (request, out) = (present(request)?, present(out)?);
        let known = if request.has_known != 0 {
            Some(panes_of(&request.known)?)
        } else {
            None
        };
        let unknown = (request.has_unknown != 0).then(|| request.unknown.into());
        let wanted = wanted_panes(
            known.as_deref(),
            unknown.as_ref(),
            &request.whole.into(),
            request.small != 0,
        );
        write_panes(&wanted, &[], out)?;
        DONE
    })
}

// ── shaders ──────────────────────────────────────────────────────────────────

/// How many fragment shaders there are.
#[unsafe(no_mangle)]
pub extern "C" fn glass_shader_count() -> u32 {
    u32::try_from(shaders::names().len()).unwrap_or(0)
}

fn write_text(content: &str, out: *mut u8, capacity: usize, length: Option<&mut usize>) -> Outcome {
    *present(length)? = content.len();
    if out.is_null() {
        return DONE;
    }
    let target = write_slice(out, capacity)?;
    target
        .get_mut(..content.len())
        .ok_or(Failure::Capacity)?
        .copy_from_slice(content.as_bytes());
    DONE
}

/// The name of shader `index`; with a null `out`, only its length.
#[unsafe(no_mangle)]
pub extern "C" fn glass_shader_name(
    index: u32,
    out: *mut u8,
    capacity: usize,
    length: Option<&mut usize>,
) -> i32 {
    guarded(|| {
        let names = shaders::names();
        let name = names
            .get(usize::try_from(index).map_err(|_| Failure::Argument)?)
            .ok_or(Failure::Unknown)?;
        write_text(name, out, capacity, length)
    })
}

/// The source of shader `index`, constants put in; with a null `out`, only its length.
#[unsafe(no_mangle)]
pub extern "C" fn glass_shader_source(
    index: u32,
    out: *mut u8,
    capacity: usize,
    length: Option<&mut usize>,
) -> i32 {
    guarded(|| {
        let names = shaders::names();
        let name = names
            .get(usize::try_from(index).map_err(|_| Failure::Argument)?)
            .ok_or(Failure::Unknown)?;
        let source = shaders::source(name).ok_or(Failure::Unknown)?;
        write_text(&source, out, capacity, length)
    })
}

// ── motion and shapes: not used by the plugin yet ────────────────────────────

/// Moves a spring forward `dt` seconds, in place: the exact solution of its
/// equation, stable for any `dt`. Retargeting is setting `target`: position
/// and velocity carry over.
#[unsafe(no_mangle)]
pub extern "C" fn glass_spring_step(spring: Option<&mut GlassSpring>, dt: f64) -> i32 {
    guarded(|| {
        let spring = present(spring)?;
        let mut stepped = spring.to_spring();
        stepped.step(dt);
        spring.position = stepped.position;
        spring.velocity = stepped.velocity;
        DONE
    })
}

/// The sliding selector's stretch for a speed in pixels a second.
#[unsafe(no_mangle)]
pub extern "C" fn glass_selector_stretch(
    velocity: f64,
    dragging: u32,
    out: Option<&mut GlassStretch>,
) -> i32 {
    guarded(|| {
        *present(out)? = selector_stretch(velocity, dragging != 0).into();
        DONE
    })
}

/// How far past its end a thing dragged by `offset` is shown.
#[unsafe(no_mangle)]
pub extern "C" fn glass_rubber_band(offset: f64, limit: f64, out: Option<&mut f64>) -> i32 {
    guarded(|| {
        *present(out)? = rubber_band(offset, limit);
        DONE
    })
}

/// The distance and the outward normal of a declared rectangle at a point.
#[unsafe(no_mangle)]
pub extern "C" fn glass_declared_sample(
    shape: Option<&GlassDeclaredRect>,
    x: f64,
    y: f64,
    out: Option<&mut GlassSample>,
) -> i32 {
    guarded(|| {
        let shape = present(shape)?.to_shape();
        *present(out)? = GlassSample {
            distance: shape.distance([x, y]),
            normal: shape.normal([x, y]),
        };
        DONE
    })
}

/// Joins up to four samples with blend distance `k`, gated by the angle between their normals.
#[unsafe(no_mangle)]
pub extern "C" fn glass_union(
    samples: *const GlassSample,
    count: usize,
    k: f64,
    out: Option<&mut GlassSample>,
) -> i32 {
    guarded(|| {
        let samples: Vec<shape::Sample> = read_slice(samples, count)?
            .iter()
            .map(|s| (*s).into())
            .collect();
        *present(out)? = union_of(&samples, k)
            .map(Into::into)
            .ok_or(Failure::Argument)?;
        DONE
    })
}

// ── genie and wobbling windows ───────────────────────────────────────────────

/// The mesh's cells and the room for its points.
fn mesh_room<'a>(mesh: &mut GlassMesh) -> Result<((usize, usize), &'a mut [f32]), Failure> {
    let cells = (
        usize::try_from(mesh.cols).map_err(|_| Failure::Argument)?,
        usize::try_from(mesh.rows).map_err(|_| Failure::Argument)?,
    );
    Ok((cells, write_slice(mesh.points, mesh.capacity)?))
}

fn mesh_answer(points: Result<usize, glass_core::motion::genie::MeshError>) -> Outcome {
    match points {
        Ok(n) => i32::try_from(n).map_err(|_| Failure::Capacity),
        Err(glass_core::motion::genie::MeshError::Cells(..)) => Err(Failure::Argument),
        Err(glass_core::motion::genie::MeshError::Room(..)) => Err(Failure::Capacity),
    }
}

/// The points of `window` poured into `mouth` at `progress`, 0 the window as
/// it is and 1 gone through, into `mesh`. Answers with the number of points.
#[unsafe(no_mangle)]
pub extern "C" fn glass_genie_mesh(
    window: Option<&GlassRect>,
    mouth: Option<&GlassMouth>,
    progress: f64,
    mesh: Option<&mut GlassMesh>,
) -> i32 {
    guarded(|| {
        let window = Rect::from(*present(window)?);
        let mouth = (*present(mouth)?).into();
        let (cells, out) = mesh_room(present(mesh)?)?;
        mesh_answer(genie_mesh(&window, &mouth, progress, cells, out))
    })
}

/// Holds the point of the sheet nearest (`u`, `v`), each 0 to 1 across the
/// window; a point outside it lets go.
#[unsafe(no_mangle)]
pub extern "C" fn glass_wobble_hold(wobble: Option<&mut GlassWobble>, u: f64, v: f64) -> i32 {
    guarded(|| {
        let wobble = present(wobble)?;
        let mut w = wobble.sheet();
        w.hold_at(u, v);
        wobble.keep(&w);
        DONE
    })
}

/// The window went from `from` to `to`: the sheet stays where it was on the
/// screen but for the held point, and trails.
#[unsafe(no_mangle)]
pub extern "C" fn glass_wobble_moved(
    wobble: Option<&mut GlassWobble>,
    spec: Option<&GlassWobbleSpec>,
    from: Option<&GlassRect>,
    to: Option<&GlassRect>,
) -> i32 {
    guarded(|| {
        let wobble = present(wobble)?;
        let spec = (*present(spec)?).into();
        let mut w = wobble.sheet();
        w.moved(&(*present(from)?).into(), &(*present(to)?).into(), &spec);
        wobble.keep(&w);
        DONE
    })
}

/// Squeezes a window of `width` by `height` towards its middle by `amount`, a
/// share of its size, for it to spring out of.
#[unsafe(no_mangle)]
pub extern "C" fn glass_wobble_squeeze(
    wobble: Option<&mut GlassWobble>,
    width: f64,
    height: f64,
    amount: f64,
) -> i32 {
    guarded(|| {
        let wobble = present(wobble)?;
        let mut w = wobble.sheet();
        w.squeeze((width, height), amount);
        wobble.keep(&w);
        DONE
    })
}

/// Moves the sheet forward `dt` seconds. Answers 1 when it has come to rest
/// (no point a quarter of a pixel from its place), 0 while it moves.
#[unsafe(no_mangle)]
pub extern "C" fn glass_wobble_step(
    wobble: Option<&mut GlassWobble>,
    spec: Option<&GlassWobbleSpec>,
    dt: f64,
) -> i32 {
    guarded(|| {
        let wobble = present(wobble)?;
        let spec = (*present(spec)?).into();
        let mut w = wobble.sheet();
        w.step(dt, &spec);
        wobble.keep(&w);
        Ok(i32::from(w.settled(0.25)))
    })
}

/// The points of `window` drawn on the sheet, into `mesh`. Answers with the
/// number of points.
#[unsafe(no_mangle)]
pub extern "C" fn glass_wobble_mesh(
    wobble: Option<&GlassWobble>,
    window: Option<&GlassRect>,
    mesh: Option<&mut GlassMesh>,
) -> i32 {
    guarded(|| {
        let w = present(wobble)?.sheet();
        let window = Rect::from(*present(window)?);
        let (cells, out) = mesh_room(present(mesh)?)?;
        mesh_answer(wobble_mesh(&w, &window, cells, out))
    })
}
