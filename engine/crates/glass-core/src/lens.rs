//! The lens model of the meniscus pipeline (docs/model.md): a slab of glass
//! with a rounded rim, lying on what it shows and seen from straight above.
//!
//! This is the one statement of the model. The glass shader holds no lens
//! arithmetic of its own: it reads the table built here.

use crate::num::{count_f64, floor_i32, index_of, unit_to_byte};

/// The largest lookup table that [`build_lens_table`] makes, in texels per row.
pub const MAX_TABLE_SIZE: usize = 4096;

/// The blur that turns coverage into distance reaches across the whole bezel
/// in this many deviations: three, where the blurred coverage is still 0.00135
/// short of 1 and 16 bits tell that apart.
pub const FIELD_SIGMAS_PER_BEZEL: f64 = 3.0;

/// Where the table's first row, the depth, is read: the centre of row 0 of 2.
pub const TABLE_ROW_DEPTH: f32 = 0.25;
/// Where the table's second row, the lens, is read: the centre of row 1 of 2.
pub const TABLE_ROW_LENS: f32 = 0.75;

/// Steps of the scan for the largest displacement. A fixed number, so the scan
/// is bounded.
const MAX_DISPLACEMENT_STEPS: u32 = 1024;

/// Why a lens table could not be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LensError {
    /// The table asked for is larger than [`MAX_TABLE_SIZE`] texels a row.
    #[error("a lens table of {0} texels a row is more than the {MAX_TABLE_SIZE} allowed")]
    TableTooLarge(usize),
}

/// The slab: lengths in pixels, whatever pixel the caller works in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lens {
    /// B: width of the rounded rim, which is also the height it rises by.
    pub bezel_width: f64,
    /// T: thickness of the slab under the rim.
    pub thickness: f64,
    /// n, for green light.
    pub refractive_index: f64,
    /// p of the rim's convex superellipse: 2 is a quarter circle, 4 a squircle.
    pub profile_exponent: f64,
    /// Multiplies the physical displacement; 1 leaves it as Snell gives it.
    pub refraction: f64,
    /// Reciprocal Abbe number: `(n_blue - n_red) / (n - 1)`.
    pub dispersion: f64,
    /// The law the rim bends by: Snell's on the profile above, or, when this
    /// is set, the thin prism measured on Apple's current material
    /// ([`paraxial_reach`]). The profile exponent is then not used, and the
    /// rim is as wide as the slab is thick.
    pub paraxial: bool,
}

impl Default for Lens {
    fn default() -> Self {
        Self {
            bezel_width: 32.0,
            thickness: 85.0,
            refractive_index: 1.5,
            profile_exponent: 2.6,
            refraction: 1.0,
            dispersion: 0.0,
            paraxial: false,
        }
    }
}

/// Height of the rim's surface above the slab, as a fraction of the bezel
/// width. `t` is the distance from the outline as a fraction of the bezel
/// width: 0 on the outline, 1 where the glass becomes flat. A convex
/// superellipse: `(1 - t)^p + h^p = 1`.
#[must_use]
pub fn profile_height(t: f64, p: f64) -> f64 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    (1.0 - u.powf(p)).max(0.0).powf(1.0 / p)
}

/// dh/dt of [`profile_height`], which is also the slope of the surface: the
/// rim is as high as it is wide. Unbounded at the outline, where the surface
/// meets the page at a right angle, and 0 where the glass becomes flat.
#[must_use]
pub fn profile_slope(t: f64, p: f64) -> f64 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    let base = (1.0 - u.powf(p)).max(1e-12);
    u.powf(p - 1.0) * base.powf(1.0 / p - 1.0)
}

