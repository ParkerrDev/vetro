//! The C ABI of the glass model: a static library the Hyprland plugin links.
//!
//! The model itself is `glass-core`, which forbids `unsafe` code. This crate
//! only carries its questions and answers across the boundary as plain
//! `#[repr(C)]` data in memory the caller owns. `unsafe` is denied here too,
//! except in `boundary` (two functions that turn a pointer and a length into
//! a slice) and in `exports` (the attribute that gives each function its C
//! name); both say so, and every `unsafe` block states why it is sound.
//!
//! `include/glass_ffi.h` is generated from this crate's source by
//! `tests/header.rs`, which fails when the committed header is stale.

mod boundary;
mod exports;
mod status;
mod types;

pub use exports::*;
pub use status::Failure;
pub use types::*;
