use super::*;

/// `OFFSETS27` of `params27.generated.ts`, in bytes: where some fields must
/// land, with the regular preset's value there.
const LANDMARKS: [(usize, f32); 9] = [
    (16, 4.0),    // inner_refraction_amount
    (68, 0.004),  // shadow_offset.y
    (80, 0.3545), // face_cm0.x
    (176, 0.2),   // shadow_cm0.x
    (280, 0.92),  // bleed_darken.x
    (324, 30.0),  // ring_shadow_radius
    (336, 0.35),  // key_fill_highlight_dir.x
    (380, 3.0),   // aberration_amount
    (384, 1.0),   // aberration_dir.x
];

#[test]
fn the_block_lands_where_the_shader_reads_it() {
    let block = Block::of(Kind::Regular).std140();
    let at = |bytes: usize| block.get(bytes / 4).copied();
    for (bytes, expected) in LANDMARKS {
        assert_eq!(at(bytes), Some(expected), "byte {bytes}");
    }
    // the last of Apple's fields ends at byte 392; the bright layer's switch is
    // the engine's own, and the rest of the row is empty
    assert_eq!(at(388), Some(0.0));
    assert_eq!(at(392), Some(0.0));
    assert_eq!(
        Block::of(Kind::Macos27).std140().get(98).copied(),
        Some(1.0)
    );
    assert_eq!(at(396), Some(0.0));
}

#[test]
fn the_presets_differ_only_where_the_transcription_says() {
    let regular = Block::of(Kind::Regular);
    let clear = Block::of(Kind::Clear);
    let late = Block::of(Kind::Macos27);
    assert!((clear.blur_radius - 63.0).abs() < f32::EPSILON);
    let recipe = Block::of(Kind::Recipe);
    assert!((recipe.blur_radius - 45.0).abs() < f32::EPSILON);
    assert_eq!(recipe.refraction_threshold, regular.refraction_threshold);
    assert_eq!(clear.refraction_threshold, regular.refraction_threshold);
    assert!(
        late.key_fill_highlight_color_bias == 0.0,
        "no dark rim: the edge is the bright layer's"
    );
    assert!(late.bright_layer > 0.0 && regular.bright_layer == 0.0);
    assert_eq!(
        late.face_cm, recipe.face_cm,
        "macOS 27's light over Apple's recipe"
    );
    assert!((late.blur_fill_darken_opacity - 0.675).abs() < f32::EPSILON);
}

#[test]
fn a_kind_goes_across_the_boundary_and_back() {
    for kind in [
        Kind::Regular,
        Kind::Clear,
        Kind::Macos27,
        Kind::Recipe,
        Kind::Lens,
        Kind::Tuned,
    ] {
        assert_eq!(Kind::from_code(kind.code()), Some(kind));
    }
    assert_eq!(Kind::from_code(0), None);
    assert_eq!(Kind::by_option(OPTION_CLEAR), Some(Kind::Clear));
    assert_eq!(
        Kind::by_option(2),
        None,
        "the current material's dark appearance is not one of these"
    );
}

#[test]
fn the_reach_covers_the_widest_read() {
    let block = Block::of(Kind::Regular);
    assert!(reach(&block) >= 24.0 + 48.0);
}

#[test]
fn the_lens_is_a_real_bezel_a_light_blur_and_a_dark_tint() {
    let lens = Block::of(Kind::Lens);
    let recipe = Block::of(Kind::Recipe);
    assert!(lens.blur_radius < recipe.blur_radius / 3.0, "not frosted");
    assert!(
        lens.inner_refraction_amount >= 16.0,
        "a bezel that visibly bends what is behind"
    );
    assert!(
        lens.refraction_opacity == 0.0
            && lens.edge_bleed_opacity == 0.0
            && lens.blur_fill_darken_opacity == 0.0
    );
    assert!(lens.tint_amount > 0.0 && lens.tint_adaptive > 0.0 && lens.hairline > 0.0);
    assert!(lens.tint.iter().all(|part| *part < 0.1), "the tint is dark");
    let floats = lens.std140();
    assert_eq!(floats.get(99).copied(), Some(lens.hairline));
    assert_eq!(floats.get(103).copied(), Some(lens.tint_amount));
    assert_eq!(floats.get(104).copied(), Some(lens.tint_adaptive));
    assert_eq!(floats.get(105).copied(), Some(lens.saturation));
    assert!(
        lens.saturation > 1.0 && lens.rim_glow > 0.0 && lens.rim_glow_depth > 0.0,
        "colour and light on the glass"
    );
    assert!(
        (Block::of(Kind::Regular).saturation - 1.0).abs() < f32::EPSILON,
        "Apple's own kinds keep their colour"
    );
    assert_eq!(
        Kind::by_option(crate::quartz::OPTION_LENS),
        Some(Kind::Lens)
    );
}

