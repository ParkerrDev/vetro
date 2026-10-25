use super::*;

const SIZE: (f64, f64) = (400.0, 300.0);

fn largest(wobble: &Wobble) -> f64 {
    wobble
        .offset
        .iter()
        .map(|o| o[0].hypot(o[1]))
        .fold(0.0, f64::max)
}

#[test]
fn a_still_sheet_stays_still() {
    let mut w = Wobble::still();
    w.step(1.0, &WobbleSpec::JELLY);
    assert!(w.settled(1e-12));
    assert_eq!(w.offset_at(0.3, 0.7), [0.0, 0.0]);
}

#[test]
fn a_moved_sheet_trails_and_comes_back() {
    let spec = WobbleSpec::JELLY;
    let mut w = Wobble::still();
    w.hold_at(0.5, 0.0);
    for frame in 0..30 {
        let at = |n: f64| Rect::new(20.0 * n, 0.0, SIZE.0, SIZE.1);
        w.moved(&at(f64::from(frame)), &at(f64::from(frame + 1)), &spec);
        w.step(1.0 / 60.0, &spec);
    }
    let held = w.held.unwrap();
    assert_eq!(
        w.offset[held],
        [0.0, 0.0],
        "the held point moves with the window"
    );
    let far = w.offset[COUNT - 1];
    assert!(far[0] < -5.0, "the far corner trails: {far:?}");
    w.hold_at(-1.0, -1.0);
    assert_eq!(w.held, None);
    for _ in 0..600 {
        w.step(1.0 / 60.0, &spec);
    }
    assert!(w.settled(0.05), "settles: {}", largest(&w));
}

#[test]
fn a_squeeze_bows_the_sides_in() {
    let mut w = Wobble::still();
    w.squeeze(SIZE, 0.1);
    let corner = w.offset[0][0];
    let side = w.offset[CENTRE - NODES / 2][0];
    assert!(
        (corner - 0.5 * SIZE.0 * 0.1).abs() < 1e-9,
        "a corner goes in by the squeeze: {corner}"
    );
    assert!(
        side > corner * 1.5,
        "the middle of a side goes further: {side} against {corner}"
    );
}

#[test]
fn it_overshoots_when_it_bounces_and_not_when_it_does_not() {
    let wobbly = WobbleSpec::new(0.5, 0.6, 2.0, 0.3);
    let firm = WobbleSpec::new(0.5, 0.0, 2.0, 0.3);
    for (spec, overshoots) in [(wobbly, true), (firm, false)] {
        let mut w = Wobble::still();
        w.squeeze(SIZE, 0.1);
        let corner_start = w.offset[0][0];
        assert!(corner_start > 0.0, "the top left corner is pushed in");
        let mut least = corner_start;
        for _ in 0..240 {
            w.step(1.0 / 120.0, &spec);
            least = least.min(w.offset[0][0]);
        }
        // the bowed sides drag a firm sheet's corner a hair past its place
        assert_eq!(
            least < -0.1 * corner_start,
            overshoots,
            "spec {spec:?}: least {least} of {corner_start}"
        );
    }
}

#[test]
fn trailing_is_held_to_the_reach() {
    let spec = WobbleSpec::new(0.5, 0.5, 2.0, 0.1);
    let mut w = Wobble::still();
    w.hold_at(0.0, 0.0);
    w.moved(
        &Rect::new(0.0, 0.0, SIZE.0, SIZE.1),
        &Rect::new(5000.0, 5000.0, SIZE.0, SIZE.1),
        &spec,
    );
    assert!(largest(&w) <= 40.0 + 1e-9, "{}", largest(&w));
}

#[test]
fn long_waits_and_odd_numbers_do_no_harm() {
    let spec = WobbleSpec::new(f64::NAN, f64::INFINITY, f64::NAN, -1.0);
    let mut w = Wobble::still();
    w.squeeze(SIZE, 0.2);
    w.moved(
        &Rect::new(f64::NAN, 0.0, 1.0, 1.0),
        &Rect::new(0.0, 0.0, SIZE.0, SIZE.1),
        &spec,
    );
    w.step(1e9, &spec);
    w.step(f64::NAN, &spec);
    w.step(-1.0, &spec);
    assert!(w.offset.iter().flatten().all(|v| v.is_finite()));
    assert!(w.velocity.iter().flatten().all(|v| v.is_finite()));
}

#[test]
fn the_mesh_passes_through_every_point() {
    let mut w = Wobble::still();
    w.squeeze(SIZE, 0.2);
    w.offset[7] = [13.0, -4.0];
    for row in 0..NODES {
        for col in 0..NODES {
            let last = count_f64(NODES - 1);
            let got = w.offset_at(count_f64(col) / last, count_f64(row) / last);
            let want = w.offset[row * NODES + col];
            assert!(
                (got[0] - want[0]).abs() < 1e-9 && (got[1] - want[1]).abs() < 1e-9,
                "{col},{row}: {got:?} {want:?}"
            );
        }
    }
    let window = Rect::new(10.0, 20.0, SIZE.0, SIZE.1);
    let mut out = vec![0.0; 2 * 9 * 9];
    assert_eq!(wobble_mesh(&w, &window, (8, 8), &mut out), Ok(81));
    let still = Wobble::still();
    wobble_mesh(&still, &window, (8, 8), &mut out).unwrap();
    assert_eq!((out[0], out[1]), (10.0, 20.0));
    assert_eq!((out[160], out[161]), (410.0, 320.0));
}

#[test]
fn growing_leaves_the_far_side_behind_for_a_moment() {
    let spec = WobbleSpec::JELLY;
    let mut w = Wobble::still();
    w.hold_at(0.0, 0.0);
    w.moved(
        &Rect::new(0.0, 0.0, 400.0, 300.0),
        &Rect::new(0.0, 0.0, 440.0, 300.0),
        &spec,
    );
    assert_eq!(w.offset[0], [0.0, 0.0]);
    assert!(
        (w.offset[NODES - 1][0] + 40.0).abs() < 1e-9,
        "{:?}",
        w.offset[NODES - 1]
    );
    assert!((w.offset[2][0] + 20.0).abs() < 1e-9);
}

#[test]
fn holding_picks_the_nearest_point() {
    let mut w = Wobble::still();
    w.hold_at(0.0, 0.0);
    assert_eq!(w.held, Some(0));
    w.hold_at(1.0, 1.0);
    assert_eq!(w.held, Some(COUNT - 1));
    w.hold_at(0.5, 0.5);
    assert_eq!(w.held, Some(CENTRE));
    w.hold_at(0.3, 0.9);
    assert_eq!(w.held, Some(4 * NODES + 1));
}
