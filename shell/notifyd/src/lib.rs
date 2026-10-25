//! The core of `vetro-notifyd`: the notifications that programs send over the
//! session bus, as Vetro's toasts and notification centre show them. Nothing
//! here prints, touches a file or the bus: the binary does that over these.
//!
//! - [`note`]: a notification as its sender asked for it, checked and bounded
//! - [`store`]: the ones kept, which are up as toasts, and when each goes
//! - [`picture`]: a picture handed over in memory, as a PNG file's bytes
//! - [`lines`]: what the shell plugin says to the daemon, and is told back
//! - [`saved`]: Do Not Disturb, kept across restarts

pub mod lines;
pub mod note;
pub mod picture;
pub mod saved;
pub mod store;
