//! The resolution chain of an option.
//!
//! An option takes the first value that is set, looking in this order:
//!
//! 1. the preset's layer for the appearance (dark or light), then its shared
//!    layer;
//! 2. the preset it inherits from, and so on, at most [`MAX_INHERITANCE_DEPTH`]
//!    presets deep;
//! 3. the config's level for the appearance (`plugin:hyprglass:dark:...`);
//! 4. the config's global level (`plugin:hyprglass:...`);
//! 5. the default of whoever asks, which is not part of the chain.
//!
//! A value is set when it is zero or more.

use super::ids::{FloatOption, IntOption};
use super::store::{RecordView, StoreView};
use super::values::{Levels, Values};

/// How many presets deep a chain of `inherits` is followed; a longer chain, or
/// a loop, ends there.
pub const MAX_INHERITANCE_DEPTH: usize = 8;

/// What to resolve the options of.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// The presets.
    pub store: StoreView<'a>,
    /// The preset to start from; one that does not exist is skipped.
    pub preset: &'a str,
    /// The appearance.
    pub dark: bool,
    /// The config's levels.
    pub levels: &'a Levels,
}

/// The presets a chain of `inherits` visits, first the one named.
fn chain<'a>(store: &StoreView<'a>, preset: &str) -> Vec<RecordView<'a>> {
    let mut records: Vec<RecordView<'a>> = Vec::with_capacity(MAX_INHERITANCE_DEPTH);
    let mut name = preset;
    for _ in 0..MAX_INHERITANCE_DEPTH {
        let Some(record) = store.find(name) else {
            break;
        };
        records.push(record);
        if record.inherits.is_empty() {
            break;
        }
        name = record.inherits;
    }
    records
}

/// Every option, resolved: the value the chain gives, or negative where no
/// level sets it.
#[must_use]
pub fn resolve(request: &Request) -> Values {
    let presets = chain(&request.store, request.preset);
    let theme = if request.dark {
        &request.levels.dark
    } else {
        &request.levels.light
    };
    let mut out = Values::UNSET;
    for option in FloatOption::ALL {
        let from_presets = presets.iter().find_map(|record| {
            let themed = if request.dark {
                record.dark
            } else {
                record.light
            };
            [themed.float(option), record.shared.float(option)]
                .into_iter()
                .find(|v| *v >= 0.0)
        });
        let value = from_presets.or_else(|| {
            [theme.float(option), request.levels.global.float(option)]
                .into_iter()
                .find(|v| *v >= 0.0)
        });
        out.set_float(option, value.unwrap_or(super::ids::UNSET_FLOAT));
    }
    for option in IntOption::ALL {
        let from_presets = presets.iter().find_map(|record| {
            let themed = if request.dark {
                record.dark
            } else {
                record.light
            };
            [themed.int(option), record.shared.int(option)]
                .into_iter()
                .find(|v| *v >= 0)
        });
        let value = from_presets.or_else(|| {
            [theme.int(option), request.levels.global.int(option)]
                .into_iter()
                .find(|v| *v >= 0)
        });
        out.set_int(option, value.unwrap_or(super::ids::UNSET_INT));
    }
    out
}
