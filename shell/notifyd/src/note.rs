//! A notification as its sender asked for it: the arguments of `Notify` and
//! the hints the shell uses, checked and cut to size. Anything unreadable is
//! left out rather than refused, as the specification asks of a server.

use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, Value};

/// The longest each field is kept, in bytes (cut at a character boundary).
const MAX_NAME: usize = 256;
const MAX_SUMMARY: usize = 1024;
const MAX_BODY: usize = 16 * 1024;
const MAX_ICON: usize = 4096;
const MAX_ACTIONS: usize = 8;
const MAX_ARGS: usize = 64;
const MAX_ARG: usize = 4096;
/// The widest and tallest picture taken from memory.
const MAX_SIDE: i32 = 4096;
/// A toast that names no time of its own stays this long.
pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgency {
    Low,
    Normal,
    Critical,
}

impl Urgency {
    /// The number the specification gives it.
    #[must_use]
    pub fn level(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Normal => 1,
            Self::Critical => 2,
        }
    }
}

/// How long a toast stays up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timeout {
    /// Until it is put away.
    Never,
    /// This many milliseconds of being looked at.
    After(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub key: String,
    pub label: String,
}

/// A picture handed over in memory (the image-data hint): rows of 8-bit RGB
/// or RGBA, `stride` bytes apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub alpha: bool,
    pub data: Vec<u8>,
}

/// What `Notify` was called with, as it came.
pub struct Asked<'a> {
    pub app: &'a str,
    pub icon: &'a str,
    pub summary: &'a str,
    pub body: &'a str,
    pub actions: &'a [String],
    pub hints: &'a HashMap<String, OwnedValue>,
    pub expire_timeout: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub app: String,
    pub summary: String,
    pub body: String,
    /// The `app_icon` argument: a name in the icon theme, a path or a URI.
    pub icon: String,
    /// The image-path hint, the same three kinds.
    pub image: String,
    /// The image-data hint.
    pub pixels: Option<Pixels>,
    pub desktop_entry: String,
    pub urgency: Urgency,
    pub actions: Vec<Action>,
    /// The command an Omarchy notification runs when clicked
    /// (`omarchy-notification-send --exec`): an argv, never a shell line.
    pub exec: Vec<String>,
    /// Stays when one of its actions is chosen.
    pub resident: bool,
    /// A toast only, never kept in the centre.
    pub transient: bool,
    pub timeout: Timeout,
}

/// `text` cut to at most `max` bytes, at a character boundary.
#[must_use]
pub fn cut(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    let end = (0..=max)
        .rev()
        .find(|&at| text.is_char_boundary(at))
        .unwrap_or(0);
    text.get(..end).unwrap_or("").to_owned()
}

/// The body without `<img>` tags: the server says it cannot show pictures in
/// a body, and a tag left in would be drawn as text.
#[must_use]
pub fn without_images(body: &str) -> String {
    let mut kept = String::with_capacity(body.len());
    let mut rest = body;
    // each turn takes at least one tag off the front: bounded by the length
    for _ in 0..=body.len() {
        let Some(start) = find_image_tag(rest) else {
            break;
        };
        kept.push_str(rest.get(..start).unwrap_or(""));
        let after = rest.get(start..).unwrap_or("");
        rest = after
            .find('>')
            .map_or("", |end| after.get(end.saturating_add(1)..).unwrap_or(""));
    }
    kept.push_str(rest);
    kept
}

fn find_image_tag(text: &str) -> Option<usize> {
    let lower = text.to_ascii_lowercase();
    let mut from = 0;
    // a match moves `from` past it: bounded by the length
    for _ in 0..=lower.len() {
        let found = lower
            .get(from..)?
            .find("<img")
            .map(|at| at.saturating_add(from))?;
        let next = lower.as_bytes().get(found.saturating_add(4)).copied();
        if matches!(next, None | Some(b' ' | b'\t' | b'\n' | b'/' | b'>')) {
            return Some(found);
        }
        from = found.saturating_add(4);
    }
    None
}

fn text<'a>(hints: &'a HashMap<String, OwnedValue>, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .filter_map(|name| hints.get(*name))
        .find_map(|value| <&str>::try_from(&**value).ok())
}

