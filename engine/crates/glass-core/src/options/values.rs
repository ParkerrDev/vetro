//! One level's worth of option values: a layer of a preset, a level of the
//! config, or the result of resolving them all.

use super::ids::{FLOAT_COUNT, FloatOption, INT_COUNT, IntOption, UNSET_FLOAT, UNSET_INT};

/// A value for every option, with a negative one standing for "not set here".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Values {
    /// Float options, by [`FloatOption::index`].
    pub f: [f32; FLOAT_COUNT],
    /// Int options, by [`IntOption::index`].
    pub i: [i64; INT_COUNT],
}

impl Values {
    /// Nothing set.
    pub const UNSET: Self = Self {
        f: [UNSET_FLOAT; FLOAT_COUNT],
        i: [UNSET_INT; INT_COUNT],
    };

    /// The value of a float option; negative when it is not set.
    #[must_use]
    pub fn float(&self, option: FloatOption) -> f32 {
        self.f.get(option.index()).copied().unwrap_or(UNSET_FLOAT)
    }

    /// The value of an int option; negative when it is not set.
    #[must_use]
    pub fn int(&self, option: IntOption) -> i64 {
        self.i.get(option.index()).copied().unwrap_or(UNSET_INT)
    }

    /// The value of a float option, or `fallback` when it is not set.
    #[must_use]
    pub fn float_or(&self, option: FloatOption, fallback: f32) -> f32 {
        let value = self.float(option);
        if value >= 0.0 { value } else { fallback }
    }

    /// The value of an int option, or `fallback` when it is not set.
    #[must_use]
    pub fn int_or(&self, option: IntOption, fallback: i64) -> i64 {
        let value = self.int(option);
        if value >= 0 { value } else { fallback }
    }

    /// Sets a float option.
    pub fn set_float(&mut self, option: FloatOption, value: f32) {
        if let Some(slot) = self.f.get_mut(option.index()) {
            *slot = value;
        }
    }

    /// Sets an int option.
    pub fn set_int(&mut self, option: IntOption, value: i64) {
        if let Some(slot) = self.i.get_mut(option.index()) {
            *slot = value;
        }
    }

    /// Copies every option that `over` sets (zero or more) onto `self`.
    pub fn merge(&mut self, over: &Self) {
        for (slot, value) in self.f.iter_mut().zip(&over.f) {
            if *value >= 0.0 {
                *slot = *value;
            }
        }
        for (slot, value) in self.i.iter_mut().zip(&over.i) {
            if *value >= 0 {
                *slot = *value;
            }
        }
    }

    /// True when `option` is set to more than zero.
    #[must_use]
    pub fn float_is_positive(&self, option: FloatOption) -> bool {
        self.float(option) > 0.0
    }
}

impl Default for Values {
    fn default() -> Self {
        Self::UNSET
    }
}

/// The three levels of the config an option can be set at, besides presets.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Levels {
    /// `plugin:hyprglass:<option>`.
    pub global: Values,
    /// `plugin:hyprglass:dark:<option>`.
    pub dark: Values,
    /// `plugin:hyprglass:light:<option>`.
    pub light: Values,
}
