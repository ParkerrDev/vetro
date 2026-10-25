//! Panes: the pieces of glass a layer is made of, found from the coarse map
//! the GPU draws of its outline.
//!
//! The map has one RGBA cell per 8 by 8 device pixels of the field. Red is the
//! largest blurred coverage in the cell and one texel around it: where a pane
//! or its surroundings can be. Green is the smallest in the cell itself: 255
//! only where every pixel is deep inside a pane, clear of its rim. Connected
//! cells are one pane; this finds each pane's box, its short side (which the
//! lens is fitted to) and the largest rectangle of it that is face and nothing
//! else.

use crate::geometry::Rect;
use crate::lens::normal_quantile;
use crate::num::{count_f64, unit_to_byte};

/// Side of one cell of the map, in device pixels.
pub const CELL_PIXELS: usize = 8;
/// The most cells a map may have: a 16k by 16k screen in cells of 8 pixels is under this.
pub const MAX_CELLS: usize = 1 << 22;
/// The most panes a layer is drawn as; more become one.
pub const MAX_PANES: usize = 16;
/// Blurred coverage from which a cell belongs to a pane's surroundings: `floor(0.08 * 255)`.
pub const CELL_IS_PANE: u8 = 20;
/// Blurred coverage on the outline itself: `floor(0.5 * 255)`.
pub const CELL_IS_INSIDE: u8 = 127;
/// Smallest coverage of a cell that is deep inside a pane, as the map stores it.
pub const CELL_IS_FACE: u8 = 255;
/// A redrawn area larger than this share of the layer says nothing about where
/// a new pane might be (Qt declares the whole surface redrawn every time): the
/// panes known so far stand until the map is read again, a frame or two later.
pub const UNKNOWN_AREA_LIMIT: f64 = 0.25;
/// A strip whose middle never reaches this coverage is told by it, not by its cells.
const THIN_STRIP_PEAK: f64 = 0.98;

/// Why a map could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PaneError {
    /// The cells are not `width * height` RGBA texels.
    #[error("{0} bytes of cells are not {1} by {2} RGBA texels")]
    CellsMismatch(usize, usize, usize),
    /// The map has more cells than [`MAX_CELLS`].
    #[error("a map of {0} by {1} cells is too large")]
    TooManyCells(usize, usize),
}

/// A piece of glass.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pane {
    /// Its extent, device pixels relative to the layer's box.
    pub rect: Rect,
    /// Of the pane itself, device pixels: what the lens is fitted to.
    pub short_side: f64,
    /// Rectangles inside it where every pixel is face only, relative to the layer's box.
    pub interiors: Vec<Rect>,
}

impl Pane {
    /// A pane of this extent, with no face rectangle.
    #[must_use]
    pub fn of(rect: Rect, short_side: f64) -> Self {
        Self {
            rect,
            short_side,
            interiors: Vec::new(),
        }
    }
}

/// The map as the GPU drew it.
#[derive(Debug, Clone, Copy)]
pub struct CellGrid<'a> {
    /// `width * height` RGBA texels, row by row.
    pub cells: &'a [u8],
    pub width: usize,
    pub height: usize,
}

/// What the map's cells stand for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapGeometry {
    /// Device pixels of field around the layer's box.
    pub margin: i32,
    /// Device pixels one texel of the field stands for.
    pub field_scale: i32,
    /// Deviation of the field's blur, device pixels.
    pub sigma: f64,
    /// Width of the whole lens's rim.
    pub bezel_width: f64,
    /// The layer's box: panes are cut to it.
    pub box_size: (f64, f64),
}

/// Every pane of a map, and for every cell how much of the whole lens the pane
/// there has room for (255 for a cell that belongs to no pane: nothing reads it).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PaneMap {
    pub panes: Vec<Pane>,
    pub fit: Vec<u8>,
}

/// How much of the whole lens a pane of this short side has room for, as the
/// byte the glass shader reads: its short side over four bezel widths, at most 1.
#[must_use]
pub fn fit_byte(short_side: f64, bezel_width: f64) -> u8 {
    let room = if bezel_width > 0.0 {
        short_side / (4.0 * bezel_width)
    } else {
        1.0
    };
    unit_to_byte(room)
}