fn flag(hints: &HashMap<String, OwnedValue>, name: &str) -> bool {
    hints
        .get(name)
        .is_some_and(|value| bool::try_from(&**value).unwrap_or(false))
}

/// The urgency hint is a byte, but some senders send another kind of number.
fn urgency(hints: &HashMap<String, OwnedValue>) -> Urgency {
    let level = hints.get("urgency").and_then(|value| match &**value {
        Value::U8(n) => Some(i64::from(*n)),
        Value::I32(n) => Some(i64::from(*n)),
        Value::U32(n) => Some(i64::from(*n)),
        Value::I64(n) => Some(*n),
        _ => None,
    });
    match level {
        Some(0) => Urgency::Low,
        Some(2) => Urgency::Critical,
        _ => Urgency::Normal,
    }
}

fn pixels(hints: &HashMap<String, OwnedValue>) -> Option<Pixels> {
    let value = ["image-data", "image_data", "icon_data"]
        .iter()
        .find_map(|name| hints.get(*name))?;
    let (width, height, stride, alpha, bits, channels, data): (
        i32,
        i32,
        i32,
        bool,
        i32,
        i32,
        Vec<u8>,
    ) = <_>::try_from(&**value).ok()?;
    let wanted_channels = if alpha { 4 } else { 3 };
    let sides = 1..=MAX_SIDE;
    if bits != 8
        || channels != wanted_channels
        || !sides.contains(&width)
        || !sides.contains(&height)
    {
        return None;
    }
    let (width, height) = (u32::try_from(width).ok()?, u32::try_from(height).ok()?);
    let stride = usize::try_from(stride).ok()?;
    let row = usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(channels).ok()?)?;
    let last = usize::try_from(height).ok()?.checked_sub(1)?;
    let needed = stride.checked_mul(last)?.checked_add(row)?;
    (stride >= row && data.len() >= needed).then_some(Pixels {
        width,
        height,
        stride,
        alpha,
        data,
    })
}

/// The command an Omarchy notification carries, as Omarchy reads it: a JSON
/// list of strings whose first is a program, not an option.
fn exec(hints: &HashMap<String, OwnedValue>) -> Vec<String> {
    let parsed: Option<Vec<String>> =
        text(hints, &["omarchy-exec-argv"]).and_then(|json| serde_json::from_str(json).ok());
    match parsed {
        Some(argv)
            if (1..=MAX_ARGS).contains(&argv.len())
                && argv.iter().all(|arg| arg.len() <= MAX_ARG)
                && argv
                    .first()
                    .is_some_and(|program| !program.is_empty() && !program.starts_with('-')) =>
        {
            argv
        }
        _ => Vec::new(),
    }
}

/// The actions, which come as a flat list of key and label pairs.
fn actions(flat: &[String]) -> Vec<Action> {
    flat.chunks_exact(2)
        .take(MAX_ACTIONS)
        .filter_map(|pair| match pair {
            [key, label] if !key.is_empty() => Some(Action {
                key: cut(key, MAX_NAME),
                label: cut(label, MAX_NAME),
            }),
            _ => None,
        })
        .collect()
}

