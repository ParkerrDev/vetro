//! The genie: a window poured into a narrow mouth, as into a bottle, or out of
//! one. Its sides first bend into a funnel that narrows towards the mouth, then
//! the whole window slides down the funnel and through the mouth, where it is
//! gone. Opening is the same run backwards.
//!
//! The window is a grid of points, and this works out where each one is at a
//! progress from 0 (the window as it is) to 1 (all of it through the mouth).
//! The caller draws the window's picture over that grid. Everything is worked
//! out along the way the window flows and across it, so one funnel serves a
//! mouth below, above, left or right of the window.

use super::smoothstep;
use crate::geometry::Rect;
use crate::num::{count_f64, to_f32};

/// The most cells a mesh has across or along.
pub const MAX_CELLS: usize = 64;

/// Below this the funnel is no length at all.
const EPSILON: f64 = 1e-6;

/// Where the window goes: a point on the screen, and how wide the mouth is
/// across the way the window flows into it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mouth {
    pub x: f64,
    pub y: f64,
    pub width: f64,
}

/// How a progress is shared between bending and sliding. They overlap, so the
/// window is still bending as it starts to slide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Phases {
    /// Progress by which the sides have bent all the way.
    pub bent_by: f64,
    /// Progress at which the window starts to slide.
    pub slide_from: f64,
}

impl Phases {
    /// The shares that look like macOS's genie: half the time to bend, sliding
    /// from a fifth of the way.
    pub const GENIE: Self = Self {
        bent_by: 0.5,
        slide_from: 0.2,
    };

    /// How far bent and how far slid at `progress`, each 0 to 1.
    #[must_use]
    pub fn at(&self, progress: f64) -> (f64, f64) {
        let p = if progress.is_finite() {
            progress.clamp(0.0, 1.0)
        } else {
            0.0
        };
        (
            smoothstep(0.0, self.bent_by, p),
            smoothstep(self.slide_from, 1.0, p),
        )
    }
}

/// Why a mesh could not be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MeshError {
    /// No cells, or more than [`MAX_CELLS`], across or along.
    #[error("a mesh of {0} by {1} cells is not between 1 and {MAX_CELLS} each way")]
    Cells(usize, usize),
    /// The buffer for the points is shorter than two numbers a point.
    #[error("{0} numbers are not room for a mesh that needs {1}")]
    Room(usize, usize),
}

/// The numbers a mesh of `cols` by `rows` cells needs: an x and a y for each of
/// its points, row by row.
///
/// # Errors
///
/// [`MeshError::Cells`] when either count is 0 or more than [`MAX_CELLS`].
pub fn mesh_len(cols: usize, rows: usize) -> Result<usize, MeshError> {
    if !(1..=MAX_CELLS).contains(&cols) || !(1..=MAX_CELLS).contains(&rows) {
        return Err(MeshError::Cells(cols, rows));
    }
    let points = cols
        .saturating_add(1)
        .saturating_mul(rows.saturating_add(1));
    Ok(points.saturating_mul(2))
}

/// Which way the window flows to its mouth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Down,
    Up,
    Right,
    Left,
}

impl Flow {
    /// The way to a mouth: along the axis on which it lies further outside the
    /// window, up or down when it is inside.
    fn towards(window: &Rect, mouth: &Mouth) -> Self {
        let outside = |at: f64, from: f64, size: f64| (from - at).max(at - (from + size)).max(0.0);
        let across = outside(mouth.x, window.x, window.w);
        let along = outside(mouth.y, window.y, window.h);
        if along >= across {
            if mouth.y >= window.y + window.h / 2.0 {
                Self::Down
            } else {
                Self::Up
            }
        } else if mouth.x >= window.x + window.w / 2.0 {
            Self::Right
        } else {
            Self::Left
        }
    }

    /// A screen point as (along the flow, across it).
    fn into_flow(self, x: f64, y: f64) -> (f64, f64) {
        match self {
            Self::Down => (y, x),
            Self::Up => (-y, x),
            Self::Right => (x, y),
            Self::Left => (-x, y),
        }
    }