/// A connected patch of cells.
struct Patch {
    members: Vec<usize>,
    /// First and last column and row of the patch.
    bounds: [usize; 4],
    /// The same for its cells that are inside the outline; none when there are none.
    inside: Option<[usize; 4]>,
    most: u8,
}

/// The box around `bounds` and the cell at (`x`, `y`).
fn grow(bounds: Option<[usize; 4]>, x: usize, y: usize) -> [usize; 4] {
    bounds.map_or([x, y, x, y], |b| {
        [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]
    })
}

fn index(x: usize, y: usize, width: usize) -> usize {
    y.saturating_mul(width).saturating_add(x)
}

/// Column and row of the cell at `at` in a map `width` cells across.
fn column_row(at: usize, width: usize) -> (usize, usize) {
    (
        at.checked_rem(width).unwrap_or(0),
        at.checked_div(width).unwrap_or(0),
    )
}

struct Scan<'a> {
    grid: &'a CellGrid<'a>,
    seen: Vec<bool>,
    /// Which pane a cell belongs to, counted from 1; 0 for none.
    owner: Vec<usize>,
}

impl Scan<'_> {
    fn red(&self, at: usize) -> u8 {
        self.grid
            .cells
            .get(at.saturating_mul(4))
            .copied()
            .unwrap_or(0)
    }

    fn is_face(&self, at: usize) -> bool {
        self.grid
            .cells
            .get(at.saturating_mul(4).saturating_add(1))
            .is_some_and(|green| *green == CELL_IS_FACE)
    }

    fn mark(&mut self, at: usize) {
        if let Some(seen) = self.seen.get_mut(at) {
            *seen = true;
        }
    }

    /// The patch of connected cells (eight neighbours) that holds `start`.
    fn flood(&mut self, start: usize) -> Patch {
        let width = self.grid.width;
        let (x, y) = column_row(start, width);
        let mut patch = Patch {
            members: Vec::new(),
            bounds: [x, y, x, y],
            inside: None,
            most: 0,
        };
        let mut stack = vec![start];
        self.mark(start);
        // every cell is pushed once: the loop ends in at most as many steps as there are cells
        for _ in 0..self.seen.len() {
            let Some(at) = stack.pop() else { break };
            let (cx, cy) = column_row(at, width);
            let value = self.red(at);
            patch.members.push(at);
            patch.bounds = grow(Some(patch.bounds), cx, cy);
            patch.most = patch.most.max(value);
            if value >= CELL_IS_INSIDE {
                patch.inside = Some(grow(patch.inside, cx, cy));
            }
            self.push_neighbours(&mut stack, cx, cy);
        }
        patch
    }

    fn push_neighbours(&mut self, stack: &mut Vec<usize>, cx: usize, cy: usize) {
        let (width, height) = (self.grid.width, self.grid.height);
        let rows = cy.saturating_sub(1)..=cy.saturating_add(1).min(height.saturating_sub(1));
        for ny in rows {
            let columns = cx.saturating_sub(1)..=cx.saturating_add(1).min(width.saturating_sub(1));
            for nx in columns {
                let at = index(nx, ny, width);
                if self.seen.get(at).copied().unwrap_or(true) || self.red(at) < CELL_IS_PANE {
                    continue;
                }
                self.mark(at);
                stack.push(at);
            }
        }
    }

    /// The largest rectangle of cells, among those of pane `id`, that are all
    /// face: row by row, the run of face cells above each cell is a histogram,
    /// and the largest rectangle under a histogram is found with a stack of
    /// the bars still open. First column, first row, last column, last row.
    fn largest_face_rectangle(&self, bounds: [usize; 4], id: usize) -> Option<[usize; 4]> {
        let [x1, y1, x2, y2] = bounds;
        let columns = x2.saturating_sub(x1).saturating_add(1);
        // the extra column of height 0 closes every bar at the end of the row
        let mut heights = vec![0_usize; columns.saturating_add(1)];
        let (mut best_area, mut best) = (0_usize, [0_usize; 4]);
        for cy in y1..=y2 {
            for (offset, height) in heights.iter_mut().take(columns).enumerate() {
                let at = index(x1.saturating_add(offset), cy, self.grid.width);
                let face = self.owner.get(at) == Some(&id) && self.is_face(at);
                *height = if face { height.saturating_add(1) } else { 0 };
            }
            let mut open: Vec<usize> = Vec::new();
            for column in 0..=columns {
                let current = heights.get(column).copied().unwrap_or(0);
                // every pass of this loop pops a bar: at most as many as are open
                for _ in 0..=open.len() {
                    let Some(&top) = open.last() else { break };
                    let bar = heights.get(top).copied().unwrap_or(0);
                    if bar < current {
                        break;
                    }
                    open.pop();
                    let left = open.last().map_or(0, |l| l.saturating_add(1));
                    let area = bar.saturating_mul(column.saturating_sub(left));
                    if area > best_area {
                        best_area = area;
                        best = [
                            x1.saturating_add(left),
                            cy.saturating_add(1).saturating_sub(bar),
                            x1.saturating_add(column).saturating_sub(1),
                            cy,
                        ];
                    }
                }
                open.push(column);
            }
        }
        (best_area > 0).then_some(best)
    }

    /// The pane a patch makes, and the byte its cells' fit is stored as, or
    /// none when the patch is only the surroundings of one or lies outside the box.
    fn pane_of(&self, patch: &Patch, geometry: &MapGeometry, id: usize) -> Option<(Pane, u8)> {
        let cell = count_f64(CELL_PIXELS);
        let margin = f64::from(geometry.margin);
        let [x1, y1, x2, y2] = patch.bounds;
        let [ix1, iy1, ix2, iy2] = patch.inside?;
        let pixels =
            |from: usize, to: usize| count_f64(to.saturating_sub(from).saturating_add(1)) * cell;
        let patch_rect = Rect::new(
            count_f64(x1) * cell - margin,
            count_f64(y1) * cell - margin,
            pixels(x1, x2),
            pixels(y1, y2),
        );
        let whole = Rect::new(0.0, 0.0, geometry.box_size.0, geometry.box_size.1);
        let clipped = patch_rect.intersection(&whole);
        if !clipped.has_area() {
            return None;
        }
        // A cell holds the largest value in it and one texel around it, so the
        // inside cells overshoot the outline by about a cell on each side.
        let across = ix2
            .saturating_sub(ix1)
            .min(iy2.saturating_sub(iy1))
            .saturating_add(1);
        let mut short_side =
            (count_f64(across) * cell - cell - 2.0 * f64::from(geometry.field_scale)).max(1.0);
        // A strip thinner than the blur never reaches full coverage along its
        // middle, and how far it falls short tells its width better than cells
        // do: v = 2 Phi(w / 2 sigma) - 1.
        let peak = f64::from(patch.most) / 255.0;
        if peak < THIN_STRIP_PEAK {
            short_side =
                short_side.min(2.0 * geometry.sigma * normal_quantile(f64::midpoint(1.0, peak)));
        }
        let short_side = short_side.max(1.0);
        let mut pane = Pane::of(clipped, short_side);
        if let Some([fx1, fy1, fx2, fy2]) = self.largest_face_rectangle(patch.bounds, id) {
            let face = Rect::new(
                count_f64(fx1) * cell - margin,
                count_f64(fy1) * cell - margin,
                pixels(fx1, fx2),
                pixels(fy1, fy2),
            );
            let interior = face.intersection(&clipped);
            if interior.has_area() {
                pane.interiors.push(interior);
            }
        }
        Some((pane, fit_byte(short_side, geometry.bezel_width)))
    }
}