impl Note {
    #[must_use]
    pub fn new(asked: &Asked<'_>) -> Self {
        let hints = asked.hints;
        let urgency = urgency(hints);
        let timeout = match (urgency, asked.expire_timeout) {
            (Urgency::Critical, _) | (_, 0) => Timeout::Never,
            (_, ms) => Timeout::After(u64::try_from(ms).unwrap_or(DEFAULT_TIMEOUT_MS)),
        };
        Self {
            app: cut(asked.app, MAX_NAME),
            summary: cut(asked.summary, MAX_SUMMARY),
            body: without_images(&cut(asked.body, MAX_BODY)),
            icon: cut(asked.icon, MAX_ICON),
            image: text(hints, &["image-path", "image_path"])
                .map_or_else(String::new, |path| cut(path, MAX_ICON)),
            pixels: pixels(hints),
            desktop_entry: text(hints, &["desktop-entry"])
                .map_or_else(String::new, |name| cut(name, MAX_NAME)),
            urgency,
            actions: actions(asked.actions),
            exec: exec(hints),
            resident: flag(hints, "resident"),
            transient: flag(hints, "transient"),
            timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Asked, DEFAULT_TIMEOUT_MS, Note, Timeout, Urgency, cut, without_images};
    use std::collections::HashMap;
    use zbus::zvariant::{OwnedValue, Value};

    fn hint(value: Value<'_>) -> OwnedValue {
        OwnedValue::try_from(value).unwrap_or_else(|_| OwnedValue::from(0u8))
    }

    fn note(actions: &[&str], hints: &[(&str, Value<'_>)], expire_timeout: i32) -> Note {
        let hints: HashMap<String, OwnedValue> = hints
            .iter()
            .map(|(name, value)| ((*name).to_owned(), hint(value.clone())))
            .collect();
        let actions: Vec<String> = actions.iter().map(|text| (*text).to_owned()).collect();
        Note::new(&Asked {
            app: "Vetro",
            icon: "dev.vetro.Glass",
            summary: "Liquid glass",
            body: "Every surface",
            actions: &actions,
            hints: &hints,
            expire_timeout,
        })
    }

    #[test]
    fn cuts_at_a_character_boundary() {
        assert_eq!(cut("glass", 10), "glass");
        assert_eq!(cut("vetrò", 5), "vetr");
        assert_eq!(cut("vetrò", 6), "vetrò");
    }

    #[test]
    fn leaves_pictures_out_of_the_body() {
        assert_eq!(without_images("a<img src=\"x.png\"/>b"), "ab");
        assert_eq!(without_images("<IMG alt='x'>left"), "left");
        assert_eq!(without_images("<b>bold</b> <imgs>"), "<b>bold</b> <imgs>");
        assert_eq!(without_images("cut <img src="), "cut ");
    }

    #[test]
    fn times_follow_the_specification() {
        assert_eq!(
            note(&[], &[], -1).timeout,
            Timeout::After(DEFAULT_TIMEOUT_MS)
        );
        assert_eq!(note(&[], &[], 0).timeout, Timeout::Never);
        assert_eq!(note(&[], &[], 1200).timeout, Timeout::After(1200));
        let critical = note(&[], &[("urgency", Value::U8(2))], 1200);
        assert_eq!(critical.urgency, Urgency::Critical);
        assert_eq!(critical.timeout, Timeout::Never);
    }

    #[test]
    fn reads_the_hints() {
        let read = note(
            &["default", "Open", "later", "Later", "odd"],
            &[
                ("urgency", Value::I32(0)),
                ("transient", Value::Bool(true)),
                ("desktop-entry", Value::from("org.gnome.Nautilus")),
                ("image-path", Value::from("/tmp/face.png")),
                ("omarchy-exec-argv", Value::from("[\"omarchy-update\"]")),
            ],
            -1,
        );
        assert_eq!(read.urgency, Urgency::Low);
        assert!(read.transient && !read.resident);
        assert_eq!(read.desktop_entry, "org.gnome.Nautilus");
        assert_eq!(read.image, "/tmp/face.png");
        assert_eq!(read.exec, vec!["omarchy-update".to_owned()]);
        assert_eq!(read.actions.len(), 2);
        assert_eq!(read.actions.first().map(|a| a.label.as_str()), Some("Open"));
    }

    #[test]
    fn refuses_a_command_that_is_an_option() {
        let read = note(
            &[],
            &[("omarchy-exec-argv", Value::from("[\"--help\"]"))],
            -1,
        );
        assert_eq!(read.exec.len(), 0);
        let read = note(&[], &[("omarchy-exec-argv", Value::from("not json"))], -1);
        assert_eq!(read.exec.len(), 0);
    }

    #[test]
    fn takes_pixels_only_when_they_add_up() {
        let picture = |stride: i32, bytes: usize| {
            Value::from((2i32, 2i32, stride, true, 8i32, 4i32, vec![0u8; bytes]))
        };
        assert!(
            note(&[], &[("image-data", picture(8, 16))], -1)
                .pixels
                .is_some()
        );
        assert!(
            note(&[], &[("image-data", picture(8, 15))], -1)
                .pixels
                .is_none()
        );
        assert!(
            note(&[], &[("image-data", picture(4, 16))], -1)
                .pixels
                .is_none()
        );
    }
}