/// Tangent of the angle by which a ray from straight above leans toward the
/// inside of the glass after entering the rim at band position `t`. It meets
/// a surface of slope `h'(t)` at the angle `a1 = atan(h'(t))` and continues at
/// `a2 = asin(sin(a1) / n)` from the normal (Snell), so it leans `a1 - a2`.
#[must_use]
pub fn deviation(t: f64, p: f64, n: f64) -> f64 {
    if t >= 1.0 || n <= 1.0 {
        return 0.0;
    }
    // on the outline the surface is vertical: a1 is a right angle and sin(a2) = 1 / n
    if t <= 0.0 {
        return (n * n - 1.0).sqrt();
    }
    let a1 = profile_slope(t, p).atan();
    let a2 = (a1.sin() / n).asin();
    (a1 - a2).tan()
}

/// The thin-prism law: how far the ray that enters on the outline lands
/// inside it, as a multiple of the slab's thickness. `D(t) = 8 T (n - 1) s^2`
/// with `s = 1 - t`, measured on the current material as thickness 10.7 points
/// and `n` 1.24 (`liquid_glass_render.frag:388-415` of `liquid_glass_widgets`,
/// see NOTICE): 20.5 points on the outline, and where its slope passes 1, from
/// the outline to 0.74 of the rim, the page shows mirrored and pressed
/// together.
#[must_use]
pub fn paraxial_reach(n: f64) -> f64 {
    8.0 * (n - 1.0).max(0.0)
}

/// The bend on the outline for a slab of thickness 1: what a pane's short
/// side limits ([`fitted_to`]).
fn reach_per_thickness(lens: &Lens, n: f64) -> f64 {
    if lens.paraxial {
        lens.refraction * paraxial_reach(n)
    } else {
        lens.refraction * (n * n - 1.0).max(1e-9).sqrt()
    }
}

/// How far, in pixels and toward the inside of the glass, the ray that
/// reaches the eye at band position `t` left the page from the point straight
/// below. By Snell's law it leans by [`deviation`] for the height that is left
/// to the page, the rim's own `B h(t)` plus the slab's `T`; by the thin-prism
/// law it is [`paraxial_reach`] times the thickness, falling as the square of
/// the way left to the flat of the glass.
#[must_use]
pub fn displacement(lens: &Lens, t: f64, n: f64) -> f64 {
    if t >= 1.0 {
        return 0.0;
    }
    if lens.paraxial {
        let s = 1.0 - t.clamp(0.0, 1.0);
        return lens.refraction * lens.thickness * paraxial_reach(n) * s * s;
    }
    let height = lens.bezel_width * profile_height(t, lens.profile_exponent) + lens.thickness;
    lens.refraction * height * deviation(t, lens.profile_exponent, n)
}

/// Refractive indices for red, green and blue: the dispersion is spread
/// evenly around green, so that `n_blue - n_red = dispersion * (n - 1)`.
#[must_use]
pub fn channel_indices(lens: &Lens) -> [f64; 3] {
    let half = 0.5 * lens.dispersion * (lens.refractive_index - 1.0);
    [
        (lens.refractive_index - half).max(1.0001),
        lens.refractive_index,
        lens.refractive_index + half,
    ]
}

/// The largest displacement of any channel anywhere in the band, in pixels.
#[must_use]
pub fn max_displacement(lens: &Lens) -> f64 {
    let blue = channel_indices(lens)[2];
    (0..MAX_DISPLACEMENT_STEPS).fold(0.0, |most, i| {
        let t = f64::from(i) / f64::from(MAX_DISPLACEMENT_STEPS);
        most.max(displacement(lens, t, blue))
    })
}

/// A pane too small for the whole lens gets a smaller one: the rim takes at
/// most a quarter of the pane's short side, and the ray that enters on the
/// outline lands no further in than the pane's centre line.
#[must_use]
pub fn fitted_to(lens: &Lens, short_side: f64) -> Lens {
    let mut fitted = *lens;
    if short_side <= 0.0 {
        return fitted;
    }
    fitted.bezel_width = lens.bezel_width.min(0.25 * short_side);
    let reach = reach_per_thickness(lens, lens.refractive_index);
    fitted.thickness = if reach > 0.0 {
        lens.thickness.min(0.5 * short_side / reach)
    } else {
        lens.thickness
    };
    fitted
}

