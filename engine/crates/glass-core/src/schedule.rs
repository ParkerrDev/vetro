//! The schedule of the passes: which resolution the frost and the field are
//! made at, how wide their margins are, how the Gaussians are cut into taps,
//! and the constants the glass shader is given.
//!
//! The arithmetic keeps the precision it has always had: lengths of the lens
//! are `f64`, what is handed to the GPU as a uniform is `f32`, and a number
//! that was a `float` stays one, so that the picture does not move.

use crate::lens::field_sigma;
use crate::num::{ceil_f32_i32, ceil_i32, int_f32, to_f32, unit_to_byte};
use crate::options::ModelParams;

/// Side of one texel of the shadow, in device pixels.
pub const SHADOW_PIXELS: i32 = 8;
/// Most device pixels of field kept around the layer's box.
pub const MAX_MARGIN: i32 = 128;
/// The field's margin is a multiple of this.
pub const MARGIN_STEP: i32 = 32;
/// Most filtered fetches a side of the frost's Gaussian takes in one pass.
pub const MAX_BLUR_PAIRS: i32 = 8;
/// Texels in a row of the lens table.
pub const LENS_TABLE_SIZE: usize = 1024;
/// Field texels up to which a layer counts as small: a bar across a 2560 pixel
/// screen is 53 thousand, a card 40 thousand, a full-screen surface 330 thousand.
/// A small layer whose outline is moving is rebuilt on every commit, so its glass
/// is a frame behind only when the outline starts to move; one whose outline
/// stands still is asked first, as a large one is (a panel fading in, or text
/// changing inside a card, needs no new field: on the integrated GPU here the
/// field was most of what a panel's glass cost on a frame, 2026-10-23).
pub const SMALL_FIELD_TEXELS: f64 = 120_000.0;
/// A layer whose outline moves on every frame, a card sliding inside it, has
/// its field rebuilt at most this often (milliseconds): between rebuilds the
/// glass keeps the outline it had.
pub const MIN_BUILD_INTERVAL_MS: u32 = 30;
/// Blurred coverage that only a pixel a whole bezel inside the outline reaches.
pub const INTERIOR_COVERAGE: f32 = 0.9995;
/// Blurred coverage of a pixel two thirds of a deviation inside: its own alpha
/// need not be read.
pub const SOLID_COVERAGE: f32 = 0.75;

/// Device pixels of screen one texel of the frost stands for: the stronger the
/// frost, the fewer texels it needs. 1, 2, 4 or 8. The Gaussian left to do on
/// the reduced copy is at least a texel and a half wide.
#[must_use]
pub fn frost_reduction(frost_sigma: f32) -> i32 {
    if frost_sigma >= 14.0 {
        8
    } else if frost_sigma >= 7.0 {
        4
    } else if frost_sigma >= 3.0 {
        2
    } else {
        1
    }
}

/// Device pixels one texel of the field stands for, given its blur: 1, 2 or 4.
#[must_use]
pub fn field_reduction(sigma: f64) -> i32 {
    if sigma >= 8.0 {
        4
    } else if sigma >= 4.0 {
        2
    } else {
        1
    }
}

/// Device pixels a pane's frost reads beyond the pane. Past two and a half
/// deviations a Gaussian has an eightieth of its weight left.
#[must_use]
pub fn frost_reach(frost: f32) -> i32 {
    if frost > 0.25 {
        ceil_f32_i32(2.5_f32 * frost)
    } else {
        0
    }
}

fn round_up_to(value: i32, step: i32) -> i32 {
    value
        .saturating_add(step)
        .saturating_sub(1)
        .checked_div(step)
        .unwrap_or(0)
        .saturating_mul(step)
}

