use super::*;

fn near(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

fn regular() -> Lens {
    Lens {
        bezel_width: 20.0,
        thickness: 53.0,
        refractive_index: 1.5,
        profile_exponent: 2.6,
        refraction: 1.0,
        dispersion: 0.0,
        paraxial: false,
    }
}

/// The lens of Apple's current material: 10.7 points thick, the rim as wide,
/// bending by the thin-prism law.
fn current() -> Lens {
    Lens {
        bezel_width: 10.7,
        thickness: 10.7,
        refractive_index: 1.24,
        paraxial: true,
        ..regular()
    }
}

/// A fixed pseudo-random sequence in `[0, 1)`: tests that sample the model at
/// scattered points must be the same run after run.
fn unit_sequence(count: usize) -> Vec<f64> {
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    (0..count)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            f64::from(u32::try_from(state >> 40).unwrap()) / f64::from(1_u32 << 24)
        })
        .collect()
}

#[test]
fn the_rim_rises_from_the_outline_to_the_flat() {
    assert!(near(profile_height(0.0, 4.0), 0.0, 1e-12));
    assert!(near(profile_height(1.0, 4.0), 1.0, 1e-12));
    assert!(
        near(profile_height(0.5, 2.0), 0.75_f64.sqrt(), 1e-12),
        "exponent 2 is a quarter circle"
    );
}

#[test]
fn the_surface_is_vertical_on_the_outline_and_level_at_the_inner_end() {
    assert!(near(profile_slope(1.0, 2.6), 0.0, 1e-9));
    assert!(profile_slope(1e-6, 2.6) > 100.0);
}

#[test]
fn the_slope_is_the_derivative_of_the_height() {
    let (t, step) = (0.3, 1e-6);
    let numeric = (profile_height(t + step, 2.6) - profile_height(t - step, 2.6)) / (2.0 * step);
    assert!(near(profile_slope(t, 2.6), numeric, 1e-5));
}

#[test]
fn snell_on_the_outline_leans_by_the_root_of_n_squared_minus_one() {
    assert!(near(deviation(0.0, 2.6, 1.5), 1.25_f64.sqrt(), 1e-12));
    assert!(
        near(deviation(1.0, 2.6, 1.5), 0.0, 1e-12),
        "no lean where the glass is flat"
    );
}

#[test]
fn the_lean_only_falls_from_the_outline_inward() {
    let falling = (1..=200).all(|i| {
        deviation(f64::from(i) / 200.0, 2.6, 1.5)
            <= deviation(f64::from(i - 1) / 200.0, 2.6, 1.5) + 1e-12
    });
    assert!(falling);
}

#[test]
fn a_higher_index_bends_more_and_air_bends_nothing() {
    assert!(deviation(0.2, 2.6, 1.6) > deviation(0.2, 2.6, 1.5));
    assert!(near(deviation(0.4, 2.6, 1.000_000_1), 0.0, 1e-5));
}

#[test]
fn the_regular_lens_follows_the_curve_measured_on_ios() {
    // 60 (1 - sqrt(1 - x^2)) with x from 1 on the outline to 0 at a depth of 20
    let lens = regular();
    let worst = (0..400)
        .map(|i| {
            let t = (f64::from(i) + 0.5) / 400.0;
            let measured = 60.0 * (1.0 - (1.0 - (1.0 - t) * (1.0 - t)).sqrt());
            (displacement(&lens, t, 1.5) - measured).abs()
        })
        .fold(0.0, f64::max);
    assert!(
        worst < 2.0,
        "largest distance from the measured curve: {worst}"
    );
    assert!(near(
        displacement(&lens, 0.0, 1.5),
        53.0 * 1.25_f64.sqrt(),
        1e-9
    ));
    assert!(
        near(max_displacement(&lens), 53.0 * 1.25_f64.sqrt(), 1e-9),
        "and that is the largest bend anywhere"
    );
}

#[test]
fn dispersion_spreads_the_index_around_green() {
    let lens = Lens {
        dispersion: 0.2,
        ..regular()
    };
    let indices = channel_indices(&lens);
    assert!(near(indices[2] - indices[0], 0.2 * 0.5, 1e-12) && near(indices[1], 1.5, 1e-12));
    let at = |n: f64| displacement(&lens, 0.1, n);
    assert!(
        at(indices[0]) < at(indices[1]) && at(indices[1]) < at(indices[2]),
        "red lands nearest, blue furthest"
    );
}

#[test]
fn a_pane_too_small_for_the_lens_gets_a_smaller_one() {
    let fitted = fitted_to(&regular(), 48.0);
    assert!(
        near(fitted.bezel_width, 12.0, 1e-12),
        "the rim is a quarter of the short side"
    );
    assert!(
        near(displacement(&fitted, 0.0, 1.5), 24.0, 1e-9),
        "and the bend reaches the centre line, no further"
    );
    let roomy = fitted_to(&regular(), 400.0);
    assert!(near(roomy.bezel_width, 20.0, 1e-12) && near(roomy.thickness, 53.0, 1e-12));
    assert_eq!(
        fitted_to(&regular(), 0.0),
        regular(),
        "a pane of no size leaves the lens alone"
    );
}

