//! Declared shapes: panes the plugin or the owner's own clients define, as a
//! rectangle with continuous corners, evaluated analytically. No texture and no
//! rebuild when a pane moves, stretches or morphs. Not wired into the plugin
//! yet; this is also the CPU reference for the shader that later evaluates it.
//!
//! Coordinates are the shape's own, in pixels or points, with the origin at its
//! centre and y down. A distance is negative inside the outline.
//!
//! The continuous corner is the report's C.1: `liquid_glass_widgets`
//! `sdf.glsl:103-135` (MIT, Copyright (c) 2024-2026 Sebastian Degenaar) and
//! liquid-dom `shaders.ts:230-243` (MIT, Copyright (c) 2026 Andras Prifer),
//! which follow Flutter's rounded superellipse. The smooth union is Glass HQ's
//! `core/union.ts:21-25` (MIT, Copyright (c) 2026 Glass HQ) and widgets
//! `sdf.glsl:174-178`; the weights of its normal are derived here from its
//! derivative, and the angle gate is liquid-dom's `normalAngleGate`. See NOTICE.

use crate::motion::smoothstep;

/// How far along each edge a continuous corner reaches, in radii (the corner
/// of Apple's continuous rectangles, as Flutter approximates it; not checked
/// against a capture).
pub const CORNER_REACH: f64 = 1.528;
/// `1 - 1 / sqrt(2)`: what makes the exponent 2, a circle, when the corner fills its zone.
const CIRCLE_SHARE: f64 = 0.292_893_22;
/// The neck of a drop that pinches off from its source, in points
/// (Glass HQ `core/morph-path.ts:108`).
pub const NECK: f64 = 18.0;

/// A rectangle with continuous corners, up to four different radii.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinuousRect {
    half: [f64; 2],
    /// Radii: top left, top right, bottom right, bottom left (y down).
    radii: [f64; 4],
}

impl ContinuousRect {
    /// A rectangle of this size with the same radius at every corner.
    #[must_use]
    pub fn new(width: f64, height: f64, radius: f64) -> Self {
        Self::with_radii(width, height, [radius; 4])
    }

    /// A rectangle with a radius for each corner: top left, top right, bottom right, bottom left.
    #[must_use]
    pub fn with_radii(width: f64, height: f64, radii: [f64; 4]) -> Self {
        let clean = |x: f64| if x.is_finite() { x.max(0.0) } else { 0.0 };
        Self {
            half: [0.5 * clean(width), 0.5 * clean(height)],
            radii: radii.map(clean),
        }
    }

    /// A capsule: the radius is half the short side, so the ends are semicircles.
    #[must_use]
    pub fn capsule(width: f64, height: f64) -> Self {
        Self::new(width, height, 0.5 * width.min(height))
    }

    /// Half the width and half the height.
    #[must_use]
    pub fn half_size(&self) -> [f64; 2] {
        self.half
    }

    /// The radius of the corner `p` is nearest to.
    fn radius_at(&self, p: [f64; 2]) -> f64 {
        let index = match (p[0] < 0.0, p[1] < 0.0) {
            (true, true) => 0,
            (false, true) => 1,
            (false, false) => 2,
            (true, false) => 3,
        };
        self.radii.get(index).copied().unwrap_or(0.0)
    }

    /// The zone of a corner of this radius: how far it reaches along each
    /// edge, at most to the middle of the short side.
    #[must_use]
    pub fn zone(&self, radius: f64) -> f64 {
        (CORNER_REACH * radius).min(self.half[0].min(self.half[1]))
    }

    /// The Lamé exponent of a corner of this radius: about 3.27 where the
    /// corner reaches [`CORNER_REACH`] radii, 2 (a circle) where it fills its zone.
    #[must_use]
    pub fn exponent(&self, radius: f64) -> f64 {
        corner_exponent(radius, self.zone(radius))
    }

    /// The signed distance from the outline: negative inside, zero on it.
    #[must_use]
    pub fn distance(&self, p: [f64; 2]) -> f64 {
        let radius = self.radius_at(p);
        let zone = self.zone(radius);
        let exponent = corner_exponent(radius, zone);
        let q = [
            p[0].abs() - self.half[0] + zone,
            p[1].abs() - self.half[1] + zone,
        ];
        let m = [q[0].max(0.0), q[1].max(0.0)];
        let biggest = m[0].max(m[1]);
        let corner = if biggest > 0.0 {
            biggest
                * ((m[0] / biggest).powf(exponent) + (m[1] / biggest).powf(exponent))
                    .powf(1.0 / exponent)
        } else {
            0.0
        };
        corner + q[0].max(q[1]).min(0.0) - zone
    }

