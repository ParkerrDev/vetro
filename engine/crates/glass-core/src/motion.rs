//! Motion: springs, stretch, rubber band and the numbers that go with them.
//! Not wired into the plugin yet: later milestones drive the sliding selector,
//! pressed thumbs and merging panes from here.
//!
//! Everything is closed form and costs a few microseconds a frame. A spring
//! is stepped by solving its equation over the step, so it is stable for any
//! time step, however long, and two steps of `a` and `b` end where one of
//! `a + b` does.
//!
//! The constants and laws are those of Glass HQ's `liquid-glass` (MIT,
//! Copyright (c) 2026 Glass HQ; see NOTICE), cited by file and line where each
//! is used. Kyant0's thumb constants (`AndroidLiquidGlass`, Apache-2.0,
//! Copyright 2025 Kyant; see NOTICE) are cited the same way.

use std::f64::consts::TAU;

pub mod genie;
pub mod wobble;

/// The shortest duration a spring takes: a spring of no duration would never move.
const MIN_DURATION: f64 = 1e-3;
/// Within this of critical damping a spring is solved as a critically damped one.
const CRITICAL_BAND: f64 = 1e-6;

/// A spring as people say it: how long one swing takes and how bouncy it is.
/// `bounce` is 0 for critical damping (no overshoot), up to 1 for no damping
/// at all, and negative for a spring that creeps (down to -1: damping ratio 2).
/// The damping ratio is `1 - bounce`. After `SwiftUI`, and Glass HQ's
/// `core/spring.ts:37-44`: `omega = 2 pi / duration`, stiffness `omega^2`,
/// damping `4 pi (1 - bounce) / duration`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringSpec {
    /// Seconds of one swing; at least a thousandth.
    pub duration: f64,
    /// Bounce, from -1 to 1.
    pub bounce: f64,
}

impl SpringSpec {
    /// A spring of this duration (seconds) and bounce, held to what they mean.
    #[must_use]
    pub fn new(duration: f64, bounce: f64) -> Self {
        Self {
            duration: if duration.is_finite() {
                duration.max(MIN_DURATION)
            } else {
                MIN_DURATION
            },
            bounce: if bounce.is_finite() {
                bounce.clamp(-1.0, 1.0)
            } else {
                0.0
            },
        }
    }

    /// Radians a second of the undamped swing.
    #[must_use]
    pub fn omega(&self) -> f64 {
        TAU / self.duration
    }

    /// The stiffness of a unit mass.
    #[must_use]
    pub fn stiffness(&self) -> f64 {
        self.omega() * self.omega()
    }

    /// The damping ratio: 1 is critical, less bounces, more creeps.
    #[must_use]
    pub fn damping_ratio(&self) -> f64 {
        1.0 - self.bounce
    }

    /// The damping coefficient of a unit mass: `4 pi (1 - bounce) / duration`.
    #[must_use]
    pub fn damping(&self) -> f64 {
        2.0 * self.damping_ratio() * self.omega()
    }
}

/// The named springs of Glass HQ (`core/spring.ts:9-22`): `(duration, bounce)`.
pub mod named {
    use super::SpringSpec;

    /// A press: quick, a little bounce.
    pub const PRESS: SpringSpec = SpringSpec {
        duration: 0.24,
        bounce: 0.20,
    };
    /// A release: slower, bouncier.
    pub const RELEASE: SpringSpec = SpringSpec {
        duration: 0.50,
        bounce: 0.42,
    };
    /// A glow following a pointer: no overshoot.
    pub const GLOW: SpringSpec = SpringSpec {
        duration: 0.42,
        bounce: 0.0,
    };
    /// A pane growing out of or into another.
    pub const MORPH: SpringSpec = SpringSpec {
        duration: 0.42,
        bounce: 0.18,
    };
    /// A selection moving to a place in the layout (`dom/selection.ts:111-119`).
    pub const LAYOUT: SpringSpec = SpringSpec {
        duration: 0.40,
        bounce: 0.16,
    };
    /// A selection that is dragged or pushed: follows closely, no overshoot.
    pub const TRACK: SpringSpec = SpringSpec {
        duration: 0.18,
        bounce: 0.0,
    };
    /// The switch-like thumb: the measured spring of `dom/control-motion.ts:10-25`,
    /// response 0.47 and damping 0.7, which is a damping ratio of 0.7: bounce 0.3.
    pub const SWITCH: SpringSpec = SpringSpec {
        duration: 0.47,
        bounce: 0.30,
    };
}

/// A value moving to a target under a spring, with the speed it has.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    /// Where it is.
    pub position: f64,
    /// How fast it is going, units a second.
    pub velocity: f64,
    /// Where it is going.
    pub target: f64,
    spec: SpringSpec,
}