/// How a layer's field, its shadow and its map of cells are laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// Deviation of the blur that turns coverage into distance, device pixels.
    pub field_sigma: f64,
    /// Device pixels per texel of the field.
    pub field_scale: i32,
    /// Device pixels of field kept around the layer's box.
    pub margin: i32,
    /// Device pixels a pane's shadow reaches beyond it.
    pub shadow_reach: i32,
    /// Texels of the field that hold data: across, down.
    pub field_texels: [i32; 2],
    /// Texels of the shadow: across, down.
    pub shadow_texels: [i32; 2],
    /// Cells of the pane map: across, down.
    pub cells: [i32; 2],
    /// The fit byte of the one pane assumed until the map is read: the layer's box.
    pub initial_fit: u8,
    /// Deviation of the field's blur in field texels.
    pub blur_sigma: f32,
    /// Taps a side of that blur.
    pub blur_radius: i32,
    /// Whether the layer has a shadow.
    pub shadow_on: bool,
    /// Deviation of the shadow's blur in field texels.
    pub shadow_sigma: f32,
    /// Taps a side of that blur.
    pub shadow_taps: i32,
    /// The layer is about the size of what it shows.
    pub small: bool,
}

/// Lays out the field for a layer of this size.
#[must_use]
pub fn layout(params: &ModelParams, box_size: [f64; 2]) -> Layout {
    let bezel = params.lens.bezel_width;
    let sigma = field_sigma(bezel);
    let field_scale = field_reduction(sigma);
    let shadow_on = params.shadow_opacity > 0.0 && params.shadow_radius > 0.0;
    let blur_reach = ceil_i32(4.5 * sigma).saturating_add(field_scale);
    // The shadow's deviation is half its blur radius. Two deviations out it is a
    // fortieth of its darkest, under one level of 8 bits at any opacity in use.
    let shadow_reach = if shadow_on {
        ceil_f32_i32(params.shadow_radius + params.shadow_offset.abs())
    } else {
        0
    };
    let margin = round_up_to(blur_reach.max(shadow_reach), MARGIN_STEP).min(MAX_MARGIN);
    let span = |side: f64| ceil_i32(side).saturating_add(margin.saturating_mul(2));
    let (span_x, span_y) = (span(box_size[0]), span(box_size[1]));
    let texels = |span: i32, per: i32| ceil_i32(f64::from(span) / f64::from(per));
    let field_texels = [texels(span_x, field_scale), texels(span_y, field_scale)];
    let blur_sigma = to_f32(sigma / f64::from(field_scale));
    let shadow_sigma = (0.5_f32 * params.shadow_radius / int_f32(field_scale)).max(0.5);
    Layout {
        field_sigma: sigma,
        field_scale,
        margin,
        shadow_reach: shadow_reach.min(margin),
        field_texels,
        shadow_texels: [texels(span_x, SHADOW_PIXELS), texels(span_y, SHADOW_PIXELS)],
        cells: [texels(span_x, 8), texels(span_y, 8)],
        initial_fit: unit_to_byte(initial_room(bezel, box_size)),
        blur_sigma,
        blur_radius: ceil_f32_i32(4.0_f32 * blur_sigma).clamp(1, 32),
        shadow_on,
        shadow_sigma,
        shadow_taps: ceil_f32_i32(3.0_f32 * shadow_sigma).clamp(1, 48),
        small: f64::from(field_texels[0]) * f64::from(field_texels[1]) <= SMALL_FIELD_TEXELS,
    }
}

/// Until the map is read nothing is known of the shape: the lens is fitted to the box itself.
fn initial_room(bezel_width: f64, box_size: [f64; 2]) -> f64 {
    if bezel_width > 0.0 {
        box_size[0].min(box_size[1]) / (4.0 * bezel_width)
    } else {
        1.0
    }
}

/// The deviation that sizes what a pane reads of the page: the wide copy's
/// where the frost has one (the current material's cloud), and the frost's own
/// otherwise.
#[must_use]
pub fn widest_frost(params: &ModelParams) -> f32 {
    params.frost.max(params.layered.frost_wide)
}

/// How the frost of a pane is made from the page behind it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrostPlan {
    /// Device pixels per texel of the reduced copy.
    pub reduction: i32,
    /// Deviation left to blur on the reduced copy, in its texels.
    pub sigma: f32,
    /// Whether there is a Gaussian to run, and with it the face is applied in its last pass.
    pub face_applied: bool,
    /// How many times the separable Gaussian is run.
    pub passes: i32,
    /// Deviation of each run.
    pub pass_sigma: f32,
    /// Filtered fetches a side of each run.
    pub pairs: i32,
}

