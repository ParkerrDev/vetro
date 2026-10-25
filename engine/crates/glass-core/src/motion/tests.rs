use super::*;

fn near(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

/// A fixed sequence in `[0, 1)`.
fn unit_sequence(count: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            f64::from(u32::try_from(state >> 40).unwrap()) / f64::from(1_u32 << 24)
        })
        .collect()
}

fn spring(from: f64, to: f64, spec: SpringSpec) -> Spring {
    let mut s = Spring::at_rest(from, spec);
    s.retarget(to);
    s
}

#[test]
fn a_spring_is_the_perceptual_pair_turned_into_physics() {
    let spec = SpringSpec::new(0.5, 0.2);
    assert!(near(spec.omega(), TAU / 0.5, 1e-12));
    assert!(near(spec.stiffness(), (TAU / 0.5).powi(2), 1e-9));
    assert!(near(spec.damping_ratio(), 0.8, 1e-12));
    assert!(
        near(spec.damping(), 4.0 * std::f64::consts::PI * 0.8 / 0.5, 1e-9),
        "damping 4 pi (1 - bounce) / duration"
    );
    assert!(
        near(SpringSpec::new(0.0, 5.0).duration, MIN_DURATION, 0.0)
            && near(SpringSpec::new(1.0, 5.0).bounce, 1.0, 0.0)
    );
    assert!(near(SpringSpec::new(f64::NAN, f64::NAN).bounce, 0.0, 0.0));
}

#[test]
fn the_named_springs_are_those_of_the_reference() {
    let pairs = [
        (named::PRESS, 0.24, 0.20),
        (named::RELEASE, 0.50, 0.42),
        (named::GLOW, 0.42, 0.0),
        (named::MORPH, 0.42, 0.18),
        (named::LAYOUT, 0.40, 0.16),
        (named::TRACK, 0.18, 0.0),
        (named::SWITCH, 0.47, 0.30),
    ];
    for (spec, duration, bounce) in pairs {
        assert!(near(spec.duration, duration, 0.0) && near(spec.bounce, bounce, 0.0));
    }
    assert!(
        near(named::SWITCH.damping_ratio(), 0.7, 1e-12),
        "the switch's measured damping of 0.7"
    );
}

#[test]
fn at_critical_damping_it_settles_without_overshoot() {
    for (from, to) in [(0.0, 1.0), (10.0, -3.0), (-5.0, -5.5)] {
        for duration in [0.05, 0.4, 2.0] {
            let mut s = spring(from, to, SpringSpec::new(duration, 0.0));
            let mut previous = (s.position - to).abs();
            for _ in 0..4000 {
                s.step(duration / 200.0);
                let distance = (s.position - to).abs();
                assert!(
                    (s.position - to) * (from - to) >= -1e-12,
                    "never past the target: {from} to {to}"
                );
                assert!(distance <= previous + 1e-12, "and always closer");
                previous = distance;
            }
            assert!(
                near(s.position, to, 1e-6) && s.velocity.abs() < 1e-5,
                "it settled"
            );
        }
    }
}

#[test]
fn a_bouncy_spring_overshoots_once_and_comes_back() {
    let mut s = spring(0.0, 1.0, named::RELEASE);
    let mut highest = 0.0_f64;
    for _ in 0..600 {
        s.step(0.005);
        highest = highest.max(s.position);
    }
    assert!(highest > 1.05, "bounce 0.42 goes past: {highest}");
    assert!(near(s.position, 1.0, 1e-3));
}

#[test]
fn a_negative_bounce_creeps_and_never_overshoots() {
    let mut s = spring(0.0, 1.0, SpringSpec::new(0.4, -0.8));
    for _ in 0..2000 {
        s.step(0.005);
        assert!(s.position <= 1.0 + 1e-12);
    }
    assert!(
        s.position > 0.99,
        "and gets there in the end: {}",
        s.position
    );
}

