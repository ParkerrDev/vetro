//! The presets, as a value that can be edited and as bytes that can be read
//! without copying.
//!
//! The plugin keeps the committed presets as a block of bytes in memory it
//! owns, and hands that block to every resolution. Nothing here is global: a
//! [`Store`] is edited, written out with [`Store::to_bytes`], and read back by
//! a [`StoreView`] that only ever borrows.
//!
//! The block is a magic number, a count, and per preset two lengths, the name,
//! the name it inherits from, and three layers of values, all little-endian.

use super::builtin::builtin_presets;
use super::ids::{FLOAT_COUNT, FloatOption, INT_COUNT, IntOption, UNSET_FLOAT, UNSET_INT};
use super::parse::{LineError, Variant, parse_line, set_field, strip_dynamic_tag_marker};
use super::values::Values;

const MAGIC: [u8; 4] = *b"GPS1";
const HEADER_BYTES: usize = 8;
/// Bytes of one layer of values: every float, then every int.
pub const LAYER_BYTES: usize = FLOAT_COUNT * 4 + INT_COUNT * 8;

/// Why a block of bytes is not a store, or a store will not fit one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    /// The block does not start with the store's magic number.
    #[error("not a preset store")]
    BadMagic,
    /// The block ends before the presets it counts.
    #[error("the preset store is cut short")]
    Truncated,
    /// A name that does not fit the store's 16-bit length.
    #[error("a preset name of {0} bytes is too long")]
    NameTooLong(usize),
    /// A name that is not UTF-8.
    #[error("a preset name is not UTF-8")]
    NotUtf8,
}

/// A preset: a name, the preset it inherits from, and three layers of options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Preset {
    /// What the config calls it.
    pub name: String,
    /// The preset to look in next for what this one does not set, or empty.
    pub inherits: String,
    /// Settings for both appearances.
    pub shared: Values,
    /// Settings for the dark appearance.
    pub dark: Values,
    /// Settings for the light appearance.
    pub light: Values,
}

impl Preset {
    fn layer_mut(&mut self, variant: Variant) -> &mut Values {
        match variant {
            Variant::Shared => &mut self.shared,
            Variant::Dark => &mut self.dark,
            Variant::Light => &mut self.light,
        }
    }
}

/// A set of presets, by name.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Store {
    presets: Vec<Preset>,
}

impl Store {
    /// A store without presets.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A store with the presets that ship with the plugin.
    #[must_use]
    pub fn builtin() -> Self {
        Self {
            presets: builtin_presets(),
        }
    }

