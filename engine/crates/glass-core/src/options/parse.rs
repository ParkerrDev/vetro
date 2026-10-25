//! Reading options from text: one setting, and the one-line form of a preset
//! (`name:clear:dark, inherits:regular, frost:4`).
//!
//! Numbers are read the way C++'s `std::from_chars` reads them, because that
//! is how the plugin has always read them: the longest numeric prefix counts
//! and what follows is ignored, there is no leading `+`, and Lua hands numbers
//! over as text with a fraction (`"1.000000"`) which an int option still takes.

use super::ids::{FloatOption, IntOption};
use super::values::Values;

/// What can be wrong with a line of a preset's definition.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LineError {
    /// A piece between commas without a colon in it.
    #[error("preset: invalid token '{0}' (expected key:value)")]
    InvalidToken(String),
    /// A variant other than `dark` or `light`.
    #[error("preset: invalid variant '{0}' (expected dark or light)")]
    InvalidVariant(String),
    /// A key that is not an option, or a value that is not a number.
    #[error("preset: unknown or invalid setting '{0}:{1}'")]
    InvalidSetting(String, String),
    /// A line that never says which preset it defines.
    #[error("preset: missing required 'name' field")]
    MissingName,
}

/// Which layer of a preset a setting belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    /// Both appearances.
    #[default]
    Shared,
    /// Only the dark appearance.
    Dark,
    /// Only the light appearance.
    Light,
}

impl Variant {
    /// The variant named `text`: `"dark"` or `"light"`.
    #[must_use]
    pub fn from_text(text: &str) -> Option<Self> {
        match text {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None,
        }
    }
}

/// One line of the legacy form, read.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetLine {
    /// The preset the line defines.
    pub name: String,
    /// The layer the line's settings go to.
    pub variant: Variant,
    /// The preset this one inherits from, or empty.
    pub inherits: String,
    /// What the line sets.
    pub values: Values,
}

/// C's `isspace` in the C locale.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r')
}

fn trim(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// Hyprland stores dynamic tags with a trailing `*`; presets are registered
/// without it, so `firefox` and `firefox*` are one preset.
#[must_use]
pub fn strip_dynamic_tag_marker(tag: &str) -> &str {
    tag.strip_suffix('*').unwrap_or(tag)
}

/// The prefix of `text` that a float can be read from: `-`, then digits with
/// an optional fraction and exponent, or `inf`, `infinity`, `nan`.
fn float_prefix(text: &str) -> &str {
    let bytes = text.as_bytes();
    let sign = usize::from(bytes.first() == Some(&b'-'));
    let rest = text.get(sign..).unwrap_or("");
    let lower = rest.to_ascii_lowercase();
    for word in ["infinity", "inf", "nan"] {
        if lower.starts_with(word) {
            return text.get(..sign.saturating_add(word.len())).unwrap_or("");
        }
    }
    let digits = |from: usize| {
        rest.bytes()
            .skip(from)
            .take_while(u8::is_ascii_digit)
            .count()
    };
    let whole = digits(0);
    let fraction = if rest.as_bytes().get(whole) == Some(&b'.') {
        digits(whole.saturating_add(1))
    } else {
        0
    };
    if whole == 0 && fraction == 0 {
        return "";
    }
    let mut end = whole;
    if fraction > 0 || rest.as_bytes().get(whole) == Some(&b'.') {
        end = whole.saturating_add(1).saturating_add(fraction);
    }
    let exponent = exponent_len(rest.get(end..).unwrap_or(""));
    text.get(..sign.saturating_add(end).saturating_add(exponent))
        .unwrap_or("")
}

/// Length of an exponent (`e5`, `E-3`) at the start of `text`, or 0.
fn exponent_len(text: &str) -> usize {
    let bytes = text.as_bytes();
    if !matches!(bytes.first(), Some(b'e' | b'E')) {
        return 0;
    }
    let sign = usize::from(matches!(bytes.get(1), Some(b'+' | b'-')));
    let digits = bytes
        .iter()
        .skip(sign.saturating_add(1))
        .take_while(|b| b.is_ascii_digit())
        .count();
    if digits == 0 {
        0
    } else {
        sign.saturating_add(1).saturating_add(digits)
    }
}

/// Reads a float the way `std::from_chars` does; `None` when there is no
/// number at the start of `text` or it does not fit.
#[must_use]
pub fn parse_float(text: &str) -> Option<f32> {
    let prefix = float_prefix(text);
    if prefix.is_empty() {
        return None;
    }
    let value: f32 = prefix.parse().ok()?;
    let finite_text = !prefix.to_ascii_lowercase().contains(['i', 'n']);
    // out of range is a failure, as it is for from_chars: overflow, and underflow to nothing
    let out_of_range = value.is_infinite() || (value == 0.0 && has_digit_before_exponent(prefix));
    if finite_text && out_of_range {
        return None;
    }
    Some(value)
}

/// True when a digit other than zero stands before the exponent of `prefix`.
fn has_digit_before_exponent(prefix: &str) -> bool {
    let mantissa = prefix.split(['e', 'E']).next().unwrap_or("");
    mantissa.bytes().any(|b| (b'1'..=b'9').contains(&b))
}

/// Reads an int the way the plugin always has: decimal, or hexadecimal after
/// `0x`, the longest run of digits counting; `None` when there is none or it
/// does not fit in 64 bits.
#[must_use]
pub fn parse_int(text: &str) -> Option<i64> {
    let (digits, radix) = match text.get(..2) {
        Some("0x" | "0X") if text.len() > 2 => (text.get(2..).unwrap_or(""), 16),
        _ => (text, 10),
    };
    let (negative, digits) = match digits.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, digits),
    };
    let mut value: i128 = 0;
    let mut any = false;
    for c in digits.chars().take(256) {
        let Some(digit) = c.to_digit(radix) else {
            break;
        };
        value = value
            .checked_mul(i128::from(radix))?
            .checked_add(i128::from(digit))?;
        any = true;
    }
    if !any {
        return None;
    }
    let signed = if negative {
        value.checked_neg()?
    } else {
        value
    };
    i64::try_from(signed).ok()
}