/// Inverse of the standard normal distribution function (P. J. Acklam's
/// rational approximation, relative error below 1.2e-9).
#[must_use]
pub fn normal_quantile(probability: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 6] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
        1.0,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 5] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
        1.0,
    ];
    const LOW: f64 = 0.02425;

    let tail = |q: f64| horner(&C, q) / horner(&D, q);
    let p = probability.clamp(1e-12, 1.0 - 1e-12);
    if p < LOW {
        return tail((-2.0 * p.ln()).sqrt());
    }
    if p > 1.0 - LOW {
        return -tail((-2.0 * (1.0 - p).ln()).sqrt());
    }
    let q = p - 0.5;
    let r = q * q;
    horner(&A, r) * q / horner(&B, r)
}

/// The polynomial with these coefficients, highest power first.
fn horner(coefficients: &[f64], x: f64) -> f64 {
    coefficients.iter().fold(0.0, |acc, c| acc * x + c)
}

/// The outline of a pane is not known as geometry: it is whatever the layer
/// drew. The pipeline blurs the pane's coverage (1 inside, 0 outside) with a
/// Gaussian of deviation `sigma`, which along a straight edge gives
/// `v = Phi(d / sigma)` for the signed distance `d` from the edge. This
/// inverts it: the distance, in pixels, that a blurred coverage `v` stands
/// for (negative outside the pane).
#[must_use]
pub fn distance_from_blurred_coverage(v: f64, sigma: f64) -> f64 {
    sigma * normal_quantile(v)
}

/// Deviation of the blur that turns a bezel of this width into a distance.
#[must_use]
pub fn field_sigma(bezel_width: f64) -> f64 {
    (bezel_width / FIELD_SIGMAS_PER_BEZEL).max(1.0)
}

/// Coverage crowds toward 1 at the inner end of the bezel, so the table is
/// indexed by `s = sqrt(2 (1 - v))`, which is 1 on the outline and 0 deep
/// inside the pane and spreads that end out.
#[must_use]
pub fn lut_coordinate(v: f64) -> f64 {
    (2.0 * (1.0 - v)).clamp(0.0, 1.0).sqrt()
}

/// The coverage a table coordinate stands for: inverse of [`lut_coordinate`].
#[must_use]
pub fn coverage_at_lut_coordinate(s: f64) -> f64 {
    1.0 - 0.5 * s * s
}

/// What the glass shader looks up instead of computing, so that it holds no
/// lens arithmetic of its own. Two rows of `size` RGBA8 texels:
///
/// * row 0, indexed by [`lut_coordinate`] of the blurred coverage: R is the
///   distance from the outline in bezel widths, clamped to `[0, 1]`;
/// * row 1, indexed by band position `t`: RGB is [`deviation`] for red, green
///   and blue as fractions of `deviation_scale`, A is [`profile_height`].
///
/// A pane's displacement for channel `c` is then
/// `refraction * (B h + T) * deviation_scale * row1.c` with its own fitted
/// `B` and `T`, which is why those two stay out of the table.
///
/// The thin-prism law goes through the same table and the same line of the
/// shader: its row 1 holds `(1 - t)^2` for each channel as a fraction of the
/// scale, which is then [`paraxial_reach`] of blue, and a height of 0, so the
/// displacement comes out as `refraction * T * 8 (n - 1) (1 - t)^2`.
#[derive(Debug, Clone, PartialEq)]
pub struct LensTable {
    /// `2 * size * 4` bytes, row 0 first.
    pub texels: Vec<u8>,
    /// Texels in a row.
    pub size: usize,
    /// The largest lean of any channel, which the bytes of row 1 are fractions of.
    pub deviation_scale: f64,
}