    /// The outward unit normal at `p`: the gradient of the distance, which in
    /// a corner is that of the p-norm, `m^(e - 1)` componentwise, and along an
    /// edge is the axis.
    #[must_use]
    pub fn normal(&self, p: [f64; 2]) -> [f64; 2] {
        let radius = self.radius_at(p);
        let zone = self.zone(radius);
        let exponent = corner_exponent(radius, zone);
        let q = [
            p[0].abs() - self.half[0] + zone,
            p[1].abs() - self.half[1] + zone,
        ];
        let m = [q[0].max(0.0), q[1].max(0.0)];
        let gradient = if m[0] > 0.0 || m[1] > 0.0 {
            [m[0].powf(exponent - 1.0), m[1].powf(exponent - 1.0)]
        } else if q[0] > q[1] {
            [1.0, 0.0]
        } else {
            [0.0, 1.0]
        };
        let length = gradient[0].hypot(gradient[1]);
        if length <= 0.0 || !length.is_finite() {
            return [0.0, 1.0];
        }
        let sign = |x: f64| if x < 0.0 { -1.0 } else { 1.0 };
        [
            sign(p[0]) * gradient[0] / length,
            sign(p[1]) * gradient[1] / length,
        ]
    }
}

/// `e = -1 / log2(1 - 0.29289322 r / zone)`.
fn corner_exponent(radius: f64, zone: f64) -> f64 {
    if radius <= 0.0 || zone <= 0.0 {
        return 2.0;
    }
    let inside = 1.0 - CIRCLE_SHARE * radius / zone;
    if inside <= 0.0 {
        2.0
    } else {
        (-1.0 / inside.log2()).max(2.0)
    }
}

/// The smooth minimum of two distances: `min(a, b) - h^2 k / 4`, with
/// `h = max(k - |a - b|, 0) / k`. For `k` of 0 it is the plain minimum.
#[must_use]
pub fn smooth_min(a: f64, b: f64, k: f64) -> f64 {
    smooth_union(a, b, k).distance
}

/// What a smooth union gives at a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blend {
    /// The distance to the joined shape.
    pub distance: f64,
    /// How much of the first shape's normal is in the joined one's; the
    /// second's is `1 - weight_a`. The weights are the derivative of the
    /// smooth minimum: `1 - h / 2` for the nearer shape and `h / 2` for the other.
    pub weight_a: f64,
}

/// Joins two distances with blend distance `k` (the width of the neck).
#[must_use]
pub fn smooth_union(a: f64, b: f64, k: f64) -> Blend {
    if !(k.is_finite() && k > 0.0) {
        return Blend {
            distance: a.min(b),
            weight_a: if a <= b { 1.0 } else { 0.0 },
        };
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    let weight_a = if a < b { 1.0 - 0.5 * h } else { 0.5 * h };
    Blend {
        distance: a.min(b) - h * h * k / 4.0,
        weight_a,
    }
}

/// The joined shape's outward normal: `normalize(w_a n_a + w_b n_b)`.
#[must_use]
pub fn blend_normals(normal_a: [f64; 2], normal_b: [f64; 2], weight_a: f64) -> [f64; 2] {
    let w = weight_a.clamp(0.0, 1.0);
    let n = [
        w * normal_a[0] + (1.0 - w) * normal_b[0],
        w * normal_a[1] + (1.0 - w) * normal_b[1],
    ];
    let length = n[0].hypot(n[1]);
    if length > 1e-12 {
        [n[0] / length, n[1] / length]
    } else {
        normal_a
    }
}

/// The width of a neck while a drop pinches off from its source, for
/// progress 0 to 1: `neck (1 - smoothstep(0.3, 0.95, progress))`
/// (Glass HQ `core/morph-path.ts:108`, `neck` 18 by default).
#[must_use]
pub fn neck_width(progress: f64, neck: f64) -> f64 {
    neck * (1.0 - smoothstep(0.3, 0.95, progress))
}

/// liquid-dom's `normalAngleGate`: `x + x^2 - x^3` for the angle between two
/// normals as a share of a half turn. 0 for normals that agree (nested or
/// overlapping outlines: a hard union), 1 for normals that face each other
/// (a neck can form).
#[must_use]
pub fn normal_angle_gate(angle_share: f64) -> f64 {
    let x = angle_share.clamp(0.0, 1.0);
    x * x.mul_add(-x, x + 1.0)
}

/// The blend distance for two normals: `k` times the gate of their angle.
#[must_use]
pub fn gated_blend(k: f64, normal_a: [f64; 2], normal_b: [f64; 2]) -> f64 {
    let cosine = (normal_a[0] * normal_b[0] + normal_a[1] * normal_b[1]).clamp(-1.0, 1.0);
    k * normal_angle_gate(cosine.acos() / std::f64::consts::PI)
}

/// One shape of a union, placed: its distance and normal at a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub distance: f64,
    pub normal: [f64; 2],
}

/// The most shapes a union joins.
pub const MAX_UNION: usize = 4;

/// Joins up to [`MAX_UNION`] samples one after the other with blend distance
/// `k`, gating it by the angle between the normals; the shapes past the
/// fourth are ignored. None for no shapes.
#[must_use]
pub fn union_of(samples: &[Sample], k: f64) -> Option<Sample> {
    let mut joined = *samples.first()?;
    for next in samples.iter().take(MAX_UNION).skip(1) {
        let blend = smooth_union(
            joined.distance,
            next.distance,
            gated_blend(k, joined.normal, next.normal),
        );
        joined = Sample {
            distance: blend.distance,
            normal: blend_normals(joined.normal, next.normal, blend.weight_a),
        };
    }
    Some(joined)
}

#[cfg(test)]
mod tests;