#[test]
fn it_is_stable_for_any_time_step() {
    let randoms = unit_sequence(900, 7);
    for chunk in randoms.chunks(3) {
        let spec = SpringSpec::new(0.02 + 3.0 * chunk[0], -1.0 + 2.0 * chunk[1]);
        let mut s = spring(2.0, -1.0, spec);
        s.velocity = 50.0 * (chunk[2] - 0.5);
        let envelope = 3.0 + s.velocity.abs() * spec.duration + 1.0;
        let dt = 10.0_f64.powf(-9.0 + 18.0 * chunk[2]);
        s.step(dt);
        assert!(
            s.position.is_finite() && s.velocity.is_finite(),
            "dt {dt} {spec:?}"
        );
        assert!(
            (s.position + 1.0).abs() <= 1000.0 * envelope,
            "bounded: {} at dt {dt}",
            s.position
        );
        if dt > 100.0 * spec.duration && spec.damping_ratio() > 0.2 {
            assert!(
                near(s.position, -1.0, 1e-3),
                "a long step lands on the target: {}",
                s.position
            );
        }
    }
}

#[test]
fn two_steps_end_where_one_of_their_sum_does() {
    let randoms = unit_sequence(300, 11);
    for chunk in randoms.chunks(3) {
        let spec = SpringSpec::new(0.1 + chunk[0], -0.9 + 1.8 * chunk[1]);
        let (a, b) = (0.3 * chunk[2], 0.5 * chunk[0] + 0.01);
        let mut split = spring(1.0, 4.0, spec);
        split.velocity = 3.0;
        let mut whole = split;
        split.step(a);
        split.step(b);
        whole.step(a + b);
        assert!(
            near(split.position, whole.position, 1e-9)
                && near(split.velocity, whole.velocity, 1e-8),
            "{spec:?}"
        );
    }
}

#[test]
fn retargeting_keeps_the_speed_it_had() {
    let mut s = spring(0.0, 10.0, named::LAYOUT);
    for _ in 0..30 {
        s.step(0.01);
    }
    let (position, velocity) = (s.position, s.velocity);
    assert!(velocity > 1.0, "it is moving: {velocity}");
    s.retarget(-4.0);
    assert!(
        near(s.position, position, 0.0) && near(s.velocity, velocity, 0.0),
        "momentum carried over"
    );
    s.step(0.0001);
    assert!(
        s.velocity > 0.0,
        "it still goes on a little the way it was going before turning"
    );
    for _ in 0..2000 {
        s.step(0.005);
    }
    assert!(s.settled(1e-3) && near(s.position, -4.0, 1e-3));
}

#[test]
fn changing_the_spring_keeps_position_and_speed_too() {
    let mut s = spring(0.0, 5.0, named::TRACK);
    s.step(0.1);
    let (position, velocity) = (s.position, s.velocity);
    s.respec(named::LAYOUT);
    assert!(near(s.position, position, 0.0) && near(s.velocity, velocity, 0.0));
    assert_eq!(s.spec(), named::LAYOUT);
}

#[test]
fn no_time_or_nonsense_time_moves_nothing() {
    let mut s = spring(0.0, 1.0, named::PRESS);
    for dt in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        s.step(dt);
        assert!(
            near(s.position, 0.0, 0.0) && near(s.velocity, 0.0, 0.0),
            "dt {dt}"
        );
    }
}

#[test]
fn the_selector_stretches_with_its_speed_and_keeps_its_area() {
    assert_eq!(selector_stretch(0.0, true), Stretch::NONE);
    let slow = selector_stretch(400.0, true);
    assert!(near(slow.along, 1.14, 1e-12) && near(slow.across, 1.0 / 1.14_f64.sqrt(), 1e-12));
    assert!(
        near(
            selector_stretch(400.0, false).along,
            1.0 + 0.14 * 0.6,
            1e-12
        ),
        "0.6 of it when not dragged"
    );
    assert!(
        near(selector_stretch(1e6, true).along, 1.2, 1e-12),
        "capped at 0.2"
    );
    assert!(
        near(selector_stretch(-400.0, true).along, slow.along, 0.0),
        "either way"
    );
    for speed in [10.0, 300.0, 5000.0] {
        let s = selector_stretch(speed, true);
        assert!(near(s.along * s.across * s.across, 1.0, 1e-12), "area kept");
    }
}