impl Spring {
    /// A spring at rest at `value`.
    #[must_use]
    pub fn at_rest(value: f64, spec: SpringSpec) -> Self {
        Self {
            position: value,
            velocity: 0.0,
            target: value,
            spec,
        }
    }

    /// The spring's duration and bounce.
    #[must_use]
    pub fn spec(&self) -> SpringSpec {
        self.spec
    }

    /// Sends it somewhere else. Its position and its speed stay as they are, so
    /// a spring that is retargeted mid-swing carries its momentum into the
    /// next one instead of stopping and starting again (Glass HQ `core/spring.ts:24-91`).
    pub fn retarget(&mut self, target: f64) {
        self.target = target;
    }

    /// Changes how it swings, keeping position and speed: the selector takes
    /// the `track` spring while it is dragged and the `layout` spring when it is let go.
    pub fn respec(&mut self, spec: SpringSpec) {
        self.spec = spec;
    }

    /// True when it is at its target to within `tolerance` and not moving more than `tolerance` a second.
    #[must_use]
    pub fn settled(&self, tolerance: f64) -> bool {
        (self.position - self.target).abs() <= tolerance && self.velocity.abs() <= tolerance
    }

    /// Moves it forward `dt` seconds: the exact solution of the spring's
    /// equation, not an integration, so any `dt` is stable. A negative or
    /// non-finite `dt` is no time at all.
    pub fn step(&mut self, dt: f64) {
        if !(dt.is_finite() && dt > 0.0) {
            return;
        }
        let offset = self.position - self.target;
        let (offset, velocity) = solve(self.spec, offset, self.velocity, dt);
        self.position = self.target + offset;
        self.velocity = velocity;
    }
}

/// Offset from the target and velocity after `t` seconds of a spring that
/// starts with `x0` and `v0`.
fn solve(spec: SpringSpec, x0: f64, v0: f64, t: f64) -> (f64, f64) {
    let (omega, zeta) = (spec.omega(), spec.damping_ratio());
    if (zeta - 1.0).abs() < CRITICAL_BAND {
        // x = e^(-w t) (x0 + (v0 + w x0) t)
        let decay = (-omega * t).exp();
        let b = v0 + omega * x0;
        return (decay * (x0 + b * t), decay * (v0 - omega * (b * t)));
    }
    if zeta < 1.0 {
        let damped = omega * (1.0 - zeta * zeta).sqrt();
        let decay = (-zeta * omega * t).exp();
        let (sin, cos) = (damped * t).sin_cos();
        let b = (v0 + zeta * omega * x0) / damped;
        let x = decay * (x0 * cos + b * sin);
        let v = decay * (v0 * cos - (zeta * omega * v0 + omega * omega * x0) / damped * sin);
        return (x, v);
    }
    // two real roots, both negative: x = a e^(r1 t) + b e^(r2 t)
    let root = omega * (zeta * zeta - 1.0).sqrt();
    let (r1, r2) = (-zeta * omega + root, -zeta * omega - root);
    let a = (v0 - r2 * x0) / (r1 - r2);
    let b = x0 - a;
    let (e1, e2) = ((r1 * t).exp(), (r2 * t).exp());
    (a * e1 + b * e2, a * r1 * e1 + b * r2 * e2)
}

/// Smoothstep: 0 below `edge0`, 1 above `edge1`, smooth between.
#[must_use]
pub fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let width = edge1 - edge0;
    if width.abs() < f64::EPSILON {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / width).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How a pane stretches along its motion and squashes across it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stretch {
    /// Scale along the direction of motion: `1 + s`.
    pub along: f64,
    /// Scale across it: `1 / sqrt(1 + s)`, which keeps the area.
    pub across: f64,
}

impl Stretch {
    /// No stretch.
    pub const NONE: Self = Self {
        along: 1.0,
        across: 1.0,
    };

    /// The stretch for a stretch amount `s` (at least 0).
    #[must_use]
    pub fn of(s: f64) -> Self {
        let s = if s.is_finite() { s.max(0.0) } else { 0.0 };
        Self {
            along: 1.0 + s,
            across: 1.0 / (1.0 + s).sqrt(),
        }
    }
}

/// The sliding selector's stretch (`dom/selection.ts:111-119`):
/// `min(0.2, |v| * 0.00035)`, all of it while dragged and 0.6 of it otherwise,
/// for `velocity` in pixels a second.
#[must_use]
pub fn selector_stretch(velocity: f64, dragging: bool) -> Stretch {
    let s = (velocity.abs() * 0.000_35).min(0.2) * if dragging { 1.0 } else { 0.6 };
    Stretch::of(s)
}