    /// The preset called `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Preset> {
        self.presets.iter().find(|p| p.name == name)
    }

    /// How many presets there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.presets.len()
    }

    /// True when there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.presets.is_empty()
    }

    /// Position of the preset called `name`, which is made if there is none.
    fn ensure(&mut self, name: &str) -> usize {
        if let Some(at) = self.presets.iter().position(|p| p.name == name) {
            return at;
        }
        self.presets.push(Preset {
            name: name.to_owned(),
            ..Preset::default()
        });
        self.presets.len().saturating_sub(1)
    }

    /// Makes sure a preset called `name` exists (without a trailing `*`).
    pub fn define(&mut self, name: &str) {
        self.ensure(strip_dynamic_tag_marker(name));
    }

    /// Adds one line of the legacy form to the presets: its settings are
    /// merged into the layer it names, over what earlier lines set.
    ///
    /// # Errors
    ///
    /// The [`LineError`] of a line that does not read.
    pub fn add_line(&mut self, text: &str) -> Result<(), LineError> {
        let line = parse_line(text)?;
        let at = self.ensure(&line.name);
        if let Some(preset) = self.presets.get_mut(at) {
            if !line.inherits.is_empty() {
                preset.inherits = line.inherits;
            }
            preset.layer_mut(line.variant).merge(&line.values);
        }
        Ok(())
    }

    /// Sets one option of one layer of the preset `name` from its text, making
    /// the preset if need be. False for an unknown key or a bad value.
    pub fn set_field(&mut self, name: &str, variant: Variant, key: &str, text: &str) -> bool {
        let at = self.ensure(strip_dynamic_tag_marker(name));
        self.presets
            .get_mut(at)
            .is_some_and(|preset| set_field(preset.layer_mut(variant), key, text))
    }

    /// Says which preset `name` inherits from.
    pub fn set_inherits(&mut self, name: &str, inherits: &str) {
        let at = self.ensure(strip_dynamic_tag_marker(name));
        if let Some(preset) = self.presets.get_mut(at) {
            strip_dynamic_tag_marker(inherits).clone_into(&mut preset.inherits);
        }
    }

    /// The presets a config ends up with: the built-in ones, and over them what
    /// the user defined. A user preset that has a built-in one's name changes
    /// only what it sets.
    #[must_use]
    pub fn committed(user: &Self) -> Self {
        let mut merged = Self::builtin();
        for preset in &user.presets {
            match merged.presets.iter_mut().find(|p| p.name == preset.name) {
                Some(builtin) => {
                    if !preset.inherits.is_empty() {
                        builtin.inherits.clone_from(&preset.inherits);
                    }
                    builtin.shared.merge(&preset.shared);
                    builtin.dark.merge(&preset.dark);
                    builtin.light.merge(&preset.light);
                }
                None => merged.presets.push(preset.clone()),
            }
        }
        merged
    }

    /// True when any preset sets `option` to more than zero in any layer.
    #[must_use]
    pub fn any_float_positive(&self, option: FloatOption) -> bool {
        self.presets.iter().any(|p| {
            [&p.shared, &p.dark, &p.light]
                .iter()
                .any(|layer| layer.float_is_positive(option))
        })
    }

    /// The store as the bytes a [`StoreView`] reads.
    ///
    /// # Errors
    ///
    /// [`StoreError::NameTooLong`] for a name or an inherited name over 65535 bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StoreError> {
        let mut out = Vec::with_capacity(
            HEADER_BYTES.saturating_add(self.presets.len().saturating_mul(3 * LAYER_BYTES + 32)),
        );
        out.extend_from_slice(&MAGIC);
        let count = u32::try_from(self.presets.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&count.to_le_bytes());
        for preset in &self.presets {
            for text in [&preset.name, &preset.inherits] {
                let length =
                    u16::try_from(text.len()).map_err(|_| StoreError::NameTooLong(text.len()))?;
                out.extend_from_slice(&length.to_le_bytes());
            }
            out.extend_from_slice(preset.name.as_bytes());
            out.extend_from_slice(preset.inherits.as_bytes());
            for layer in [&preset.shared, &preset.dark, &preset.light] {
                write_layer(layer, &mut out);
            }
        }
        Ok(out)
    }

    /// Reads a store back from the bytes [`Self::to_bytes`] made.
    ///
    /// # Errors
    ///
    /// A [`StoreError`] when the bytes are not a whole store.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        let view = StoreView::new(bytes)?;
        let mut presets = Vec::with_capacity(view.count);
        for record in view.records() {
            let record = record?;
            presets.push(Preset {
                name: record.name.to_owned(),
                inherits: record.inherits.to_owned(),
                shared: record.shared.to_values(),
                dark: record.dark.to_values(),
                light: record.light.to_values(),
            });
        }
        Ok(Self { presets })
    }
}

fn write_layer(layer: &Values, out: &mut Vec<u8>) {
    for value in layer.f {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for value in layer.i {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

/// One layer of one preset, read in place.
#[derive(Debug, Clone, Copy)]
pub struct LayerView<'a>(&'a [u8]);

impl LayerView<'_> {
    /// The value of a float option; negative when the layer does not set it.
    #[must_use]
    pub fn float(&self, option: FloatOption) -> f32 {
        let at = option.index().saturating_mul(4);
        self.0
            .get(at..at.saturating_add(4))
            .and_then(|b| <[u8; 4]>::try_from(b).ok())
            .map_or(UNSET_FLOAT, f32::from_le_bytes)
    }

    /// The value of an int option; negative when the layer does not set it.
    #[must_use]
    pub fn int(&self, option: IntOption) -> i64 {
        let at = FLOAT_COUNT
            .saturating_mul(4)
            .saturating_add(option.index().saturating_mul(8));
        self.0
            .get(at..at.saturating_add(8))
            .and_then(|b| <[u8; 8]>::try_from(b).ok())
            .map_or(UNSET_INT, i64::from_le_bytes)
    }

    fn to_values(self) -> Values {
        let mut values = Values::UNSET;
        for option in FloatOption::ALL {
            values.set_float(option, self.float(option));
        }
        for option in IntOption::ALL {
            values.set_int(option, self.int(option));
        }
        values
    }
}

/// One preset, read in place.
#[derive(Debug, Clone, Copy)]
pub struct RecordView<'a> {
    /// Its name.
    pub name: &'a str,
    /// The preset it inherits from, or empty.
    pub inherits: &'a str,
    /// Settings for both appearances.
    pub shared: LayerView<'a>,
    /// Settings for the dark appearance.
    pub dark: LayerView<'a>,
    /// Settings for the light appearance.
    pub light: LayerView<'a>,
}

