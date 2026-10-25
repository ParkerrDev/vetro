use super::*;

fn near(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

/// Points on the outline of a rectangle with this corner radius, walked all the way round.
fn outline(shape: &ContinuousRect, radius: f64, steps: u32) -> Vec<[f64; 2]> {
    let [hx, hy] = shape.half_size();
    let zone = shape.zone(radius);
    let e = shape.exponent(radius);
    let mut points = Vec::new();
    for corner in 0..4 {
        let (sx, sy) = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)][corner];
        for i in 0..=steps {
            let t = f64::from(i) / f64::from(steps) * std::f64::consts::FRAC_PI_2;
            let m = [zone * t.cos().powf(2.0 / e), zone * t.sin().powf(2.0 / e)];
            points.push([sx * (hx - zone + m[0]), sy * (hy - zone + m[1])]);
        }
    }
    // the straight parts of the four edges
    for i in 0..=steps {
        let f = f64::from(i) / f64::from(steps);
        let along_x = (hx - zone) * (2.0 * f - 1.0);
        let along_y = (hy - zone) * (2.0 * f - 1.0);
        points.extend([[along_x, hy], [along_x, -hy], [hx, along_y], [-hx, along_y]]);
    }
    points
}

#[test]
fn the_exponent_is_a_circle_when_the_corner_fills_its_zone_and_about_3_27_when_it_reaches_further()
{
    let wide = ContinuousRect::new(400.0, 300.0, 40.0);
    assert!(near(wide.zone(40.0), 1.528 * 40.0, 1e-12));
    assert!(
        near(wide.exponent(40.0), 3.25, 0.05),
        "{}",
        wide.exponent(40.0)
    );
    let capsule = ContinuousRect::capsule(200.0, 60.0);
    assert!(
        near(capsule.zone(30.0), 30.0, 1e-12) && near(capsule.exponent(30.0), 2.0, 1e-6),
        "a capsule's ends are semicircles"
    );
    assert!(
        near(wide.exponent(0.0), 2.0, 0.0),
        "no radius, no corner to shape"
    );
}

#[test]
fn the_distance_is_zero_on_the_outline() {
    for (w, h, r) in [
        (400.0, 300.0, 40.0),
        (120.0, 120.0, 30.0),
        (600.0, 40.0, 20.0),
        (50.0, 50.0, 0.0),
        (300.0, 100.0, 80.0),
    ] {
        let shape = ContinuousRect::new(w, h, r);
        for p in outline(&shape, r, 64) {
            assert!(
                near(shape.distance(p), 0.0, 1e-8),
                "{w}x{h} r {r}: {} at {p:?}",
                shape.distance(p)
            );
        }
    }
}

#[test]
fn it_is_negative_inside_and_positive_outside_and_grows_along_straight_edges() {
    let shape = ContinuousRect::new(400.0, 300.0, 40.0);
    assert!(
        shape.distance([0.0, 0.0]) < 0.0 && near(shape.distance([0.0, 0.0]), -150.0, 1e-9),
        "the centre is half the short side inside"
    );
    assert!(
        near(shape.distance([230.0, 0.0]), 30.0, 1e-9)
            && near(shape.distance([0.0, -180.0]), 30.0, 1e-9),
        "straight out of an edge"
    );
    assert!(
        shape.distance([199.0, 149.0]) > 0.0,
        "the corner is cut: a point at the corner of the box is outside"
    );
    assert!(shape.distance([-200.0, -150.0]) > shape.distance([-200.0, 0.0]));
}

#[test]
fn the_normal_has_unit_length_everywhere() {
    for (w, h, r) in [
        (400.0, 300.0, 40.0),
        (120.0, 120.0, 60.0),
        (600.0, 40.0, 20.0),
    ] {
        let shape = ContinuousRect::new(w, h, r);
        let mut x = -w;
        while x < w {
            let mut y = -h;
            while y < h {
                let n = shape.normal([x, y]);
                assert!(
                    near(n[0].hypot(n[1]), 1.0, 1e-12),
                    "{w}x{h}: {n:?} at {x} {y}"
                );
                y += 17.3;
            }
            x += 19.1;
        }
    }
}

