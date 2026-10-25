#![expect(
    clippy::arithmetic_side_effects,
    reason = "tests lay out small synthetic maps with counters and literals; an overflow there is a failing test"
)]

use super::*;

/// A map to draw cells on: red and green of each cell, as the GPU writes them.
struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<u8>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![0; width * height * 4],
        }
    }

    /// Cells `x1..=x2` by `y1..=y2` get this red and green.
    fn fill(&mut self, [x1, y1, x2, y2]: [usize; 4], red: u8, green: u8) {
        for y in y1..=y2 {
            for x in x1..=x2 {
                let at = (y * self.width + x) * 4;
                self.cells[at] = red;
                self.cells[at + 1] = green;
            }
        }
    }

    /// A pane: a ring of surroundings, then cells inside the outline, then face.
    fn pane(&mut self, [x1, y1, x2, y2]: [usize; 4]) {
        self.fill([x1, y1, x2, y2], 100, 0);
        self.fill([x1 + 1, y1 + 1, x2 - 1, y2 - 1], 255, 0);
        self.fill([x1 + 2, y1 + 2, x2 - 2, y2 - 2], 255, 255);
    }

    fn map(&self, geometry: &MapGeometry) -> Result<PaneMap, PaneError> {
        map_from_cells(
            &CellGrid {
                cells: &self.cells,
                width: self.width,
                height: self.height,
            },
            geometry,
        )
    }
}

fn geometry(width: usize, height: usize) -> MapGeometry {
    MapGeometry {
        margin: 0,
        field_scale: 1,
        sigma: 10.0,
        bezel_width: 20.0,
        box_size: (
            count_f64(width * CELL_PIXELS),
            count_f64(height * CELL_PIXELS),
        ),
    }
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect::new(x, y, w, h)
}

#[test]
fn the_thresholds_are_the_bytes_of_their_fractions() {
    assert_eq!(
        CELL_IS_PANE,
        u8::try_from(crate::num::trunc_i32(f64::from(0.08_f32 * 255.0_f32))).unwrap()
    );
    assert_eq!(
        CELL_IS_INSIDE,
        u8::try_from(crate::num::trunc_i32(f64::from(0.5_f32 * 255.0_f32))).unwrap()
    );
}

#[test]
fn one_patch_of_cells_is_one_pane_with_its_face() {
    let mut canvas = Canvas::new(30, 20);
    canvas.pane([4, 3, 17, 10]);
    let map = canvas.map(&geometry(30, 20)).unwrap();
    assert_eq!(map.panes.len(), 1);
    let pane = &map.panes[0];
    assert_eq!(
        pane.rect,
        rect(32.0, 24.0, 14.0 * 8.0, 8.0 * 8.0),
        "the patch including its surroundings"
    );
    assert_eq!(
        pane.interiors,
        vec![rect(48.0, 40.0, 10.0 * 8.0, 4.0 * 8.0)],
        "and the face inside it"
    );
    assert!(pane.short_side >= 1.0);
}

#[test]
fn two_patches_are_two_panes_and_the_fit_says_which_cells_are_whose() {
    let mut canvas = Canvas::new(40, 20);
    canvas.pane([2, 2, 15, 15]);
    canvas.pane([22, 5, 36, 9]);
    let map = canvas.map(&geometry(40, 20)).unwrap();
    assert_eq!(map.panes.len(), 2);
    assert_eq!(map.fit.len(), 40 * 20);
    let big = map.fit[8 * 40 + 8];
    let small = map.fit[7 * 40 + 28];
    assert!(
        big > small,
        "a taller pane has room for more of the lens: {big} against {small}"
    );
    assert_eq!(
        map.fit[0], 255,
        "a cell of no pane has room for the whole lens"
    );
    assert!(map.panes[0].short_side > map.panes[1].short_side);
}

#[test]
fn cells_that_touch_at_a_corner_are_one_pane() {
    let mut canvas = Canvas::new(30, 20);
    canvas.pane([2, 2, 9, 9]);
    canvas.pane([10, 10, 17, 17]);
    assert_eq!(canvas.map(&geometry(30, 20)).unwrap().panes.len(), 1);
}

#[test]
fn surroundings_without_an_inside_are_no_pane() {
    let mut canvas = Canvas::new(20, 20);
    canvas.fill([3, 3, 12, 12], 100, 0);
    assert_eq!(canvas.map(&geometry(20, 20)).unwrap().panes.len(), 0);
    canvas.fill([5, 5, 6, 6], 126, 0);
    assert!(
        canvas.map(&geometry(20, 20)).unwrap().panes.is_empty(),
        "one short of the outline is not inside"
    );
    canvas.fill([5, 5, 6, 6], 127, 0);
    assert_eq!(canvas.map(&geometry(20, 20)).unwrap().panes.len(), 1);
}

