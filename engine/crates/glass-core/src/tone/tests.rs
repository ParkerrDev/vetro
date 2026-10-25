use super::*;

/// Luminances from black to white, `n` steps apart.
fn levels(n: u16) -> impl Iterator<Item = f32> {
    (0..=n).map(move |i| f32::from(i) / f32::from(n))
}

#[test]
fn a_strong_dim_turns_white_darker_than_grey() {
    // The window's glass on the author's laptop: white came out darker than a quarter grey.
    assert!(curve(1.0, 0.9, 0.0, 1.0) < curve(0.25, 0.9, 0.0, 1.0));
    let peak = peak(0.9, 0.0, 1.0);
    assert!(peak.at > DIM_FROM && peak.at < DIM_TO, "{peak:?}");
    assert!(held(1.0, 0.9, 0.0, 1.0, peak) >= held(0.25, 0.9, 0.0, 1.0, peak));
}

#[test]
fn below_the_peak_the_curve_is_as_it_was() {
    let peak = peak(0.9, 0.0, 1.0);
    for lum in levels(400).filter(|&lum| lum <= peak.at) {
        assert!((held(lum, 0.9, 0.0, 1.0, peak) - curve(lum, 0.9, 0.0, 1.0)).abs() < f32::EPSILON);
    }
}

#[test]
fn past_it_white_is_held_where_the_curve_turned() {
    let peak = peak(0.9, 0.0, 1.0);
    assert!((held(1.0, 0.9, 0.0, 1.0, peak) - peak.value).abs() < f32::EPSILON);
    assert!(peak.value > 0.27 && peak.value < 0.29, "{peak:?}");
}

#[test]
fn a_gentle_dim_is_left_alone() {
    for dim in [0.0, 0.25, 0.3, 0.36] {
        assert_eq!(peak(dim, 0.0, 0.82).at, NEVER, "dim {dim}");
    }
    assert!(peak(0.37, 0.0, 1.0).at < NEVER);
}

#[test]
fn nothing_brighter_behind_comes_out_darker() {
    let grid = |n: u16, top: f32| (0..=n).map(move |i| top * f32::from(i) / f32::from(n));
    for dim in grid(15, 1.5) {
        for boost in grid(4, 1.0) {
            for brightness in [0.8, 1.0, 1.2] {
                let peak = peak(dim, boost, brightness);
                let mut before = f32::NEG_INFINITY;
                for lum in levels(3000) {
                    let now = held(lum, dim, boost, brightness, peak);
                    assert!(
                        now >= before - 1e-5,
                        "dim {dim} boost {boost} brightness {brightness}: {lum} gives {now} after {before}"
                    );
                    before = now;
                }
            }
        }
    }
}

#[test]
fn what_is_not_a_number_changes_nothing() {
    assert_eq!(peak(f32::NAN, 0.0, 1.0).at, NEVER);
}