#[test]
fn displacement_is_never_negative_and_ends_at_zero() {
    let lens = regular();
    let unit = unit_sequence(3000);
    for chunk in unit.chunks(3) {
        let (t, n, p) = (chunk[0], 1.01 + 1.99 * chunk[1], 1.0 + 7.0 * chunk[2]);
        let lens = Lens {
            profile_exponent: p,
            ..lens
        };
        let d = displacement(&lens, t, n);
        assert!(d >= 0.0 && d.is_finite(), "t {t} n {n} p {p}: {d}");
        assert!(
            near(displacement(&lens, 1.0, n), 0.0, 0.0),
            "zero at the inner edge of the bezel"
        );
    }
}

#[test]
fn on_the_outline_the_bend_is_the_fit_times_the_root_of_n_squared_minus_one() {
    let unit = unit_sequence(2000);
    for chunk in unit.chunks(2) {
        let (short_side, n) = (4.0 + 400.0 * chunk[0], 1.05 + 1.9 * chunk[1]);
        let lens = Lens {
            refractive_index: n,
            ..regular()
        };
        let fitted = fitted_to(&lens, short_side);
        let root = (n * n - 1.0).sqrt();
        let bend = displacement(&fitted, 0.0, n);
        assert!(near(
            bend,
            fitted.refraction * fitted.thickness * root,
            1e-9 * bend.max(1.0)
        ));
        assert!(
            bend <= 0.5 * short_side + 1e-9,
            "never further than the centre line"
        );
    }
}

#[test]
fn displacement_grows_with_the_index() {
    let lens = regular();
    for i in 1..200 {
        let t = f64::from(i) / 200.0;
        let ladder: Vec<f64> = (0..40)
            .map(|k| displacement(&lens, t, 1.02 + 0.05 * f64::from(k)))
            .collect();
        assert!(
            ladder.windows(2).all(|w| w[1] >= w[0]),
            "monotone in n at t {t}"
        );
    }
}

#[test]
fn the_normal_quantile_inverts_the_normal_distribution() {
    assert!(near(normal_quantile(0.5), 0.0, 1e-9));
    assert!(near(normal_quantile(0.841_344_746_068_542_9), 1.0, 1e-7));
    assert!(near(normal_quantile(0.998_650_101_968_369_9), 3.0, 1e-6));
    assert!(
        near(normal_quantile(0.001_349_898_031_630_1), -3.0, 1e-6),
        "and is odd about one half"
    );
}

#[test]
fn coverage_one_half_is_the_outline() {
    assert!(near(distance_from_blurred_coverage(0.5, 10.0), 0.0, 1e-9));
    assert!(distance_from_blurred_coverage(0.3, 10.0) < 0.0);
    assert!(near(
        field_sigma(30.0) * FIELD_SIGMAS_PER_BEZEL,
        30.0,
        1e-12
    ));
    assert!(
        near(field_sigma(0.5), 1.0, 0.0),
        "the blur is never narrower than a pixel"
    );
}

#[test]
fn the_table_coordinate_and_its_inverse_agree() {
    for v in [0.5, 0.7, 0.95, 0.999] {
        assert!(near(
            coverage_at_lut_coordinate(lut_coordinate(v)),
            v,
            1e-12
        ));
    }
}

fn table_lens() -> Lens {
    Lens {
        bezel_width: 32.0,
        thickness: 85.0,
        dispersion: 0.1,
        ..Lens::default()
    }
}

#[test]
fn the_table_has_two_rows_of_rgba_texels() {
    let table = build_lens_table(&table_lens(), 1024).unwrap();
    assert_eq!(table.texels.len(), 1024 * 2 * 4);
    let blue = channel_indices(&table_lens())[2];
    assert!(near(
        table.deviation_scale,
        (blue * blue - 1.0).sqrt(),
        1e-12
    ));
    assert_eq!(
        build_lens_table(&table_lens(), 1).unwrap().size,
        2,
        "a row has at least two texels"
    );
    assert_eq!(
        build_lens_table(&table_lens(), MAX_TABLE_SIZE + 1),
        Err(LensError::TableTooLarge(MAX_TABLE_SIZE + 1))
    );
}

#[test]
fn row_zero_runs_from_deep_inside_to_the_outline() {
    let table = build_lens_table(&table_lens(), 1024).unwrap();
    assert!(table.texels[(1024 - 1) * 4] <= 1 && table.texels[0] == 255);
    // one deviation inside the outline is a third of the bezel
    let s = lut_coordinate(0.841_344_746_068_542_9);
    let at = crate::num::index_of(crate::num::trunc_i32(s * 1024.0));
    assert!((i32::from(table.texels[at * 4]) - 85).abs() <= 1);
}

