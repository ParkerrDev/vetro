//! Wobbly windows: a window that is moved is a sheet of jelly. Its corners and
//! edges keep where they were for a moment and are pulled after it by springs,
//! so a dragged window trails behind the pointer and jiggles to rest when it
//! stops. A window that opens can be squeezed and let go, and it bounces into
//! shape.
//!
//! The sheet is a grid of [`NODES`] by [`NODES`] points. Each is held to its
//! place in the window by a spring and to its neighbours by others, so a pull
//! on one spreads as a wave. One point can be held: the one under the pointer,
//! which moves with the window. The caller keeps the state; this steps it and
//! works out a finer mesh over it with a smooth curve through the points.

use super::SpringSpec;
use crate::geometry::Rect;
use crate::num::{count_f64, floor_i32, index_of, to_f32};

/// Points a side of the sheet.
pub const NODES: usize = 5;
/// Points of the sheet.
pub const COUNT: usize = NODES * NODES;
/// The point in the middle of the sheet, held when a window is moved for it.
pub const CENTRE: usize = COUNT / 2;

/// The step the sheet is moved forward by, seconds.
const SUBSTEP: f64 = 1.0 / 240.0;
/// The most steps one call makes: a fifth of a second. A longer wait (the
/// screen asleep, a stall) is dropped rather than caught up on.
const MAX_SUBSTEPS: u32 = 48;
/// How much further in a squeeze pushes the middle of each side than its
/// corners: the sides bow in, so the sheet wobbles as it springs out rather
/// than only growing.
const BOW: f64 = 0.6;
/// The quickest swing a sheet may have: quicker would not be stable at [`SUBSTEP`].
const MIN_DURATION: f64 = 0.12;

/// How the jelly swings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WobbleSpec {
    /// The spring of each point to its place: how long a swing takes and how
    /// much it bounces.
    pub spring: SpringSpec,
    /// How strongly a point is held to its neighbours, as a share of the
    /// spring to its place. More is a stiffer sheet that moves as one.
    pub coupling: f64,
    /// How far a point may trail, as a share of the window's longer side.
    pub reach: f64,
}

impl WobbleSpec {
    /// A soft jelly that settles in about a second.
    pub const JELLY: Self = Self {
        spring: SpringSpec {
            duration: 0.55,
            bounce: 0.55,
        },
        coupling: 2.0,
        reach: 0.3,
    };

    /// The spec held to what it can be: a swing no quicker than 0.12
    /// seconds, damped at least a little, coupling 0 to 8, reach up
    /// to the whole window.
    #[must_use]
    pub fn new(duration: f64, bounce: f64, coupling: f64, reach: f64) -> Self {
        let spring = SpringSpec::new(duration.max(MIN_DURATION), bounce.min(0.95));
        let finite =
            |v: f64, lo: f64, hi: f64, or: f64| if v.is_finite() { v.clamp(lo, hi) } else { or };
        Self {
            spring,
            coupling: finite(coupling, 0.0, 8.0, Self::JELLY.coupling),
            reach: finite(reach, 0.0, 1.0, Self::JELLY.reach),
        }
    }
}

/// A sheet of jelly: each point's offset from its place in the window and its
/// speed, in pixels and pixels a second, and the point held, if one is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wobble {
    pub offset: [[f64; 2]; COUNT],
    pub velocity: [[f64; 2]; COUNT],
    pub held: Option<usize>,
}

impl Default for Wobble {
    fn default() -> Self {
        Self::still()
    }
}

impl Wobble {
    /// A sheet at rest, nothing held.
    #[must_use]
    pub const fn still() -> Self {
        Self {
            offset: [[0.0; 2]; COUNT],
            velocity: [[0.0; 2]; COUNT],
            held: None,
        }
    }

    /// Holds the point nearest (`u`, `v`), each 0 to 1 across the window, or
    /// lets go with a point outside it.
    pub fn hold_at(&mut self, u: f64, v: f64) {
        let inside = (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v);
        self.held = inside.then(|| {
            let last = count_f64(NODES.saturating_sub(1));
            let near = |t: f64| index_of(floor_i32(t * last + 0.5)).min(NODES.saturating_sub(1));
            near(v).saturating_mul(NODES).saturating_add(near(u))
        });
    }