/// Plans the frost for this deviation (device pixels).
///
/// The reduction and the filtered read of the glass shader already spread light
/// by about half a texel each way: the Gaussian supplies the rest of the frost.
/// More than [`MAX_BLUR_PAIRS`] filtered fetches a side would cut the kernel
/// short: it is run more than once instead.
#[must_use]
pub fn frost_plan(frost: f32) -> FrostPlan {
    let reduction = frost_reduction(frost);
    let wanted = frost / int_f32(reduction);
    let sigma = (wanted * wanted - if reduction > 1 { 0.25_f32 } else { 0.0_f32 })
        .max(0.0)
        .sqrt();
    let pairs_wanted = 3.0_f32 * sigma / 2.0_f32;
    let per_pass = pairs_wanted / int_f32(MAX_BLUR_PAIRS);
    let passes = ceil_f32_i32(per_pass * per_pass).clamp(1, 4);
    let pass_sigma = sigma / int_f32(passes).sqrt();
    FrostPlan {
        reduction,
        sigma,
        face_applied: sigma > 0.3,
        passes,
        pass_sigma,
        pairs: ceil_f32_i32(3.0_f32 * pass_sigma / 2.0_f32).clamp(1, MAX_BLUR_PAIRS),
    }
}

/// The part of the screen a pane's frost is read from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleQuery {
    /// The pane, relative to the layer's box.
    pub pane: crate::geometry::Rect,
    /// Where the layer's box is on the monitor.
    pub box_position: [f64; 2],
    /// The framebuffer the frost is read out of.
    pub source_size: [f64; 2],
    /// Deviation of the frost, device pixels.
    pub frost: f32,
    /// [`frost_reach`] of it.
    pub reach: i32,
}

/// The monitor rectangle a pane's frost would be read from, snapped outward to
/// the reduction's grid, so that every texel is a whole block of screen pixels
/// and the same pane reads the same blocks frame after frame, and clipped to
/// the source. Nothing, as a rectangle of no size, where it is empty.
#[must_use]
pub fn sample_rect(query: &SampleQuery) -> crate::geometry::Rect {
    use crate::geometry::Rect;
    use crate::num::{floor_i32, trunc_i32};

    let reduction = frost_reduction(query.frost);
    let reach = f64::from(query.reach.saturating_add(reduction));
    let step = f64::from(reduction);
    let (pane, at) = (&query.pane, query.box_position);
    let snap_down = |value: f64| floor_i32(value / step).saturating_mul(reduction);
    let snap_up = |value: f64| ceil_i32(value / step).saturating_mul(reduction);
    let limit = |size: f64| {
        trunc_i32(size)
            .checked_div(reduction)
            .unwrap_or(0)
            .saturating_mul(reduction)
    };
    let x1 = snap_down(at[0] + pane.x - reach).max(0);
    let y1 = snap_down(at[1] + pane.y - reach).max(0);
    let x2 = snap_up(at[0] + pane.x + pane.w + reach).min(limit(query.source_size[0]));
    let y2 = snap_up(at[1] + pane.y + pane.h + reach).min(limit(query.source_size[1]));
    if x2 <= x1 || y2 <= y1 {
        return Rect::default();
    }
    Rect::new(
        f64::from(x1),
        f64::from(y1),
        f64::from(x2.saturating_sub(x1)),
        f64::from(y2.saturating_sub(y1)),
    )
}

/// How far glass reaches beyond a pane, in device pixels, for the places that
/// have to make room for it or draw it again.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reaches {
    /// What the frost reads beyond a pane.
    pub frost: i32,
    /// What the shadow covers.
    pub shadow: i32,
    /// As far as the shadow falls and as far as the frost reads, on its coarsest grid.
    pub damage: i32,
    /// What a sample of the background must include beyond the layer.
    pub padding: i32,
    /// Around a part of the outline that moved: the shadow and the field's blur, and a margin.
    pub repaint: f64,
}