/// The presets of a block of bytes, read in place.
#[derive(Debug, Clone, Copy)]
pub struct StoreView<'a> {
    count: usize,
    body: &'a [u8],
}

impl<'a> StoreView<'a> {
    /// Reads the header of a block.
    ///
    /// # Errors
    ///
    /// [`StoreError::BadMagic`] or [`StoreError::Truncated`] for a block that is not a store.
    pub fn new(bytes: &'a [u8]) -> Result<Self, StoreError> {
        let header = bytes.get(..HEADER_BYTES).ok_or(StoreError::Truncated)?;
        if header.get(..4) != Some(&MAGIC[..]) {
            return Err(StoreError::BadMagic);
        }
        let count = header
            .get(4..8)
            .and_then(|b| <[u8; 4]>::try_from(b).ok())
            .map_or(0, u32::from_le_bytes);
        Ok(Self {
            count: usize::try_from(count).unwrap_or(0),
            body: bytes.get(HEADER_BYTES..).unwrap_or(&[]),
        })
    }

    /// The presets, in the order they were written; ends early at a record that is cut short.
    pub fn records(&self) -> impl Iterator<Item = Result<RecordView<'a>, StoreError>> {
        let mut rest = self.body;
        let mut failed = false;
        (0..self.count).map_while(move |_| {
            if failed {
                return None;
            }
            let read = read_record(rest);
            match read {
                Ok((record, after)) => {
                    rest = after;
                    Some(Ok(record))
                }
                Err(error) => {
                    failed = true;
                    Some(Err(error))
                }
            }
        })
    }

    /// The preset called `name`.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<RecordView<'a>> {
        self.records()
            .map_while(Result::ok)
            .find(|r| r.name == name)
    }

    /// True when a preset is called `name`.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.find(name).is_some()
    }

    /// True when any preset sets `option` to more than zero in any layer.
    #[must_use]
    pub fn any_float_positive(&self, option: FloatOption) -> bool {
        self.records().map_while(Result::ok).any(|r| {
            [r.shared, r.dark, r.light]
                .iter()
                .any(|layer| layer.float(option) > 0.0)
        })
    }
}

/// Reads the record at the start of `bytes`, and what follows it.
fn read_record(bytes: &[u8]) -> Result<(RecordView<'_>, &[u8]), StoreError> {
    let length = |at: usize| {
        bytes
            .get(at..at.saturating_add(2))
            .and_then(|b| <[u8; 2]>::try_from(b).ok())
            .map(u16::from_le_bytes)
            .map(usize::from)
            .ok_or(StoreError::Truncated)
    };
    let (name_len, inherits_len) = (length(0)?, length(2)?);
    let text_end = 4_usize
        .saturating_add(name_len)
        .saturating_add(inherits_len);
    let text = |from: usize, to: usize| {
        bytes
            .get(from..to)
            .ok_or(StoreError::Truncated)
            .and_then(|b| std::str::from_utf8(b).map_err(|_| StoreError::NotUtf8))
    };
    let name = text(4, 4_usize.saturating_add(name_len))?;
    let inherits = text(4_usize.saturating_add(name_len), text_end)?;
    let layer = |n: usize| {
        let from = text_end.saturating_add(n.saturating_mul(LAYER_BYTES));
        bytes
            .get(from..from.saturating_add(LAYER_BYTES))
            .map(LayerView)
            .ok_or(StoreError::Truncated)
    };
    let record = RecordView {
        name,
        inherits,
        shared: layer(0)?,
        dark: layer(1)?,
        light: layer(2)?,
    };
    let after = bytes
        .get(text_end.saturating_add(3 * LAYER_BYTES)..)
        .ok_or(StoreError::Truncated)?;
    Ok((record, after))
}