    /// The window moved, or grew, from `from` to `to`: every point but the
    /// held one stays where it was on the screen, and so is that much
    /// further from its new place, never further than the spec's reach.
    pub fn moved(&mut self, from: &Rect, to: &Rect, spec: &WobbleSpec) {
        let finite = |r: &Rect| [r.x, r.y, r.w, r.h].iter().all(|v| v.is_finite());
        if !(finite(from) && finite(to)) {
            return;
        }
        let limit = spec.reach * to.w.max(to.h).max(0.0);
        let last = count_f64(NODES.saturating_sub(1));
        for (row, offsets) in self.offset.chunks_exact_mut(NODES).enumerate() {
            let v = count_f64(row) / last;
            for (col, offset) in offsets.iter_mut().enumerate() {
                if self.held == row.checked_mul(NODES).and_then(|r| r.checked_add(col)) {
                    continue;
                }
                let u = count_f64(col) / last;
                let dx = (to.x + to.w * u) - (from.x + from.w * u);
                let dy = (to.y + to.h * v) - (from.y + from.h * v);
                let (x, y) = (offset[0] - dx, offset[1] - dy);
                let length = x.hypot(y);
                let keep = if length > limit && length > 0.0 {
                    limit / length
                } else {
                    1.0
                };
                *offset = [x * keep, y * keep];
            }
        }
    }

    /// Squeezes a window of `width` by `height` towards its middle by
    /// `amount` (a share of its size), the middle of each side 0.6 of that further, to
    /// let go of: it springs out into shape, overshooting as far as the spec
    /// bounces.
    pub fn squeeze(&mut self, (width, height): (f64, f64), amount: f64) {
        let amount = if amount.is_finite() {
            amount.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let last = count_f64(NODES.saturating_sub(1));
        // 1 at the corners, 1 + BOW halfway along a side
        let bow = |t: f64| 1.0 + BOW * (1.0 - 4.0 * t * t);
        for (row, offsets) in self.offset.chunks_exact_mut(NODES).enumerate() {
            for (col, offset) in offsets.iter_mut().enumerate() {
                let u = count_f64(col) / last - 0.5;
                let v = count_f64(row) / last - 0.5;
                *offset = [-u * width * amount * bow(v), -v * height * amount * bow(u)];
            }
        }
        self.velocity = [[0.0; 2]; COUNT];
    }

    /// True when no point is further than `tolerance` pixels from its place
    /// or moving faster than ten times that a second.
    #[must_use]
    pub fn settled(&self, tolerance: f64) -> bool {
        let small = |v: &[f64; 2], within: f64| v[0].abs() <= within && v[1].abs() <= within;
        self.offset.iter().all(|o| small(o, tolerance))
            && self.velocity.iter().all(|v| small(v, tolerance * 10.0))
    }

    /// Moves the sheet forward `dt` seconds in steps of 1/240 of a second,
    /// at most 48 of them.
    pub fn step(&mut self, dt: f64, spec: &WobbleSpec) {
        if !(dt.is_finite() && dt > 0.0) {
            return;
        }
        let mut left = dt.min(SUBSTEP * f64::from(MAX_SUBSTEPS));
        for _ in 0..MAX_SUBSTEPS {
            if left <= 0.0 {
                break;
            }
            let h = left.min(SUBSTEP);
            self.substep(h, spec);
            left -= h;
        }
    }

    /// One step: each point is pulled to its place and towards its
    /// neighbours, slowed by the damping, and moved (semi-implicit Euler,
    /// which keeps the energy of a spring from growing).
    fn substep(&mut self, dt: f64, spec: &WobbleSpec) {
        let (stiffness, damping) = (spec.spring.stiffness(), spec.spring.damping());
        let pull = stiffness * spec.coupling;
        let before = self.offset;
        for (point, (offset, velocity)) in self
            .offset
            .iter_mut()
            .zip(self.velocity.iter_mut())
            .enumerate()
        {
            if self.held == Some(point) {
                *offset = [0.0; 2];
                *velocity = [0.0; 2];
                continue;
            }
            let (col, row) = (point % NODES, point / NODES);
            let spread = neighbours(col, row)
                .into_iter()
                .flatten()
                .filter_map(|n| before.get(n))
                .fold([0.0; 2], |sum, o| {
                    [sum[0] + o[0] - offset[0], sum[1] + o[1] - offset[1]]
                });
            for axis in 0..2 {
                let (Some(o), Some(v), Some(s)) = (
                    offset.get_mut(axis),
                    velocity.get_mut(axis),
                    spread.get(axis),
                ) else {
                    continue;
                };
                *v += (-stiffness * *o - damping * *v + pull * s) * dt;
                *o += *v * dt;
            }
        }
    }

    /// The offset of the sheet at (`u`, `v`), each 0 to 1 across the window:
    /// a Catmull-Rom curve through the points, so the mesh is smooth and
    /// passes through every point, the held one included.
    #[must_use]
    pub fn offset_at(&self, u: f64, v: f64) -> [f64; 2] {
        let (col, fx) = cell(u);
        let (row, fy) = cell(v);
        let (wx, wy) = (catmull_rom(fx), catmull_rom(fy));
        let mut sum = [0.0; 2];
        for (j, weight_y) in wy.iter().enumerate() {
            for (i, weight_x) in wx.iter().enumerate() {
                let point = self.node(near_index(col, i), near_index(row, j));
                let w = weight_x * weight_y;
                sum = [sum[0] + point[0] * w, sum[1] + point[1] * w];
            }
        }
        sum
    }

    fn node(&self, col: usize, row: usize) -> [f64; 2] {
        row.checked_mul(NODES)
            .and_then(|r| r.checked_add(col))
            .and_then(|k| self.offset.get(k))
            .copied()
            .unwrap_or_default()
    }
}

/// The points beside the one at `col`, `row`: left, right, above, below.
fn neighbours(col: usize, row: usize) -> [Option<usize>; 4] {
    let at = |c: usize, r: usize| {
        (c < NODES && r < NODES).then(|| r.saturating_mul(NODES).saturating_add(c))
    };
    [
        col.checked_sub(1).and_then(|c| at(c, row)),
        col.checked_add(1).and_then(|c| at(c, row)),
        row.checked_sub(1).and_then(|r| at(col, r)),
        row.checked_add(1).and_then(|r| at(col, r)),
    ]
}

/// The cell of the sheet a share `t` across it falls in, and how far into it.
fn cell(t: f64) -> (usize, f64) {
    let cells = NODES.saturating_sub(1);
    let at = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    } * count_f64(cells);
    let index = index_of(floor_i32(at)).min(cells.saturating_sub(1));
    (index, at - count_f64(index))
}