#[test]
fn the_older_kinds_keep_the_light_they_had() {
    for kind in [Kind::Regular, Kind::Macos27, Kind::Lens] {
        let floats = Block::of(kind).std140();
        assert_eq!(
            floats.get(108..110),
            Some(&[0.0, -1.0][..]),
            "the key light from above"
        );
        let h = -core::f32::consts::FRAC_1_SQRT_2;
        assert_eq!(
            floats.get(110..112),
            Some(&[h, h][..]),
            "the hairline lit from the top left"
        );
        assert_eq!(
            floats.get(112..116),
            Some(&[0.5, 0.0, 0.0, 0.0][..]),
            "half a side light, no inner light"
        );
    }
}

#[test]
fn a_tuning_lands_in_its_block() {
    use crate::tuning::{Id, Tuning};
    let mut tuning = Tuning::DEFAULT;
    tuning.set(Id::Frost, 14.0);
    tuning.set(Id::Lens, 16.0);
    tuning.set(Id::Bend, 0.5);
    tuning.set(Id::DarkEdge, 1.0);
    tuning.set(Id::LightAngle, -90.0);
    tuning.set(Id::Lift, 0.3);
    let block = Block::tuned(&tuning);
    assert!((block.blur_radius - 14.0).abs() < f32::EPSILON);
    assert!((block.inner_refraction_amount - 8.0).abs() < 1e-6);
    assert!((block.inner_refraction_inv_height - 1.0 / 16.0).abs() < 1e-6);
    assert!((block.key_fill_highlight_color_bias - 2.0 * DARK_EDGE_BIAS).abs() < 1e-6);
    assert!(
        (block.light_dir[0] + 1.0).abs() < 1e-6 && block.light_dir[1].abs() < 1e-6,
        "from the left"
    );
    let floats = block.std140();
    assert_eq!(floats.get(113).copied(), Some(0.3));
    assert_eq!(floats.get(4).copied(), Some(8.0));
}

#[test]
fn the_tuned_glass_as_it_comes_is_the_authors_pick() {
    let block = Block::of(Kind::Tuned);
    assert!(
        (block.light_dir[0]).abs() < 1e-6 && (block.light_dir[1] + 1.0).abs() < 1e-6,
        "lit from straight above"
    );
    assert!(block.key_fill_highlight_color_bias < 0.0, "a darkened edge");
    assert!(
        block.shadow_opacity == 0.0 && block.ring_shadow_opacity == 0.0,
        "no shadow"
    );
    assert!(block.lift == 0.0, "no white veil");
    assert_eq!(block.face_cm, IDENTITY);
    assert!(
        reach(&block) < 40.0,
        "no read wider than its own knobs: {}",
        reach(&block)
    );
}

#[test]
fn a_tuning_without_a_lens_bends_nothing_and_has_no_fringe() {
    use crate::tuning::{Id, Tuning};
    let mut tuning = Tuning::DEFAULT;
    tuning.set(Id::Lens, 0.0);
    let block = Block::tuned(&tuning);
    assert!(block.inner_refraction_amount == 0.0 && block.inner_refraction_inv_height == 0.0);
    assert!(block.aberration_amount.abs() < f32::EPSILON);
}

#[test]
fn apples_colour_blends_toward_the_recipe() {
    use crate::tuning::{Id, Tuning};
    let mut tuning = Tuning::DEFAULT;
    tuning.set(Id::AppleColour, 1.0);
    assert_eq!(
        Block::tuned(&tuning).face_cm,
        Block::of(Kind::Recipe).face_cm
    );
    tuning.set(Id::AppleColour, 0.5);
    let half = Block::tuned(&tuning).face_cm;
    assert!((half[0][0] - f32::midpoint(1.0, 0.921)).abs() < 1e-6);
    assert!((half[0][3] - 0.0813 / 2.0).abs() < 1e-6);
}