#[test]
fn the_normal_points_out_and_is_the_gradient_of_the_distance() {
    let shape = ContinuousRect::new(400.0, 300.0, 40.0);
    let step = 1e-5;
    for p in outline(&shape, 40.0, 24) {
        let n = shape.normal(p);
        let gradient = [
            (shape.distance([p[0] + step, p[1]]) - shape.distance([p[0] - step, p[1]]))
                / (2.0 * step),
            (shape.distance([p[0], p[1] + step]) - shape.distance([p[0], p[1] - step]))
                / (2.0 * step),
        ];
        // the corner's distance is a p-norm, not a true distance: its gradient is the direction of the normal, not unit
        let length = gradient[0].hypot(gradient[1]);
        let gradient = [gradient[0] / length, gradient[1] / length];
        let mismatch = (n[0] - gradient[0]).hypot(n[1] - gradient[1]);
        assert!(mismatch < 1e-3, "at {p:?}: {n:?} against {gradient:?}");
        assert!(n[0] * p[0] + n[1] * p[1] > 0.0, "outward at {p:?}");
    }
    let down = shape.normal([100.0, 149.0]);
    assert!(
        near(down[0], 0.0, 1e-12) && near(down[1], 1.0, 1e-12),
        "the bottom edge faces down"
    );
    let left = shape.normal([-199.0, 0.0]);
    assert!(
        near(left[0], -1.0, 1e-12) && near(left[1], 0.0, 1e-12),
        "the left edge faces left"
    );
}

#[test]
fn each_corner_can_have_its_own_radius() {
    let shape = ContinuousRect::with_radii(300.0, 200.0, [10.0, 50.0, 0.0, 30.0]);
    // a point on the outline near each corner is on it, whatever the others are
    for (corner, sign, radius) in [
        (0, (-1.0, -1.0), 10.0),
        (1, (1.0, -1.0), 50.0),
        (3, (-1.0, 1.0), 30.0),
    ] {
        let zone = (1.528_f64 * radius).min(100.0);
        let e = if radius > 0.0 {
            -1.0 / (1.0 - 0.292_893_22 * radius / zone).log2()
        } else {
            2.0
        };
        let t: f64 = 0.6;
        let m = [zone * t.cos().powf(2.0 / e), zone * t.sin().powf(2.0 / e)];
        let p = [
            sign.0 * (150.0 - zone + m[0]),
            sign.1 * (100.0 - zone + m[1]),
        ];
        assert!(
            near(shape.distance(p), 0.0, 1e-8),
            "corner {corner}: {}",
            shape.distance(p)
        );
    }
    assert!(
        near(shape.distance([150.0, 100.0]), 0.0, 1e-9),
        "a square corner is the corner of the box"
    );
}

#[test]
fn a_shape_of_no_size_does_not_break() {
    let nothing = ContinuousRect::new(0.0, 0.0, 10.0);
    assert!(nothing.distance([0.0, 0.0]).is_finite() && nothing.distance([3.0, 4.0]).is_finite());
    let n = nothing.normal([0.0, 0.0]);
    assert!(near(n[0].hypot(n[1]), 1.0, 1e-12));
    assert!(
        ContinuousRect::new(f64::NAN, -5.0, f64::INFINITY)
            .distance([1.0, 1.0])
            .is_finite()
    );
}

#[test]
fn the_smooth_minimum_is_the_minimum_far_apart_and_dips_below_it_close() {
    assert!(
        near(smooth_min(3.0, 10.0, 4.0), 3.0, 0.0),
        "further apart than k: the plain minimum"
    );
    assert!(
        near(smooth_min(0.0, 0.0, 4.0), -1.0, 1e-12),
        "equal distances: k / 4 below"
    );
    assert!(near(smooth_min(1.0, 2.0, 0.0), 1.0, 0.0));
    for i in 0..100 {
        let b = f64::from(i) * 0.1;
        let joined = smooth_min(0.0, b, 5.0);
        assert!(
            (-5.0 / 4.0..=0.0).contains(&joined),
            "never above the minimum, never more than k / 4 below"
        );
    }
    assert!(
        near(smooth_min(2.0, 3.0, 4.0), smooth_min(3.0, 2.0, 4.0), 1e-12),
        "symmetric"
    );
}

#[test]
fn the_weights_are_the_derivative_of_the_smooth_minimum() {
    let k = 10.0;
    let step = 1e-6;
    for (a, b) in [
        (0.0, 3.0),
        (3.0, 0.0),
        (1.0, 1.5),
        (-2.0, 5.0),
        (4.0, 4.0001),
    ] {
        let weight = smooth_union(a, b, k).weight_a;
        let slope = (smooth_min(a + step, b, k) - smooth_min(a - step, b, k)) / (2.0 * step);
        assert!(
            near(weight, slope, 1e-4),
            "({a}, {b}): {weight} against {slope}"
        );
        assert!(
            near(weight + smooth_union(b, a, k).weight_a, 1.0, 1e-12),
            "the two weights add to 1"
        );
    }
}