/// Sets the option named `key` from its text. False for an unknown key or a
/// value that does not parse. Int options are tried first, then float ones.
pub fn set_field(values: &mut Values, key: &str, text: &str) -> bool {
    if let (Some(option), Some(parsed)) = (IntOption::from_key(key), parse_int(text)) {
        values.set_int(option, parsed);
        return true;
    }
    if let (Some(option), Some(parsed)) = (FloatOption::from_key(key), parse_float(text)) {
        values.set_float(option, parsed);
        return true;
    }
    false
}

/// Reads one line of the legacy form: `name:clear:dark, inherits:regular, frost:4`.
///
/// # Errors
///
/// A [`LineError`] for the first thing in the line that is wrong.
pub fn parse_line(input: &str) -> Result<PresetLine, LineError> {
    let mut line = PresetLine {
        name: String::new(),
        variant: Variant::Shared,
        inherits: String::new(),
        values: Values::UNSET,
    };
    for token in input.split(',').map(trim).filter(|t| !t.is_empty()) {
        let Some((key, value)) = token.split_once(':') else {
            return Err(LineError::InvalidToken(token.to_owned()));
        };
        apply_token(&mut line, trim(key), trim(value))?;
    }
    if line.name.is_empty() {
        return Err(LineError::MissingName);
    }
    line.name = strip_dynamic_tag_marker(&line.name).to_owned();
    line.inherits = strip_dynamic_tag_marker(&line.inherits).to_owned();
    Ok(line)
}

/// One `key:value` of a line.
fn apply_token(line: &mut PresetLine, key: &str, value: &str) -> Result<(), LineError> {
    match key {
        "name" => match value.split_once(':') {
            Some((name, variant)) => {
                line.variant = Variant::from_text(variant)
                    .ok_or_else(|| LineError::InvalidVariant(variant.to_owned()))?;
                name.clone_into(&mut line.name);
            }
            None => value.clone_into(&mut line.name),
        },
        "inherits" => value.clone_into(&mut line.inherits),
        _ if set_field(&mut line.values, key, value) => {}
        _ => return Err(LineError::InvalidSetting(key.to_owned(), value.to_owned())),
    }
    Ok(())
}