/// The reaches for a layer with this frost, shadow and bezel.
#[must_use]
pub fn reaches(frost: f32, shadow_reach: i32, bezel_width: f64) -> Reaches {
    let frost_reach = frost_reach(frost);
    Reaches {
        frost: frost_reach,
        shadow: shadow_reach,
        damage: shadow_reach.max(frost_reach.saturating_add(16)),
        padding: frost_reach.saturating_add(8).max(shadow_reach).max(1),
        repaint: f64::from(shadow_reach) + 3.0 * field_sigma(bezel_width) + 2.0,
    }
}

/// What the glass shader is given besides the textures and the geometry of a
/// quad: constants of the lens and the light, for one layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawConstants {
    /// Slab thickness that brings the bend to the centre line of a pane of fit 1.
    pub thickness_per_fit: f32,
    /// Blurred coverage above which the lens has ended.
    pub interior_coverage: f32,
    /// Blurred coverage above which a pixel is whole pane, well clear of the outline.
    pub solid_coverage: f32,
    /// Blurred coverage of a point a pixel and a half outside the outline.
    pub outer_coverage: f32,
    /// Blurred coverage below which a pixel can be on the rim line.
    pub rim_coverage: f32,
    /// The lens table's scale times the refraction multiplier.
    pub deviation_scale: f32,
    /// True when red and blue bend differently from green.
    pub dispersive: bool,
    /// Unit vector from the pane toward the light, y down.
    pub light_direction: [f32; 2],
}

/// The constants for one layer; `table_scale` is the lens table's `deviation_scale`.
#[must_use]
pub fn draw_constants(params: &ModelParams, table_scale: f64) -> DrawConstants {
    let lens = &params.lens;
    let n = lens.refractive_index;
    // fitted_to(): the bend on the outline, refraction * sqrt(n^2 - 1) * T (or, by the thin-prism
    // law, refraction * 8 (n - 1) * T), is at most half the short side, which is 2 B fit
    let reach_per_thickness = if lens.paraxial {
        lens.refraction * crate::lens::paraxial_reach(n).max(1e-9)
    } else {
        lens.refraction * (n * n - 1.0).max(1e-9).sqrt()
    };
    // v = Phi(d / sigma) at a distance d inside the outline. The rim line can only be on a
    // pixel within its own width of the outline and the two pixels its antialiasing takes;
    // the layer's antialiased edge reaches a pixel and a half outside the outline. A pixel
    // is safely whole pane at SOLID_COVERAGE only when that is further in than both.
    let sigma = field_sigma(lens.bezel_width);
    let coverage_at =
        |distance: f64| f64::midpoint(1.0, libm::erf(distance / sigma / 2.0_f64.sqrt()));
    let rim = f64::from(params.rim_width);
    let solid = f64::from(SOLID_COVERAGE)
        .max(coverage_at(rim + 3.0))
        .clamp(0.0, f64::from(INTERIOR_COVERAGE));
    let angle = params.light_angle * std::f32::consts::PI / 180.0;
    DrawConstants {
        thickness_per_fit: to_f32(2.0 * lens.bezel_width / reach_per_thickness),
        interior_coverage: INTERIOR_COVERAGE,
        solid_coverage: to_f32(solid),
        outer_coverage: to_f32(coverage_at(-1.5)),
        rim_coverage: to_f32(coverage_at(rim + 2.0)),
        deviation_scale: to_f32(table_scale * lens.refraction),
        dispersive: lens.dispersion > 0.0,
        // clockwise from the top, with y growing downward
        light_direction: [angle.sin(), -angle.cos()],
    }
}

/// The opacity of the glass, and of its shadow, in a frame where the layer is
/// drawn with this alpha: the glass's own opacity times the layer's fade.
#[must_use]
pub fn opacities(params: &ModelParams, alpha: f32) -> (f32, f32) {
    let glass = (params.glass_opacity * alpha).clamp(0.0, 1.0);
    (glass, params.shadow_opacity * glass)
}

#[cfg(test)]
mod tests;