/// Reads the panes out of the map.
///
/// # Errors
///
/// [`PaneError`] when the cells do not make a map of the size said, or the map is too large.
pub fn map_from_cells(grid: &CellGrid, geometry: &MapGeometry) -> Result<PaneMap, PaneError> {
    let count = grid.width.saturating_mul(grid.height);
    if count > MAX_CELLS {
        return Err(PaneError::TooManyCells(grid.width, grid.height));
    }
    if grid.cells.len() != count.saturating_mul(4) {
        return Err(PaneError::CellsMismatch(
            grid.cells.len(),
            grid.width,
            grid.height,
        ));
    }
    let mut scan = Scan {
        grid,
        seen: vec![false; count],
        owner: vec![0; count],
    };
    // a cell that belongs to no pane has room for the whole lens: nothing reads it
    let mut map = PaneMap {
        panes: Vec::new(),
        fit: vec![255; count],
    };
    for start in 0..count {
        if scan.seen.get(start).copied().unwrap_or(true) || scan.red(start) < CELL_IS_PANE {
            continue;
        }
        let patch = scan.flood(start);
        let id = map.panes.len().saturating_add(1);
        // the cells belong to the pane before its face is looked for in them
        let claimed: Vec<usize> = patch.members.clone();
        for member in &claimed {
            if let Some(owner) = scan.owner.get_mut(*member) {
                *owner = id;
            }
        }
        match scan.pane_of(&patch, geometry, id) {
            Some((pane, fit)) => {
                for member in &claimed {
                    if let Some(slot) = map.fit.get_mut(*member) {
                        *slot = fit;
                    }
                }
                map.panes.push(pane);
            }
            None => {
                for member in &claimed {
                    if let Some(owner) = scan.owner.get_mut(*member) {
                        *owner = 0;
                    }
                }
            }
        }
    }
    Ok(map)
}

