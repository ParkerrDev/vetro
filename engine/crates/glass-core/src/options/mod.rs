//! Options and presets: what the config, a preset and the defaults say about
//! how glass looks, and the chain that decides between them.
//!
//! * [`ids`]: every option, by kind and key;
//! * [`values`]: a value for each of them, at one level;
//! * [`parse`]: reading them from text, and the one-line form of a preset;
//! * [`builtin`]: the defaults and the presets that ship;
//! * [`store`]: presets as a value to edit and as bytes to read in place;
//! * [`mod@resolve`]: the chain, from preset to default;
//! * [`model`]: what the chain gives the lens model, in device pixels.

pub mod builtin;
pub mod ids;
pub mod model;
pub mod parse;
pub mod resolve;
pub mod store;
pub mod values;

pub use ids::{FLOAT_COUNT, FloatOption, INT_COUNT, IntOption, UNSET_FLOAT, UNSET_INT};
pub use model::{Changes, Layered, MIN_THIRD, ModelParams, changes, model_params, uses_model};
pub use parse::{LineError, PresetLine, Variant, parse_line};
pub use resolve::{MAX_INHERITANCE_DEPTH, Request, resolve};
pub use store::{Preset, Store, StoreError, StoreView};
pub use values::{Levels, Values};

#[cfg(test)]
mod tests;
