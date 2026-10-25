#![forbid(unsafe_code)]
//! The glass model as pure computation.
//!
//! Nothing here touches Hyprland, GL, the clock or the terminal: every
//! function takes numbers and returns numbers, so the plugin's C++ adapter can
//! ask it every decision and a test can check each one without a display.

pub mod geometry;
pub mod lens;
pub mod material;
pub mod motion;
pub mod num;
pub mod options;
pub mod panes;
pub mod quartz;
pub mod schedule;
pub mod shaders;
pub mod shape;
pub mod tone;
pub mod tuning;