#[test]
fn a_patch_outside_the_layer_is_dropped() {
    let mut canvas = Canvas::new(40, 20);
    canvas.pane([30, 3, 38, 12]);
    let mut small = geometry(40, 20);
    small.box_size = (100.0, 160.0);
    assert_eq!(canvas.map(&small).unwrap().panes.len(), 0);
    assert!(
        canvas.map(&small).unwrap().fit.iter().all(|f| *f == 255),
        "and its cells claim nothing"
    );
}

#[test]
fn a_pane_is_cut_to_the_layer_and_the_margin_is_taken_off() {
    let mut canvas = Canvas::new(30, 20);
    canvas.pane([0, 0, 13, 9]);
    let mut shifted = geometry(30, 20);
    shifted.margin = 16;
    let pane = canvas.map(&shifted).unwrap().panes.remove(0);
    assert_eq!(
        pane.rect,
        rect(0.0, 0.0, 14.0 * 8.0 - 16.0, 10.0 * 8.0 - 16.0),
        "the margin is field around the layer"
    );
    assert!(pane.interiors[0].x >= 0.0 && pane.interiors[0].y >= 0.0);
}

#[test]
fn a_strip_thinner_than_the_blur_is_told_by_how_high_it_peaks() {
    let mut canvas = Canvas::new(40, 10);
    canvas.fill([2, 1, 30, 8], 150, 0);
    let strip = canvas.map(&geometry(40, 10)).unwrap().panes.remove(0);
    let peak = 150.0 / 255.0;
    let expected = 2.0 * 10.0 * normal_quantile(f64::midpoint(1.0, peak));
    assert!(
        (strip.short_side - expected).abs() < 1e-9,
        "{} against {expected}",
        strip.short_side
    );
    assert!(strip.interiors.is_empty(), "and has no face");
}

#[test]
fn the_face_is_the_largest_rectangle_of_face_cells() {
    let mut canvas = Canvas::new(40, 30);
    canvas.fill([2, 2, 30, 25], 255, 0);
    // an L of face cells: a 3 by 20 column and a 12 by 4 row; the 12 by 4 row is the larger rectangle
    canvas.fill([4, 4, 6, 23], 255, 255);
    canvas.fill([4, 20, 15, 23], 255, 255);
    let map = canvas.map(&geometry(40, 30)).unwrap();
    assert_eq!(
        map.panes[0].interiors,
        vec![rect(32.0, 32.0, 3.0 * 8.0, 20.0 * 8.0)],
        "3 by 20 beats 12 by 4 by 12 cells"
    );
}

#[test]
fn a_map_that_is_not_the_size_said_is_refused() {
    let canvas = Canvas::new(10, 10);
    let grid = CellGrid {
        cells: &canvas.cells[..100],
        width: 10,
        height: 10,
    };
    assert_eq!(
        map_from_cells(&grid, &geometry(10, 10)),
        Err(PaneError::CellsMismatch(100, 10, 10))
    );
    let huge = CellGrid {
        cells: &[],
        width: 1 << 12,
        height: 1 << 12,
    };
    assert_eq!(
        map_from_cells(&huge, &geometry(1, 1)),
        Err(PaneError::TooManyCells(1 << 12, 1 << 12))
    );
    let empty = CellGrid {
        cells: &[],
        width: 0,
        height: 0,
    };
    assert_eq!(
        map_from_cells(&empty, &geometry(1, 1)).unwrap(),
        PaneMap::default()
    );
}

/// Rectangles at pseudo-random places: the maps a layer of cards would give.
fn scattered_canvas(seed: u64, width: usize, height: usize) -> Canvas {
    let mut state = seed;
    let mut next = |bound: usize| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(state >> 33).unwrap() % bound
    };
    let mut canvas = Canvas::new(width, height);
    for _ in 0..(3 + next(8)) {
        let (x1, y1) = (next(width - 8), next(height - 8));
        let (w, h) = (
            5 + next(width - x1 - 4).min(40),
            5 + next(height - y1 - 4).min(30),
        );
        canvas.pane([x1, y1, (x1 + w).min(width - 1), (y1 + h).min(height - 1)]);
    }
    canvas
}

#[test]
fn every_face_rectangle_lies_inside_its_pane_and_the_counts_are_bounded() {
    for seed in 1..=60 {
        let (width, height) = (80, 60);
        let canvas = scattered_canvas(seed, width, height);
        let map = canvas.map(&geometry(width, height)).unwrap();
        let whole = rect(0.0, 0.0, count_f64(width * 8), count_f64(height * 8));
        assert!(
            map.panes.len() <= width * height,
            "no more panes than cells"
        );
        assert_eq!(map.fit.len(), width * height);
        for pane in &map.panes {
            assert!(
                pane.rect.has_area() && pane.short_side >= 1.0,
                "seed {seed}"
            );
            assert_eq!(
                pane.rect.intersection(&whole),
                pane.rect,
                "cut to the layer"
            );
            assert!(pane.interiors.len() <= 1);
            for face in &pane.interiors {
                assert!(face.has_area());
                assert_eq!(
                    face.intersection(&pane.rect),
                    *face,
                    "seed {seed}: {face:?} lies in {:?}",
                    pane.rect
                );
            }
        }
    }
}