    /// The screen point at (along the flow, across it).
    fn out_of_flow(self, along: f64, across: f64) -> (f64, f64) {
        match self {
            Self::Down => (across, along),
            Self::Up => (across, -along),
            Self::Right => (along, across),
            Self::Left => (-along, across),
        }
    }
}

/// The window and the mouth along the flow: the window from `start` (the edge
/// away from the mouth) to the mouth at `mouth`, across from `left` to
/// `right`, and the mouth across from `mouth_left` to `mouth_right`.
#[derive(Debug, Clone, Copy)]
struct Funnel {
    start: f64,
    mouth: f64,
    left: f64,
    right: f64,
    mouth_left: f64,
    mouth_right: f64,
}

impl Funnel {
    fn new(flow: Flow, window: &Rect, mouth: &Mouth) -> Self {
        let (a0, c0) = flow.into_flow(window.x, window.y);
        let (a1, c1) = flow.into_flow(window.x + window.w, window.y + window.h);
        let (m, centre) = flow.into_flow(mouth.x, mouth.y);
        let half = mouth.width.max(0.0) / 2.0;
        Self {
            start: a0.min(a1),
            // a mouth inside the window is taken to be on its near edge
            mouth: m.max(a0.max(a1)),
            left: c0.min(c1),
            right: c0.max(c1),
            mouth_left: centre - half,
            mouth_right: centre + half,
        }
    }

    /// Where a point of the window at (`along`, `across`) is, bent by `bend`
    /// and slid by `slide`.
    fn place(&self, along: f64, across: f64, bend: f64, slide: f64) -> (f64, f64) {
        let length = self.mouth - self.start;
        let moved = (along + slide * length).min(self.mouth);
        let t = if length > EPSILON {
            (moved - self.start) / length
        } else {
            1.0
        };
        let narrowing = smoothstep(0.0, 1.0, t);
        let lerp = |a: f64, b: f64, k: f64| a + (b - a) * k;
        let left = lerp(self.left, lerp(self.left, self.mouth_left, narrowing), bend);
        let right = lerp(
            self.right,
            lerp(self.right, self.mouth_right, narrowing),
            bend,
        );
        let width = self.right - self.left;
        let share = if width > EPSILON {
            (across - self.left) / width
        } else {
            0.5
        };
        (moved, lerp(left, right, share))
    }
}

/// The points of a window of `cols` by `rows` cells poured into `mouth` at
/// `progress` (0 the window as it is, 1 gone through), into `out` as x, y
/// pairs, row by row from the window's top left. The point in column `i` and
/// row `j` shows the window's picture at (`i / cols`, `j / rows`). Returns the
/// number of points.
///
/// # Errors
///
/// [`MeshError`] when the cells are out of range or `out` is too short.
pub fn genie_mesh(
    window: &Rect,
    mouth: &Mouth,
    progress: f64,
    (cols, rows): (usize, usize),
    out: &mut [f32],
) -> Result<usize, MeshError> {
    let needed = mesh_len(cols, rows)?;
    let room = out.len();
    let out = out.get_mut(..needed).ok_or(MeshError::Room(room, needed))?;
    let flow = Flow::towards(window, mouth);
    let funnel = Funnel::new(flow, window, mouth);
    let (bend, slide) = Phases::GENIE.at(progress);
    let row_len = cols.saturating_add(1).saturating_mul(2);
    for (j, row) in out.chunks_exact_mut(row_len).enumerate() {
        let y = window.y + window.h * count_f64(j) / count_f64(rows);
        for (i, pair) in row.chunks_exact_mut(2).enumerate() {
            let x = window.x + window.w * count_f64(i) / count_f64(cols);
            let (along, across) = flow.into_flow(x, y);
            let (along, across) = funnel.place(along, across, bend, slide);
            let (x, y) = flow.out_of_flow(along, across);
            if let [px, py] = pair {
                (*px, *py) = (to_f32(x), to_f32(y));
            }
        }
    }
    Ok(needed.div_euclid(2))
}

#[cfg(test)]
mod tests;