#[test]
fn blended_normals_stay_unit_and_agree_at_the_ends() {
    let (a, b) = ([1.0, 0.0], [0.0, 1.0]);
    for w in [0.0, 0.25, 0.5, 1.0] {
        let n = blend_normals(a, b, w);
        assert!(near(n[0].hypot(n[1]), 1.0, 1e-12));
    }
    let all_a = blend_normals(a, b, 1.0);
    assert!(near(all_a[0], 1.0, 1e-12) && near(all_a[1], 0.0, 1e-12));
    let fallback = blend_normals(a, [-1.0, 0.0], 0.5);
    assert!(
        near(fallback[0], 1.0, 1e-12) && near(fallback[1], 0.0, 1e-12),
        "normals that cancel fall back to the first"
    );
}

#[test]
fn the_neck_closes_as_the_drop_leaves() {
    assert!(near(neck_width(0.0, NECK), 18.0, 0.0) && near(neck_width(0.3, NECK), 18.0, 0.0));
    assert!(near(neck_width(0.95, NECK), 0.0, 0.0) && near(neck_width(1.0, NECK), 0.0, 0.0));
    let mid = neck_width(0.625, NECK);
    assert!(
        near(mid, 9.0, 1e-9),
        "halfway through the smoothstep: {mid}"
    );
    let mut previous = 18.0;
    for i in 0..=100 {
        let w = neck_width(f64::from(i) / 100.0, NECK);
        assert!(w <= previous + 1e-12);
        previous = w;
    }
}

#[test]
fn the_angle_gate_keeps_nested_outlines_hard_and_lets_facing_ones_join() {
    assert!(near(normal_angle_gate(0.0), 0.0, 0.0) && near(normal_angle_gate(1.0), 1.0, 1e-12));
    let mut previous = 0.0;
    for i in 0..=100 {
        let g = normal_angle_gate(f64::from(i) / 100.0);
        assert!(g >= previous - 1e-12, "monotone");
        previous = g;
    }
    assert!(
        near(gated_blend(20.0, [1.0, 0.0], [1.0, 0.0]), 0.0, 1e-12),
        "agreeing normals: no neck"
    );
    assert!(
        near(gated_blend(20.0, [1.0, 0.0], [-1.0, 0.0]), 20.0, 1e-9),
        "facing normals: the whole neck"
    );
}

#[test]
fn two_capsules_that_approach_join_with_a_neck_and_nested_ones_do_not() {
    let a = ContinuousRect::capsule(100.0, 40.0);
    let b = ContinuousRect::capsule(100.0, 40.0);
    // b centred 124 to the right: the gap between them is 24, p in the middle of it
    let gap_point = [62.0, 0.0];
    let (da, db) = (
        a.distance(gap_point),
        b.distance([gap_point[0] - 124.0, 0.0]),
    );
    let facing = union_of(
        &[
            Sample {
                distance: da,
                normal: a.normal(gap_point),
            },
            Sample {
                distance: db,
                normal: b.normal([gap_point[0] - 124.0, 0.0]),
            },
        ],
        28.0,
    )
    .unwrap();
    assert!(
        facing.distance < da.min(db) - 0.1,
        "the neck pulls the outline in: {} against {}",
        facing.distance,
        da.min(db)
    );
    // the same two shapes sitting inside each other along one edge: normals agree, the union stays hard
    let sample = Sample {
        distance: 3.0,
        normal: [0.0, 1.0],
    };
    let nested = union_of(
        &[
            sample,
            Sample {
                distance: 3.5,
                normal: [0.0, 1.0],
            },
        ],
        28.0,
    )
    .unwrap();
    assert!(near(nested.distance, 3.0, 1e-12));
    assert!(union_of(&[], 28.0).is_none());
    assert_eq!(union_of(&[sample], 28.0), Some(sample));
}

#[test]
fn a_union_joins_four_shapes_and_ignores_the_fifth() {
    let samples = [Sample {
        distance: 1.0,
        normal: [1.0, 0.0],
    }; 5];
    let four = union_of(&samples[..4], 10.0).unwrap();
    let five = union_of(&samples, 10.0).unwrap();
    assert_eq!(four, five);
}
