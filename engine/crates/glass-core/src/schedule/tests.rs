use super::*;
use crate::geometry::Rect;
use crate::options::{Values, model_params};

fn scene_params() -> ModelParams {
    model_params(&Values::UNSET, 1.6)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn the_reductions_follow_the_blur() {
    let reductions: Vec<i32> = [0.0, 2.9, 3.0, 6.9, 7.0, 13.9, 14.0, 40.0]
        .map(frost_reduction)
        .to_vec();
    assert_eq!(reductions, vec![1, 1, 2, 2, 4, 4, 8, 8]);
    assert_eq!(
        [3.9, 4.0, 7.9, 8.0, 20.0].map(field_reduction),
        [1, 2, 2, 4, 4]
    );
}

#[test]
fn the_bar_of_the_benchmark_scene_is_laid_out_as_worked_out_by_hand() {
    // 1590 by 30 points at 1.6: 2544 by 48 pixels; bezel 32, so a blur of 10.67 pixels
    let layout = layout(&scene_params(), [2544.0, 48.0]);
    assert_eq!(layout.field_scale, 4);
    assert_eq!(
        layout.margin, 96,
        "the shadow reaches 77 pixels: the next multiple of 32"
    );
    assert_eq!(layout.shadow_reach, 77);
    assert_eq!(layout.field_texels, [684, 60]);
    assert_eq!(layout.shadow_texels, [342, 30]);
    assert_eq!(layout.cells, [342, 30]);
    assert!(layout.shadow_on && layout.small);
    assert_eq!(
        layout.initial_fit, 96,
        "48 pixels over four bezels of 32 is three eighths... of 255"
    );
    assert!((f64::from(layout.blur_sigma) - 10.666_666_666 / 4.0).abs() < 1e-6);
    assert_eq!(
        layout.blur_radius, 11,
        "four deviations of the field's blur, in texels"
    );
}

#[test]
fn a_full_screen_layer_is_not_small() {
    let layout = layout(&scene_params(), [2560.0, 1600.0]);
    assert_eq!(layout.field_texels, [688, 448]);
    assert!(!layout.small);
    assert_eq!(layout.initial_fit, 255);
}

#[test]
fn the_margin_is_a_multiple_of_the_step_and_never_more_than_the_most() {
    for radius in [0.0_f32, 10.0, 40.0, 100.0, 400.0] {
        for bezel in [4.0_f64, 32.0, 100.0] {
            let mut params = scene_params();
            params.shadow_radius = radius;
            params.lens.bezel_width = bezel;
            let layout = layout(&params, [300.0, 200.0]);
            assert!(
                layout.margin % MARGIN_STEP == 0 && layout.margin <= MAX_MARGIN,
                "radius {radius} bezel {bezel}: {}",
                layout.margin
            );
            assert!(layout.shadow_reach <= layout.margin);
            assert!(layout.field_texels[0] * layout.field_scale >= 300 + 2 * layout.margin);
        }
    }
}

#[test]
fn without_a_shadow_there_is_no_reach() {
    let mut params = scene_params();
    params.shadow_opacity = 0.0;
    let layout = layout(&params, [300.0, 200.0]);
    assert!(!layout.shadow_on && layout.shadow_reach == 0);
    assert_eq!(
        layout.margin, 64,
        "the field's blur alone: 52 pixels, rounded up to 64"
    );
}

#[test]
fn the_frost_of_the_default_glass_is_one_light_blur_at_a_quarter() {
    let plan = frost_plan(9.6);
    assert_eq!((plan.reduction, plan.passes, plan.pairs), (4, 1, 4));
    assert!((f64::from(plan.sigma) - 2.347_338_914_871_216).abs() < 1e-6);
    assert!(plan.face_applied);
    assert_eq!(frost_reach(9.6), 24);
    assert_eq!(frost_reach(0.2), 0);
}

#[test]
fn a_blur_too_wide_for_one_pass_is_run_more_than_once() {
    let plan = frost_plan(100.0);
    assert!(plan.passes > 1 && plan.pairs == MAX_BLUR_PAIRS, "{plan:?}");
    for frost in [0.0_f32, 0.5, 1.0, 2.0, 3.2, 9.6, 14.0, 22.4, 60.0, 400.0] {
        let plan = frost_plan(frost);
        assert!(
            (1..=4).contains(&plan.passes) && (1..=MAX_BLUR_PAIRS).contains(&plan.pairs),
            "frost {frost}"
        );
        assert_eq!(
            plan.face_applied,
            plan.sigma > 0.3,
            "the face rides on the Gaussian"
        );
        let joined = f64::from(plan.pass_sigma) * f64::from(plan.passes).sqrt();
        if plan.passes < 4 {
            assert!(
                (joined - f64::from(plan.sigma)).abs() < 1e-4 * (1.0 + f64::from(plan.sigma)),
                "the passes add up to the blur: frost {frost}"
            );
        }
    }
    assert!(
        !frost_plan(0.2).face_applied,
        "a blur under a third of a texel is none"
    );
}

fn query(pane: Rect, frost: f32) -> SampleQuery {
    SampleQuery {
        pane,
        box_position: [100.0, 20.0],
        source_size: [2560.0, 1600.0],
        frost,
        reach: frost_reach(frost),
    }
}

#[test]
fn the_sample_is_snapped_outward_to_the_grid_of_the_reduction() {
    let rect = sample_rect(&query(Rect::new(13.0, 7.0, 101.0, 33.0), 9.6));
    // reach 24 + 4: x from 100 + 13 - 28 = 85 down to 84, to 100 + 13 + 101 + 28 = 242 up to 244
    assert_eq!((rect.x, rect.w), (84.0, 160.0));
    assert_eq!(
        (rect.y, rect.h),
        (0.0, 88.0 + 0.0),
        "and y from -1 clipped to 0, to 20 + 7 + 33 + 28 = 88"
    );
    for value in [rect.x, rect.y, rect.w, rect.h] {
        assert!(near(value % 4.0, 0.0), "{rect:?}");
    }
}

#[test]
fn the_sample_is_clipped_to_the_source_and_empty_beyond_it() {
    let q = SampleQuery {
        box_position: [2400.0, 1500.0],
        ..query(Rect::new(0.0, 0.0, 400.0, 300.0), 9.6)
    };
    let rect = sample_rect(&q);
    assert!(
        rect.x + rect.w <= 2560.0 && rect.y + rect.h <= 1600.0,
        "{rect:?}"
    );
    let away = SampleQuery {
        box_position: [5000.0, 0.0],
        ..query(Rect::new(0.0, 0.0, 10.0, 10.0), 9.6)
    };
    assert_eq!(sample_rect(&away), Rect::default());
}

#[test]
fn a_sample_always_holds_the_pane_it_is_for() {
    for frost in [1.0_f32, 4.0, 9.6, 20.0] {
        for (x, w) in [(0.0, 50.0), (13.7, 99.3), (1000.0, 640.0)] {
            let pane = Rect::new(x, 5.0, w, 40.0);
            let rect = sample_rect(&query(pane, frost));
            let on_screen = Rect::new(100.0 + pane.x, 20.0 + pane.y, pane.w, pane.h);
            assert_eq!(
                on_screen.intersection(&rect),
                on_screen,
                "frost {frost}: {rect:?} holds {on_screen:?}"
            );
        }
    }
}

#[test]
fn the_reaches_make_room_for_the_frost_and_the_shadow() {
    let reaches = reaches(9.6, 77, 32.0);
    assert_eq!(
        (
            reaches.frost,
            reaches.shadow,
            reaches.damage,
            reaches.padding
        ),
        (24, 77, 77, 77)
    );
    assert!(near(reaches.repaint, 77.0 + 32.0 + 2.0));
    let no_shadow = super::reaches(9.6, 0, 32.0);
    assert_eq!((no_shadow.damage, no_shadow.padding), (40, 32));
    assert_eq!(super::reaches(0.0, 0, 32.0).padding, 8);
}

#[test]
fn the_glass_shader_is_given_the_constants_worked_out_by_hand() {
    let constants = draw_constants(&scene_params(), 1.25_f64.sqrt());
    assert!((f64::from(constants.rim_coverage) - 0.632_129_994_770_862).abs() < 1e-6);
    assert!((f64::from(constants.outer_coverage) - 0.444_083_098_674_026_5).abs() < 1e-6);
    assert!(
        (constants.solid_coverage - 0.75).abs() < 1e-7
            && constants.interior_coverage > constants.solid_coverage
    );
    // 2 B / (refraction sqrt(n^2 - 1)) with B = 32
    assert!((f64::from(constants.thickness_per_fit) - 64.0 / 1.25_f64.sqrt()).abs() < 1e-4);
    assert!(!constants.dispersive);
    assert!((f64::from(constants.deviation_scale) - 1.25_f64.sqrt()).abs() < 1e-6);
}

#[test]
fn the_light_direction_is_a_unit_vector_clockwise_from_the_top() {
    let mut params = scene_params();
    for (angle, expected) in [
        (0.0_f32, [0.0, -1.0]),
        (90.0, [1.0, 0.0]),
        (180.0, [0.0, 1.0]),
        (
            315.0,
            [
                -std::f32::consts::FRAC_1_SQRT_2,
                -std::f32::consts::FRAC_1_SQRT_2,
            ],
        ),
    ] {
        params.light_angle = angle;
        let [x, y] = draw_constants(&params, 1.0).light_direction;
        assert!(
            (x - expected[0]).abs() < 1e-5 && (y - expected[1]).abs() < 1e-5,
            "angle {angle}: {x} {y}"
        );
        assert!((x.hypot(y) - 1.0).abs() < 1e-6);
    }
}

#[test]
fn the_opacity_of_the_glass_and_of_its_shadow_follow_the_fade() {
    let mut params = scene_params();
    params.glass_opacity = 0.8;
    let (glass, shadow) = opacities(&params, 0.5);
    assert!((glass - 0.4).abs() < 1e-7 && (shadow - 0.12 * 0.4).abs() < 1e-7);
    assert!((opacities(&params, 7.0).0 - 1.0).abs() < 1e-7, "held to 1");
}