/// The first two panes whose rectangles overlap.
fn first_overlap(panes: &[Pane]) -> Option<(usize, usize)> {
    for (i, a) in panes.iter().enumerate() {
        let after = i.saturating_add(1);
        if let Some(j) = panes
            .iter()
            .skip(after)
            .position(|b| a.rect.overlaps(&b.rect))
        {
            return Some((i, after.saturating_add(j)));
        }
    }
    None
}

/// Panes whose rectangles overlap would be drawn over one another: they
/// become one, with the lens of the smaller, which fits both, and the faces
/// of both.
#[must_use]
pub fn merge_overlapping(mut panes: Vec<Pane>) -> Vec<Pane> {
    // each pass joins two panes, so there are at most as many passes as panes
    for _ in 0..panes.len() {
        let Some((i, j)) = first_overlap(&panes) else {
            break;
        };
        if j >= panes.len() {
            break;
        }
        let other = panes.remove(j);
        if let Some(first) = panes.get_mut(i) {
            first.rect = first.rect.unite(&other.rect);
            first.short_side = first.short_side.min(other.short_side);
            first.interiors.extend(other.interiors);
        }
    }
    panes
}

/// One pane that holds all of these, with the smallest lens and every face.
fn union_of(panes: &[Pane]) -> Option<Pane> {
    let mut all = panes.first()?.clone();
    all.interiors.clear();
    for pane in panes {
        all.rect = all.rect.unite(&pane.rect);
        all.short_side = all.short_side.min(pane.short_side);
        all.interiors.extend(pane.interiors.iter().copied());
    }
    Some(all)
}

/// The panes that stand now: the last map read, plus whatever was redrawn
/// since, which is the only place a new pane can have appeared. Where the
/// outline is not known yet (`known` is none), one pane as large as the layer
/// and a lens fitted to that.
///
/// `small` says the layer is about the size of what it shows: whatever was
/// redrawn counts, however much of it there is.
#[must_use]
pub fn wanted_panes(
    known: Option<&[Pane]>,
    unknown: Option<&Rect>,
    whole: &Rect,
    small: bool,
) -> Vec<Pane> {
    let Some(known) = known else {
        return vec![Pane::of(*whole, whole.w.min(whole.h))];
    };
    let mut wanted = known.to_vec();
    if let Some(unknown) = unknown {
        let clipped = unknown.intersection(whole);
        if clipped.has_area()
            && (small || clipped.w * clipped.h <= UNKNOWN_AREA_LIMIT * whole.w * whole.h)
        {
            wanted.push(Pane::of(clipped, clipped.w.min(clipped.h)));
        }
    }
    let merged = merge_overlapping(wanted);
    if merged.len() > MAX_PANES {
        return union_of(&merged).into_iter().collect();
    }
    merged
}

#[cfg(test)]
mod tests;