/// Builds the table for `lens` with rows of `size` texels (at least 2).
///
/// # Errors
///
/// [`LensError::TableTooLarge`] when `size` is more than [`MAX_TABLE_SIZE`].
pub fn build_lens_table(lens: &Lens, size: usize) -> Result<LensTable, LensError> {
    if size > MAX_TABLE_SIZE {
        return Err(LensError::TableTooLarge(size));
    }
    let size = size.max(2);
    let indices = channel_indices(lens);
    let deviation_scale = if lens.paraxial {
        paraxial_reach(indices[2]).max(1e-9)
    } else {
        (indices[2] * indices[2] - 1.0).sqrt()
    };
    let last = count_f64(size.saturating_sub(1));
    let row_depth = (0..size).flat_map(|i| {
        // v = Phi(d / sigma) and sigma = B / FIELD_SIGMAS_PER_BEZEL, so d / B is the quantile over that constant
        let x = (count_f64(i) + 0.5) / count_f64(size);
        let depth = normal_quantile(coverage_at_lut_coordinate(x)) / FIELD_SIGMAS_PER_BEZEL;
        [unit_to_byte(depth), 0, 0, 255]
    });
    // the first and last texels stand for the two ends exactly, so the filtered read meets them
    let row_lens = (0..size).flat_map(|i| {
        let t = count_f64(i) / last;
        let lean = |channel: usize| {
            let n = indices
                .get(channel)
                .copied()
                .unwrap_or(lens.refractive_index);
            let bend = if lens.paraxial {
                paraxial_reach(n) * (1.0 - t) * (1.0 - t)
            } else {
                deviation(t, lens.profile_exponent, n)
            };
            unit_to_byte(bend / deviation_scale)
        };
        let height = if lens.paraxial {
            0.0
        } else {
            profile_height(t, lens.profile_exponent)
        };
        [lean(0), lean(1), lean(2), unit_to_byte(height)]
    });
    Ok(LensTable {
        texels: row_depth.chain(row_lens).collect(),
        size,
        deviation_scale,
    })
}

impl LensTable {
    /// A texel of the table, as a fraction of 1; `row` 0 or 1, `channel` 0 to 3.
    fn texel(&self, row: usize, index: usize, channel: usize) -> f64 {
        let at = row
            .saturating_mul(self.size)
            .saturating_add(index)
            .saturating_mul(4)
            .saturating_add(channel);
        self.texels.get(at).map_or(0.0, |b| f64::from(*b) / 255.0)
    }

    /// A linearly filtered read of one row at `x` in `[0, 1]` of the row's
    /// width, texel centres at `(i + 0.5) / size`: what the GPU does.
    fn filtered(&self, row: usize, x: f64, channel: usize) -> f64 {
        let size = count_f64(self.size);
        let position = (x * size - 0.5).clamp(0.0, (size - 1.0).max(0.0));
        let low = position.floor();
        let weight = position - low;
        let first = index_of(floor_i32(low));
        let second = first.saturating_add(1).min(self.size.saturating_sub(1));
        self.texel(row, first, channel) * (1.0 - weight) + self.texel(row, second, channel) * weight
    }

    /// Distance from the outline, in bezel widths, for a blurred coverage `v`:
    /// what the shader reads from row 0.
    #[must_use]
    pub fn depth_at(&self, v: f64) -> f64 {
        self.filtered(0, lut_coordinate(v), 0)
    }

    /// Lean for `channel` (0 red, 1 green, 2 blue) at band position `t`, in
    /// the physical units of [`deviation`]: what the shader reads from row 1
    /// times the scale.
    #[must_use]
    pub fn lean_at(&self, t: f64, channel: usize) -> f64 {
        let x = (t.clamp(0.0, 1.0) * count_f64(self.size.saturating_sub(1)) + 0.5)
            / count_f64(self.size);
        self.filtered(1, x, channel) * self.deviation_scale
    }

    /// Height of the rim at band position `t`, as a fraction of the bezel width.
    #[must_use]
    pub fn height_at(&self, t: f64) -> f64 {
        let x = (t.clamp(0.0, 1.0) * count_f64(self.size.saturating_sub(1)) + 0.5)
            / count_f64(self.size);
        self.filtered(1, x, 3)
    }
}

#[cfg(test)]
mod tests;