#[test]
fn row_one_starts_on_the_outline_and_ends_where_the_glass_is_flat() {
    let table = build_lens_table(&table_lens(), 1024).unwrap();
    let row = 1024 * 4;
    assert!(
        table.texels[row + 2] == 255 && table.texels[row + 3] == 0,
        "blue's lean is the scale, the rim has no height yet"
    );
    let end = row + (1024 - 1) * 4;
    assert!(
        table.texels[end + 1] == 0 && table.texels[end + 3] == 255,
        "no lean, full height"
    );
    let mid = row + 100 * 4;
    assert!(
        table.texels[mid] < table.texels[mid + 1] && table.texels[mid + 1] < table.texels[mid + 2],
        "red, green, blue in rising order of bend"
    );
}

#[test]
fn the_lean_in_the_table_reads_back_as_the_lean_that_made_it() {
    let lens = table_lens();
    let table = build_lens_table(&lens, 1024).unwrap();
    let indices = channel_indices(&lens);
    let mut worst = 0.0_f64;
    for i in 0..=400 {
        let t = f64::from(i) / 400.0;
        for (channel, n) in indices.iter().enumerate() {
            let exact = deviation(t, lens.profile_exponent, *n);
            worst = worst.max((table.lean_at(t, channel) - exact).abs() / table.deviation_scale);
        }
        assert!(
            near(
                table.height_at(t),
                profile_height(t, lens.profile_exponent),
                0.01
            ),
            "height at {t}"
        );
    }
    assert!(
        worst < 0.012,
        "the table is within a byte and a half of the model: {worst}"
    );
}

#[test]
fn the_depth_in_the_table_reads_back_as_the_distance_that_made_it() {
    let table = build_lens_table(&table_lens(), 1024).unwrap();
    for i in 0..=100 {
        let v = 0.5 + 0.4985 * f64::from(i) / 100.0;
        let exact = (normal_quantile(v) / FIELD_SIGMAS_PER_BEZEL).clamp(0.0, 1.0);
        assert!(
            near(table.depth_at(v), exact, 0.012),
            "coverage {v}: {} against {exact}",
            table.depth_at(v)
        );
    }
}

#[test]
fn the_thin_prism_lands_twenty_points_in_on_the_outline_and_falls_as_a_square() {
    let lens = current();
    assert!(
        near(displacement(&lens, 0.0, 1.24), 20.544, 1e-9),
        "8 * 10.7 * 0.24"
    );
    assert!(near(displacement(&lens, 0.5, 1.24), 0.25 * 20.544, 1e-9));
    assert!(near(displacement(&lens, 1.0, 1.24), 0.0, 1e-12));
    assert!(near(max_displacement(&lens), 20.544, 1e-9));
    // where the bend changes faster than the place, the page shows mirrored: from the
    // outline to 0.74 of the rim, as measured
    let slope = |t: f64| {
        (displacement(&lens, t, 1.24) - displacement(&lens, t + 1e-6, 1.24))
            / (1e-6 * lens.bezel_width)
    };
    assert!(slope(0.73) > 1.0 && slope(0.75) < 1.0);
}

#[test]
fn a_small_pane_fits_the_thin_prism_too() {
    let fitted = fitted_to(&current(), 24.0);
    assert!(near(fitted.bezel_width, 6.0, 1e-12));
    assert!(
        near(displacement(&fitted, 0.0, 1.24), 12.0, 1e-9),
        "the bend reaches the centre line, no further"
    );
    let roomy = fitted_to(&current(), 200.0);
    assert!(near(roomy.thickness, 10.7, 1e-12));
}

#[test]
fn the_table_holds_the_thin_prism_for_the_shader_as_it_holds_snell() {
    let lens = current();
    let table = build_lens_table(&lens, 1024).unwrap();
    assert!(near(table.deviation_scale, 8.0 * 0.24, 1e-12));
    for t in [0.0, 0.1, 0.26, 0.5, 0.74, 0.9, 1.0] {
        // the shader: (B fit h + T) * scale * row1, with a height of 0
        let read = lens.thickness * table.lean_at(t, 1);
        assert!(
            near(read, displacement(&lens, t, 1.24), 0.05),
            "at {t}: {read} against {}",
            displacement(&lens, t, 1.24)
        );
        assert!(
            near(table.height_at(t), 0.0, 1e-12),
            "no height: the slab alone bends"
        );
    }
    // the depth row is the outline's, whatever the law
    let snell = build_lens_table(
        &Lens {
            paraxial: false,
            ..lens
        },
        1024,
    )
    .unwrap();
    assert!(near(table.depth_at(0.9), snell.depth_at(0.9), 1e-12));
}
