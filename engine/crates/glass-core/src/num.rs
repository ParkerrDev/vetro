//! Numeric conversions, in one place.
//!
//! Rust's `as` between a float and an integer saturates and maps NaN to zero:
//! defined, never undefined. It is still a lossy cast, which the lints flag, so
//! the few that the model needs are written once here, each saying why it is
//! the conversion wanted, and the rest of the crate does not use `as`.

/// The nearest `i32` to `x`, rounding halves away from zero; saturates at the
/// ends of the range, and NaN gives 0.
#[must_use]
pub fn round_i32(x: f64) -> i32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "float to int `as` saturates and maps NaN to 0, which is the conversion wanted"
    )]
    {
        x.round() as i32
    }
}

/// The smallest `i32` not below `x`; saturates, and NaN gives 0.
#[must_use]
pub fn ceil_i32(x: f64) -> i32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "float to int `as` saturates and maps NaN to 0, which is the conversion wanted"
    )]
    {
        x.ceil() as i32
    }
}

/// The largest `i32` not above `x`; saturates, and NaN gives 0.
#[must_use]
pub fn floor_i32(x: f64) -> i32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "float to int `as` saturates and maps NaN to 0, which is the conversion wanted"
    )]
    {
        x.floor() as i32
    }
}

/// `x` with the fraction cut off, as `i32`; saturates, and NaN gives 0.
#[must_use]
pub fn trunc_i32(x: f64) -> i32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "float to int `as` truncates toward zero, saturates, and maps NaN to 0, which is the conversion wanted"
    )]
    {
        x as i32
    }
}

/// The byte nearest to `x` times 255, with `x` held to `[0, 1]`: how a level
/// between black and white is stored in 8 bits.
#[must_use]
pub fn unit_to_byte(x: f64) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is held to 0..=255 before the cast, and NaN maps to 0"
    )]
    {
        (x.clamp(0.0, 1.0) * 255.0).round() as u8
    }
}

/// An `f64` for a count. Exact for any count below 2^53, far above every size
/// this crate meets.
#[must_use]
pub fn count_f64(n: usize) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts here are cells and pixels, orders of magnitude below 2^53"
    )]
    {
        n as f64
    }
}

/// A float of single precision for a double, the way a uniform is set.
#[must_use]
pub fn to_f32(x: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the value is handed to the GPU as a single-precision float: rounding to one is the point"
    )]
    {
        x as f32
    }
}

/// The smallest `i32` not below a float of single precision; saturates, and NaN gives 0.
#[must_use]
pub fn ceil_f32_i32(x: f32) -> i32 {
    ceil_i32(f64::from(x))
}

/// An `f32` for a small integer (a reduction, a pass count). Exact below 2^24.
#[must_use]
pub fn int_f32(n: i32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "the integers here are reductions and counts of passes, far below 2^24"
    )]
    {
        n as f32
    }
}

/// The `usize` for a non-negative `i32`, or 0 for a negative one.
#[must_use]
pub fn index_of(n: i32) -> usize {
    usize::try_from(n).unwrap_or(0)
}

/// An `i32` for a count, saturating at `i32::MAX`.
#[must_use]
pub fn count_i32(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_to_int_rounds_as_the_model_expects() {
        assert_eq!(round_i32(2.5), 3);
        assert_eq!(round_i32(-2.5), -3);
        assert_eq!(ceil_i32(0.1), 1);
        assert_eq!(floor_i32(-0.1), -1);
        assert_eq!(trunc_i32(-1.9), -1);
    }

    #[test]
    fn float_to_int_never_wraps() {
        assert_eq!(round_i32(f64::NAN), 0);
        assert_eq!(round_i32(1e300), i32::MAX);
        assert_eq!(round_i32(-1e300), i32::MIN);
    }

    #[test]
    fn bytes_hold_a_unit_level() {
        assert_eq!(unit_to_byte(0.0), 0);
        assert_eq!(unit_to_byte(1.0), 255);
        assert_eq!(unit_to_byte(0.5), 128);
        assert_eq!(unit_to_byte(-3.0), 0);
        assert_eq!(unit_to_byte(7.0), 255);
        assert_eq!(unit_to_byte(f64::NAN), 0);
    }

    #[test]
    fn counts_convert_without_loss() {
        assert!((count_f64(1 << 20) - 1_048_576.0).abs() < f64::EPSILON);
        assert_eq!(index_of(-4), 0);
        assert_eq!(index_of(9), 9);
        assert_eq!(count_i32(usize::MAX), i32::MAX);
    }
}