#[test]
fn a_noisy_map_still_gives_bounded_answers() {
    let (width, height) = (64, 48);
    let mut state: u64 = 99;
    let cells: Vec<u8> = (0..width * height * 4)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            u8::try_from(state >> 56).unwrap()
        })
        .collect();
    let map = map_from_cells(
        &CellGrid {
            cells: &cells,
            width,
            height,
        },
        &geometry(width, height),
    )
    .unwrap();
    assert!(map.panes.len() <= width * height && map.fit.len() == width * height);
}

fn pane(rect: Rect, short_side: f64, faces: &[Rect]) -> Pane {
    Pane {
        rect,
        short_side,
        interiors: faces.to_vec(),
    }
}

#[test]
fn panes_that_overlap_become_one_with_the_smaller_lens_and_both_faces() {
    let a = pane(
        rect(0.0, 0.0, 100.0, 50.0),
        50.0,
        &[rect(10.0, 10.0, 30.0, 10.0)],
    );
    let b = pane(
        rect(90.0, 20.0, 60.0, 80.0),
        60.0,
        &[rect(100.0, 30.0, 10.0, 10.0)],
    );
    let merged = merge_overlapping(vec![a, b]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].rect, rect(0.0, 0.0, 150.0, 100.0));
    assert!((merged[0].short_side - 50.0).abs() < 1e-12);
    assert_eq!(merged[0].interiors.len(), 2);
}

#[test]
fn a_chain_of_overlaps_ends_as_one_and_panes_that_only_touch_stay_apart() {
    let chain = vec![
        pane(rect(0.0, 0.0, 10.0, 10.0), 10.0, &[]),
        pane(rect(8.0, 0.0, 10.0, 10.0), 10.0, &[]),
        pane(rect(16.0, 0.0, 10.0, 10.0), 10.0, &[]),
        pane(rect(26.0, 0.0, 10.0, 10.0), 10.0, &[]),
    ];
    let merged = merge_overlapping(chain);
    assert_eq!(merged.len(), 2, "the fourth only touches the third");
    assert_eq!(merged[0].rect, rect(0.0, 0.0, 26.0, 10.0));
}

#[test]
fn before_the_first_map_the_layer_is_one_pane_fitted_to_its_box() {
    let whole = rect(0.0, 0.0, 400.0, 300.0);
    let wanted = wanted_panes(None, Some(&rect(1.0, 1.0, 5.0, 5.0)), &whole, false);
    assert_eq!(wanted, vec![Pane::of(whole, 300.0)]);
}

#[test]
fn what_was_redrawn_since_the_map_may_hold_a_new_pane() {
    let whole = rect(0.0, 0.0, 1000.0, 800.0);
    let known = vec![pane(rect(0.0, 0.0, 100.0, 100.0), 100.0, &[])];
    let small_area = rect(500.0, 500.0, 50.0, 40.0);
    let wanted = wanted_panes(Some(&known), Some(&small_area), &whole, false);
    assert_eq!(wanted.len(), 2);
    assert_eq!(wanted[1], Pane::of(small_area, 40.0));
    let large_area = rect(100.0, 100.0, 900.0, 600.0);
    assert_eq!(
        wanted_panes(Some(&known), Some(&large_area), &whole, false).len(),
        1,
        "a surface declared all redrawn says nothing"
    );
    assert_eq!(
        wanted_panes(Some(&known), Some(&large_area), &whole, true).len(),
        2,
        "unless the layer is small"
    );
    assert_eq!(
        wanted_panes(
            Some(&known),
            Some(&rect(2000.0, 0.0, 5.0, 5.0)),
            &whole,
            true
        )
        .len(),
        1,
        "outside the layer is nothing"
    );
}

#[test]
fn more_than_sixteen_panes_are_drawn_as_one() {
    let whole = rect(0.0, 0.0, 2000.0, 100.0);
    let many: Vec<Pane> = (0..17)
        .map(|i| {
            pane(
                rect(f64::from(i) * 100.0, 0.0, 50.0, 50.0),
                50.0 - f64::from(i),
                &[rect(f64::from(i) * 100.0, 0.0, 5.0, 5.0)],
            )
        })
        .collect();
    let wanted = wanted_panes(Some(&many), None, &whole, false);
    assert_eq!(wanted.len(), 1);
    assert_eq!(wanted[0].interiors.len(), 17);
    assert!((wanted[0].short_side - 34.0).abs() < 1e-12);
    assert_eq!(wanted[0].rect, rect(0.0, 0.0, 1650.0, 50.0));
}

#[test]
fn the_fit_is_the_short_side_over_four_bezels_as_a_byte() {
    assert_eq!(fit_byte(80.0, 20.0), 255);
    assert_eq!(fit_byte(40.0, 20.0), 128);
    assert_eq!(fit_byte(1000.0, 20.0), 255);
    assert_eq!(fit_byte(10.0, 0.0), 255, "no bezel, no need to fit");
}