/// The point `i` (0 to 3) of the four a cell's curve passes near: the one
/// before the cell, its two ends, the one after, the edge repeated past the
/// sheet's ends.
fn near_index(cell: usize, i: usize) -> usize {
    cell.saturating_add(i)
        .saturating_sub(1)
        .min(NODES.saturating_sub(1))
}

/// The weights of a Catmull-Rom segment at `t`, 0 to 1 between its two middle points.
fn catmull_rom(t: f64) -> [f64; 4] {
    let (t2, t3) = (t * t, t * t * t);
    [
        0.5_f64.mul_add(-t3, t2) - 0.5 * t,
        1.5_f64.mul_add(t3, (-2.5_f64).mul_add(t2, 1.0)),
        (-1.5_f64).mul_add(t3, 2.0_f64.mul_add(t2, 0.5 * t)),
        0.5 * t3 - 0.5 * t2,
    ]
}

/// The points of a window drawn on the sheet, `cols` by `rows` cells, into
/// `out` as x, y pairs row by row, as [`super::genie::genie_mesh`] lays them
/// out. Returns the number of points.
///
/// # Errors
///
/// [`super::genie::MeshError`] when the cells are out of range or `out` is too short.
pub fn wobble_mesh(
    wobble: &Wobble,
    window: &Rect,
    (cols, rows): (usize, usize),
    out: &mut [f32],
) -> Result<usize, super::genie::MeshError> {
    let needed = super::genie::mesh_len(cols, rows)?;
    let room = out.len();
    let out = out
        .get_mut(..needed)
        .ok_or(super::genie::MeshError::Room(room, needed))?;
    let row_len = cols.saturating_add(1).saturating_mul(2);
    for (j, row) in out.chunks_exact_mut(row_len).enumerate() {
        let v = count_f64(j) / count_f64(rows);
        for (i, pair) in row.chunks_exact_mut(2).enumerate() {
            let u = count_f64(i) / count_f64(cols);
            let [dx, dy] = wobble.offset_at(u, v);
            if let [px, py] = pair {
                *px = to_f32(window.x + window.w * u + dx);
                *py = to_f32(window.y + window.h * v + dy);
            }
        }
    }
    Ok(needed.div_euclid(2))
}

#[cfg(test)]
mod tests;
