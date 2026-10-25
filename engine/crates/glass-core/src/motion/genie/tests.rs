#![expect(
    clippy::arithmetic_side_effects,
    reason = "tests count small grids with literals; an overflow there is a failing test"
)]

use super::*;

const WINDOW: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);
const BELOW: Mouth = Mouth {
    x: 300.0,
    y: 1000.0,
    width: 40.0,
};
const CELLS: (usize, usize) = (4, 8);

fn mesh(mouth: &Mouth, progress: f64) -> Vec<(f32, f32)> {
    let mut out = vec![0.0; mesh_len(CELLS.0, CELLS.1).unwrap()];
    let points = genie_mesh(&WINDOW, mouth, progress, CELLS, &mut out).unwrap();
    assert_eq!(points * 2, out.len());
    out.chunks_exact(2).map(|p| (p[0], p[1])).collect()
}

fn near(a: f32, b: f64) -> bool {
    (f64::from(a) - b).abs() < 1e-3
}

#[test]
fn at_the_start_the_window_is_as_it_is() {
    for mouth in [
        BELOW,
        Mouth {
            x: 300.0,
            y: -50.0,
            width: 10.0,
        },
        Mouth {
            x: 900.0,
            y: 250.0,
            width: 10.0,
        },
        Mouth {
            x: -900.0,
            y: 250.0,
            width: 10.0,
        },
    ] {
        for (k, (x, y)) in mesh(&mouth, 0.0).into_iter().enumerate() {
            let (i, j) = (k % (CELLS.0 + 1), k / (CELLS.0 + 1));
            assert!(
                near(x, 100.0 + 100.0 * count_f64(i)),
                "{mouth:?} point {k} x {x}"
            );
            assert!(
                near(y, 100.0 + 37.5 * count_f64(j)),
                "{mouth:?} point {k} y {y}"
            );
        }
    }
}

#[test]
fn at_the_end_all_of_it_is_in_the_mouth() {
    for (x, y) in mesh(&BELOW, 1.0) {
        assert!(near(y, 1000.0), "y {y}");
        assert!((280.0..=320.0).contains(&f64::from(x)), "x {x}");
    }
    let left = Mouth {
        x: -200.0,
        y: 250.0,
        width: 20.0,
    };
    for (x, y) in mesh(&left, 1.0) {
        assert!(near(x, -200.0), "x {x}");
        assert!((240.0..=260.0).contains(&f64::from(y)), "y {y}");
    }
}

#[test]
fn bending_keeps_the_far_edge_and_narrows_the_near_one() {
    let bent = mesh(&BELOW, Phases::GENIE.bent_by.min(Phases::GENIE.slide_from));
    let top: Vec<_> = bent.iter().take(CELLS.0 + 1).collect();
    let bottom: Vec<_> = bent.iter().skip((CELLS.0 + 1) * CELLS.1).collect();
    let span = |row: &[&(f32, f32)]| row.last().unwrap().0 - row.first().unwrap().0;
    assert!(span(&top) > span(&bottom), "the near edge is narrower");
    assert!(span(&top) <= 400.0 + 1e-3);
}

#[test]
fn rows_never_pass_the_mouth_and_keep_their_order() {
    for step in 0..=20 {
        let points = mesh(&BELOW, f64::from(step) / 20.0);
        let ys: Vec<f32> = points.iter().step_by(CELLS.0 + 1).map(|p| p.1).collect();
        assert!(
            ys.windows(2).all(|w| w[0] <= w[1] + 1e-3),
            "step {step}: {ys:?}"
        );
        assert!(ys.iter().all(|&y| f64::from(y) <= 1000.0 + 1e-3));
    }
}

#[test]
fn a_mouth_inside_the_window_is_on_its_near_edge() {
    let inside = Mouth {
        x: 300.0,
        y: 380.0,
        width: 10.0,
    };
    for (_, y) in mesh(&inside, 1.0) {
        assert!(near(y, 400.0), "y {y}");
    }
}

#[test]
fn bad_sizes_are_refused() {
    let mut out = [0.0; 4];
    assert_eq!(
        genie_mesh(&WINDOW, &BELOW, 0.5, (0, 3), &mut out),
        Err(MeshError::Cells(0, 3))
    );
    assert_eq!(
        genie_mesh(&WINDOW, &BELOW, 0.5, (MAX_CELLS + 1, 1), &mut out),
        Err(MeshError::Cells(MAX_CELLS + 1, 1))
    );
    assert_eq!(
        genie_mesh(&WINDOW, &BELOW, 0.5, (1, 1), &mut out),
        Err(MeshError::Room(4, 8))
    );
}

#[test]
fn odd_numbers_are_held_in_range() {
    let mut out = vec![0.0; mesh_len(2, 2).unwrap()];
    for progress in [f64::NAN, f64::INFINITY, -3.0, 7.0] {
        genie_mesh(&WINDOW, &BELOW, progress, (2, 2), &mut out).unwrap();
        assert!(out.iter().all(|v| v.is_finite()));
    }
    let empty = Rect::new(10.0, 10.0, 0.0, 0.0);
    genie_mesh(&empty, &BELOW, 0.5, (2, 2), &mut out).unwrap();
    assert!(out.iter().all(|v| v.is_finite()));
}
