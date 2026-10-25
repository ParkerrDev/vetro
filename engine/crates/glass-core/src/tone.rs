//! The classic shader's tone curve (`src/Shaders.hpp`, "FROSTED TINT"), and
//! where it stops rising.
//!
//! Behind the classic glass each colour is scaled by
//! `brightness * (1 - adaptive_dim * s)` and lifted by
//! `adaptive_boost * (1 - s) / 2`, where `s` is a smoothstep of its luminance
//! from 0.25 to 0.55. A strong dim makes that curve fall as it goes: a dim of
//! 0.9 leaves a quarter grey at 0.25 and takes white to 0.10, so white letters
//! behind a window, blurred, come out dark inside a light rim. The shader is
//! therefore told where the curve first stops rising and how high it is
//! there, and holds whatever is brighter behind the glass to that height.
//! Below that point the curve is as it was; with a dim too gentle to fall
//! (below about 0.36 without a boost) nothing changes at all.

/// Where the smoothstep of the dim and the boost starts.
pub const DIM_FROM: f32 = 0.25;
/// Where it ends.
pub const DIM_TO: f32 = 0.55;

/// What [`Peak::at`] is when the curve never falls: beyond any luminance.
pub const NEVER: f32 = 1.0e6;

/// How many steps the curve is walked in, from black to white, to find
/// where it stops rising.
const STEPS: u16 = 2048;

/// Where the tone curve first stops rising, and how high it is there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peak {
    /// The luminance behind the glass past which nothing comes out darker
    /// than [`Peak::value`]; [`NEVER`] when the curve never falls.
    pub at: f32,
    /// What the curve comes to there.
    pub value: f32,
}

fn smoothstep(from: f32, to: f32, x: f32) -> f32 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * 2.0f32.mul_add(-t, 3.0)
}

/// The luminance a colour of luminance `lum` behind the glass comes out at,
/// as the shader has it before it is held up: the colour keeps its luminance
/// through the shader's desaturation, is scaled by the dim and lifted by the
/// boost (a grey, which adds its own level to the luminance).
#[must_use]
pub fn curve(lum: f32, dim: f32, boost: f32, brightness: f32) -> f32 {
    let s = smoothstep(DIM_FROM, DIM_TO, lum);
    let scaled = lum * brightness * dim.mul_add(-s, 1.0);
    (boost * 0.5).mul_add(1.0 - s, scaled)
}

/// Where the curve of `dim`, `boost` and `brightness` first stops rising.
#[must_use]
pub fn peak(dim: f32, boost: f32, brightness: f32) -> Peak {
    let step = 1.0 / f32::from(STEPS);
    let mut before = Peak {
        at: 0.0,
        value: curve(0.0, dim, boost, brightness),
    };
    for n in 1..=STEPS {
        let lum = f32::from(n) * step;
        let value = curve(lum, dim, boost, brightness);
        if value < before.value {
            return before;
        }
        before = Peak { at: lum, value };
    }
    Peak {
        at: NEVER,
        value: before.value,
    }
}

/// What the shader shows for luminance `lum` behind the glass: the curve,
/// held up to the peak's height past it.
#[must_use]
pub fn held(lum: f32, dim: f32, boost: f32, brightness: f32, peak: Peak) -> f32 {
    let value = curve(lum, dim, boost, brightness);
    if lum > peak.at && value < peak.value {
        peak.value
    } else {
        value
    }
}

#[cfg(test)]
mod tests;
