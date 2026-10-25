//! What a call answers with: an integer, zero for success and negative for
//! each way it can fail, so that nothing is thrown across the boundary.

use crate::types::{
    GLASS_ERR_ARG, GLASS_ERR_CAPACITY, GLASS_ERR_NULL, GLASS_ERR_PANIC, GLASS_ERR_PARSE,
    GLASS_ERR_STORE, GLASS_ERR_UNKNOWN, GLASS_OK,
};

/// Why a call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Failure {
    #[error("a pointer that must not be null was")]
    Null,
    #[error("an argument is out of range")]
    Argument,
    #[error("a buffer is too small")]
    Capacity,
    #[error("the block of presets is not one, or is full")]
    Store,
    #[error("a line of a preset does not read")]
    Parse,
    #[error("a name or option that does not exist")]
    Unknown,
}

impl Failure {
    /// The code the caller is given.
    #[must_use]
    pub fn code(self) -> i32 {
        match self {
            Self::Null => GLASS_ERR_NULL,
            Self::Argument => GLASS_ERR_ARG,
            Self::Capacity => GLASS_ERR_CAPACITY,
            Self::Store => GLASS_ERR_STORE,
            Self::Parse => GLASS_ERR_PARSE,
            Self::Unknown => GLASS_ERR_UNKNOWN,
        }
    }
}

/// The result of a body: nothing, or a number the caller asked for.
pub type Outcome = Result<i32, Failure>;

/// Runs the body of a call and turns what it ends with into the answer. A
/// panic is stopped here and becomes [`GLASS_ERR_PANIC`]; with the release
/// profile's `panic = "abort"` a panic never gets this far, and nothing can
/// unwind into C++ either way.
pub fn guarded(body: impl FnOnce() -> Outcome) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(Ok(value)) => value,
        Ok(Err(failure)) => failure.code(),
        Err(_) => GLASS_ERR_PANIC,
    }
}

/// The answer of a body that has nothing to say but that it went through.
pub const DONE: Outcome = Ok(GLASS_OK);

/// A reference out of an argument that may be null.
pub fn present<T>(argument: Option<T>) -> Result<T, Failure> {
    argument.ok_or(Failure::Null)
}