#[test]
fn a_switch_stretches_with_its_speed_when_pressed() {
    assert!(near(control_stretch(200.0, 1.0).along, 1.14, 1e-12));
    assert!(
        near(control_stretch(1e6, 1.0).along, 1.28, 1e-12),
        "capped at 0.28"
    );
    assert_eq!(control_stretch(1000.0, 0.0), Stretch::NONE);
    assert_eq!(Stretch::of(-1.0), Stretch::NONE);
}

#[test]
fn the_rubber_band_gives_way_and_never_passes_its_limit() {
    let limit = 20.0;
    assert!(near(rubber_band(0.0, limit), 0.0, 0.0));
    let mut previous = 0.0;
    for step in 1..200 {
        let shown = rubber_band(f64::from(step) * 5.0, limit);
        assert!(shown > previous && shown < limit, "rising, below the limit");
        previous = shown;
    }
    assert!(
        near(rubber_band(-30.0, limit), -rubber_band(30.0, limit), 1e-12),
        "odd"
    );
    assert!(
        near(rubber_band(1e-6, limit) / 1e-6, 0.55, 1e-4),
        "at the start it follows at 0.55"
    );
    assert!(near(rubber_band(5.0, 0.0), 0.0, 0.0) && near(rubber_band(f64::NAN, limit), 0.0, 0.0));
}

#[test]
fn the_limits_and_growth_depend_on_the_size() {
    assert!(near(control_rubber_limit(40.0, 20.0), 10.0, 1e-12));
    assert!(
        near(surface_rubber_limit(100.0, 50.0), 7.0, 1e-12)
            && near(surface_rubber_limit(1000.0, 5.0), 10.0, 1e-12)
    );
    assert!(
        near(press_growth(100.0, 40.0), 0.08, 1e-12)
            && near(press_growth(20.0, 20.0), 0.16, 1e-12)
            && near(press_growth(0.0, 0.0), 0.16, 0.0)
    );
}

#[test]
fn a_thumb_turns_from_a_white_face_into_a_lens() {
    let at_rest = thumb_lens(0.0, ThumbKind::SliderOrToggle);
    assert!(
        near(at_rest.blur, 8.0, 0.0)
            && near(at_rest.face_alpha, 1.0, 0.0)
            && near(at_rest.refraction_amount, 0.0, 0.0)
    );
    let pressed = thumb_lens(1.0, ThumbKind::TabBar);
    assert!(
        near(pressed.blur, 0.0, 0.0)
            && near(pressed.refraction_height, 10.0, 0.0)
            && near(pressed.refraction_amount, 14.0, 0.0)
    );
    assert!(
        near(pressed.inner_shadow_radius, 8.0, 0.0)
            && near(pressed.press_scale, 78.0 / 56.0, 1e-12)
            && near(pressed.highlight_alpha, 1.0, 0.0)
    );
    assert!(
        near(
            thumb_lens(1.0, ThumbKind::SliderOrToggle).inner_shadow_radius,
            4.0,
            0.0
        ) && near(
            thumb_lens(0.5, ThumbKind::SliderOrToggle).press_scale,
            1.5,
            0.0
        )
    );
    assert!(
        near(thumb_lens(3.0, ThumbKind::TabBar).face_alpha, 0.0, 0.0),
        "progress is held to 0..1"
    );
}

#[test]
fn speed_makes_a_selector_a_lens() {
    assert!(
        near(lens_progress(0.0, SUGGESTED_REFERENCE_SPEED), 0.0, 0.0)
            && near(lens_progress(-300.0, SUGGESTED_REFERENCE_SPEED), 1.0, 1e-12)
    );
    assert!(near(
        lens_progress(150.0, SUGGESTED_REFERENCE_SPEED),
        0.5,
        1e-12
    ));
    assert!(
        near(smoothstep(0.3, 0.95, 0.3), 0.0, 0.0)
            && near(smoothstep(0.3, 0.95, 0.95), 1.0, 0.0)
            && near(smoothstep(1.0, 1.0, 0.5), 0.0, 0.0)
    );
}