/// The stretch of a switch-like control (`dom/control-motion.ts:113-115`):
/// `min(0.28, |speed| * 0.0007)` times how far it is pressed, 0 to 1.
#[must_use]
pub fn control_stretch(speed: f64, press_progress: f64) -> Stretch {
    Stretch::of((speed.abs() * 0.000_7).min(0.28) * press_progress.clamp(0.0, 1.0))
}

/// How far past its end a thing dragged by `offset` is shown: the rubber band
/// `limit (1 - 1 / (0.55 |offset| / limit + 1))` (`core/spring.ts:110-114`),
/// with the sign of the offset. It starts at the pace of 0.55 of the drag and
/// never goes past `limit`.
#[must_use]
pub fn rubber_band(offset: f64, limit: f64) -> f64 {
    if !(limit.is_finite() && limit > 0.0 && offset.is_finite()) {
        return 0.0;
    }
    offset.signum() * limit * (1.0 - 1.0 / (0.55 * offset.abs() / limit + 1.0))
}

/// The limit of the rubber band of a small control of this size
/// (`dom/interaction.ts`): `4 + 0.15 max(w, h)`.
#[must_use]
pub fn control_rubber_limit(width: f64, height: f64) -> f64 {
    0.15_f64.mul_add(width.max(height), 4.0)
}

/// The limit of the rubber band of a surface of this size: `min(10, 3 + 0.04 max(w, h))`.
#[must_use]
pub fn surface_rubber_limit(width: f64, height: f64) -> f64 {
    0.04_f64.mul_add(width.max(height), 3.0).min(10.0)
}

/// How much a pressed pane grows (`dom/interaction.ts:180`): `min(0.16, 8 / max(w, h))`.
#[must_use]
pub fn press_growth(width: f64, height: f64) -> f64 {
    let longest = width.max(height);
    if longest > 0.0 {
        (8.0 / longest).min(0.16)
    } else {
        0.16
    }
}

/// How lens-like a moving thumb is, 0 to 1, from its speed in pixels a
/// second: `smoothstep(0, v_ref, |v|)`. The reference speed is a few hundred
/// pixels a second; no source gives one, so it is this crate's suggestion.
#[must_use]
pub fn lens_progress(speed: f64, reference_speed: f64) -> f64 {
    smoothstep(0.0, reference_speed, speed.abs())
}

/// A suggested reference speed for [`lens_progress`], pixels a second.
pub const SUGGESTED_REFERENCE_SPEED: f64 = 300.0;

/// Which kind of thumb a lens belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbKind {
    /// A slider's or a toggle's thumb.
    SliderOrToggle,
    /// A tab bar's thumb.
    TabBar,
}

/// A thumb that cross-fades from an opaque white face with a small shadow to a
/// clear lens as its interaction progress `p` goes from 0 to 1. Lengths in dp
/// (Android's density-independent pixel, a logical pixel). Kyant0's constants:
/// `LiquidSlider.kt`, `LiquidToggle.kt`, `LiquidBottomTabs.kt:241-286`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThumbLens {
    /// Blur of what is behind: `8 dp (1 - p)`.
    pub blur: f64,
    /// Height of the refraction: `10 dp p`.
    pub refraction_height: f64,
    /// Amount of the refraction: `14 dp p`.
    pub refraction_amount: f64,
    /// Alpha of the highlight: `p`.
    pub highlight_alpha: f64,
    /// Radius of the inner shadow: `4 dp p` (slider, toggle) or `8 dp p` (tab bar).
    pub inner_shadow_radius: f64,
    /// Alpha of the white face: `1 - p`.
    pub face_alpha: f64,
    /// How much the pressed thumb is scaled: 1.5 (slider, toggle) or 78/56 (tab bar).
    pub press_scale: f64,
}

/// The thumb's lens at interaction progress `p` (0 to 1).
#[must_use]
pub fn thumb_lens(progress: f64, kind: ThumbKind) -> ThumbLens {
    let p = progress.clamp(0.0, 1.0);
    let (shadow, scale) = match kind {
        ThumbKind::SliderOrToggle => (4.0, 1.5),
        ThumbKind::TabBar => (8.0, 78.0 / 56.0),
    };
    ThumbLens {
        blur: 8.0 * (1.0 - p),
        refraction_height: 10.0 * p,
        refraction_amount: 14.0 * p,
        highlight_alpha: p,
        inner_shadow_radius: shadow * p,
        face_alpha: 1.0 - p,
        press_scale: scale,
    }
}

#[cfg(test)]
mod tests;
