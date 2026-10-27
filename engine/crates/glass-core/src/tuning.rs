//! Glass tuned by its user: the values of `~/.config/vetro/glass.conf`, which
//! the tuner (`apps/tuner`, `vetro tune`) writes and the engine reads, and the
//! presets the tuner offers. [`crate::quartz::Block::tuned`] turns them into the
//! block of Apple's shader.
//!
//! The file is lines of `key = value`, and a line that begins with `#` is a
//! comment. A key nobody knows and a value that does not read are skipped and
//! counted, so that a file a newer tuner wrote still loads, and every value is
//! held to its knob's range.
//!
//! The file holds the way windows move as well ([`Group::Motion`]): the pace of
//! every animation, the genie, the jelly and the wobble, which the engine runs
//! with [`crate::motion`].
//!
//! The knobs of the glass and their defaults follow what iOS 27 changed in Liquid Glass
//! (WWDC 2026, as the press and gamery.app reported it, and macOS
//! 27.0's `QuartzCore` as gpui-glass-demo decoded it): one slider from clear to
//! tinted, a darkened edge with brighter static highlights keyed from straight
//! above, no shadow, and more light of its own over a dark backdrop.

/// Where a knob sits in the tuner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// The body of the glass: tint, frost, colour.
    Glass,
    /// Its edge: the lens, the colour fringe, the dark edge.
    Edge,
    /// The light on it.
    Light,
    /// What shows through it.
    Behind,
    /// How windows move: the pace of every animation, the genie, the jelly
    /// and the wobble. A preset of the glass leaves these as they are.
    Motion,
}

impl Group {
    /// The number the C interface carries.
    #[must_use]
    pub fn code(self) -> u32 {
        match self {
            Self::Glass => 0,
            Self::Edge => 1,
            Self::Light => 2,
            Self::Behind => 3,
            Self::Motion => 4,
        }
    }
}

/// How the tuner shows a knob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// A slider over the knob's range.
    Slider,
    /// On or off.
    Switch,
    /// A colour: this knob is its red, and the two after it its green and blue.
    Colour,
    /// The green or blue of the colour before it: not shown by itself.
    Part,
}

impl Shown {
    /// The number the C interface carries.
    #[must_use]
    pub fn code(self) -> u32 {
        match self {
            Self::Slider => 0,
            Self::Switch => 1,
            Self::Colour => 2,
            Self::Part => 3,
        }
    }
}

/// Every value of the glass the user can set, in the order of [`KNOBS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id {
    Tint,
    Frost,
    Saturation,
    Lift,
    Adaptive,
    TintRed,
    TintGreen,
    TintBlue,
    AppleColour,
    Lens,
    Bend,
    BodyBend,
    Dispersion,
    DarkEdge,
    LightAngle,
    RimLight,
    SideLight,
    Hairline,
    Glow,
    GlowDepth,
    SeeWindows,
    Pace,
    GenieClose,
    GenieOpen,
    GeniePointer,
    GenieTime,
    Jelly,
    JellySquash,
    Wobble,
    WobbleMoves,
    WobbleTime,
    WobbleBounce,
}

/// How many knobs there are.
pub const COUNT: usize = 32;

impl Id {
    /// Every knob, in the order of its index.
    pub const ALL: [Self; COUNT] = [
        Self::Tint,
        Self::Frost,
        Self::Saturation,
        Self::Lift,
        Self::Adaptive,
        Self::TintRed,
        Self::TintGreen,
        Self::TintBlue,
        Self::AppleColour,
        Self::Lens,
        Self::Bend,
        Self::BodyBend,
        Self::Dispersion,
        Self::DarkEdge,
        Self::LightAngle,
        Self::RimLight,
        Self::SideLight,
        Self::Hairline,
        Self::Glow,
        Self::GlowDepth,
        Self::SeeWindows,
        Self::Pace,
        Self::GenieClose,
        Self::GenieOpen,
        Self::GeniePointer,
        Self::GenieTime,
        Self::Jelly,
        Self::JellySquash,
        Self::Wobble,
        Self::WobbleMoves,
        Self::WobbleTime,
        Self::WobbleBounce,
    ];

    /// Position in [`Self::ALL`], in [`KNOBS`] and in every array of values.
    #[must_use]
    pub fn index(self) -> usize {
        usize::from(self as u8)
    }

    /// The knob at this position.
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    /// The knob a line of the file names; the colour's green and blue have no key.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        KNOBS
            .iter()
            .position(|knob| !knob.key.is_empty() && knob.key == key)
            .and_then(Self::from_index)
    }

    /// What the tuner shows for it.
    #[must_use]
    pub fn knob(self) -> &'static Knob {
        KNOBS.get(self.index()).unwrap_or(&FALLBACK)
    }
}

/// One value of the glass as the tuner shows it, and the range it is held to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Knob {
    /// Its key in the file; empty for the parts of a colour.
    pub key: &'static str,
    /// What the tuner calls it.
    pub label: &'static str,
    /// One sentence on what it does.
    pub help: &'static str,
    /// The unit its value is shown in: `pt`, `%` (shown times 100), `x`, `deg`,
    /// `s`, or none.
    pub unit: &'static str,
    pub group: Group,
    pub shown: Shown,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub default: f32,
}

const FALLBACK: Knob = Knob {
    key: "",
    label: "",
    help: "",
    unit: "",
    group: Group::Glass,
    shown: Shown::Part,
    min: 0.0,
    max: 0.0,
    step: 0.0,
    default: 0.0,
};

/// A slider: `words` are its label, help and unit, `range` its `[min, max, step, default]`.
const fn slider(
    key: &'static str,
    words: [&'static str; 3],
    group: Group,
    range: [f32; 4],
) -> Knob {
    let [label, help, unit] = words;
    let [min, max, step, default] = range;
    Knob {
        key,
        label,
        help,
        unit,
        group,
        shown: Shown::Slider,
        min,
        max,
        step,
        default,
    }
}

/// A switch of the motion, on or off as it comes.
const fn motion_switch(key: &'static str, words: [&'static str; 2], on: bool) -> Knob {
    let [label, help] = words;
    Knob {
        key,
        label,
        help,
        group: Group::Motion,
        shown: Shown::Switch,
        max: 1.0,
        step: 1.0,
        default: if on { 1.0 } else { 0.0 },
        ..FALLBACK
    }
}

const fn part(default: f32) -> Knob {
    Knob {
        default,
        max: 1.0,
        step: 0.0,
        ..FALLBACK
    }
}

/// Every knob, by [`Id`]: `[min, max, step, default]` for each slider.
pub const KNOBS: [Knob; COUNT] = [
    slider(
        "tint",
        [
            "Clear to tinted",
            "How far the glass goes toward its tint colour. Clear lets what is behind through; tinted frosts it over.",
            "%",
        ],
        Group::Glass,
        [0.0, 1.0, 0.01, 0.30],
    ),
    slider(
        "frost",
        [
            "Frost",
            "How blurred what is behind looks, in points.",
            "pt",
        ],
        Group::Glass,
        [0.0, 60.0, 0.5, 9.0],
    ),
    slider(
        "colour",
        [
            "Colour",
            "How strongly the colours behind come through: 1 keeps them as they are.",
            "x",
        ],
        Group::Glass,
        [0.0, 2.5, 0.05, 1.25],
    ),
    slider(
        "lift",
        [
            "Inner light",
            "Light the glass gives off by itself, which lifts it off a dark wallpaper, as iOS 27 does.",
            "%",
        ],
        Group::Glass,
        [0.0, 1.0, 0.01, 0.15],
    ),
    slider(
        "adaptive",
        [
            "Darker over bright",
            "How much more the glass is tinted where what is behind is bright, so white text on it stays readable.",
            "%",
        ],
        Group::Glass,
        [0.0, 1.0, 0.01, 0.55],
    ),
    Knob {
        key: "tint_color",
        label: "Tint colour",
        help: "The colour the glass is tinted toward.",
        shown: Shown::Colour,
        max: 1.0,
        default: 0.14,
        ..FALLBACK
    },
    part(0.14),
    part(0.15),
    slider(
        "apple_colour",
        [
            "Apple's colour",
            "Blends in the colour matrix of Apple's own dark glass (platformContentGlass): deeper colours, darks lifted.",
            "%",
        ],
        Group::Glass,
        [0.0, 1.0, 0.01, 0.0],
    ),
    slider(
        "lens",
        [
            "Lens depth",
            "How far in from the edge the glass bends what is behind, in points.",
            "pt",
        ],
        Group::Edge,
        [0.0, 60.0, 0.5, 20.0],
    ),
    slider(
        "bend",
        [
            "Bend",
            "How strongly the lens pulls what is beyond the edge into the rim.",
            "x",
        ],
        Group::Edge,
        [0.0, 2.0, 0.05, 1.0],
    ),
    slider(
        "body_bend",
        [
            "Body bend",
            "How much the whole of a window's glass bends what is behind it, as a thick, gently curved sheet would: nothing at the centre, most toward the edges. At 0 only the lens at the edge bends.",
            "%",
        ],
        Group::Edge,
        [0.0, 1.0, 0.01, 0.0],
    ),
    slider(
        "dispersion",
        [
            "Dispersion",
            "The colour fringe where the edge bends light, in points.",
            "pt",
        ],
        Group::Edge,
        [0.0, 12.0, 0.25, 3.0],
    ),
    slider(
        "dark_edge",
        [
            "Dark edge",
            "The darkened edge iOS 27 puts around glass, darker on the sides.",
            "%",
        ],
        Group::Edge,
        [0.0, 1.0, 0.01, 0.5],
    ),
    slider(
        "light_angle",
        [
            "Light from",
            "Where the light comes from: 0 is straight above, as in iOS 27; -45 is the top left, as in iOS 26.",
            "deg",
        ],
        Group::Light,
        [-90.0, 90.0, 5.0, 0.0],
    ),
    slider(
        "rim_light",
        [
            "Rim light",
            "The bright, static highlight along the edges that face the light.",
            "x",
        ],
        Group::Light,
        [0.0, 2.0, 0.05, 1.0],
    ),
    slider(
        "side_light",
        [
            "Side light",
            "How much of the rim light the edges across the light catch.",
            "%",
        ],
        Group::Light,
        [0.0, 1.0, 0.05, 0.25],
    ),
    slider(
        "hairline",
        [
            "Hairline",
            "A thin bright line along the edge where it faces the light.",
            "%",
        ],
        Group::Light,
        [0.0, 1.5, 0.05, 0.45],
    ),
    slider(
        "glow",
        ["Edge glow", "Soft light just inside the lit edges.", "%"],
        Group::Light,
        [0.0, 0.6, 0.01, 0.10],
    ),
    slider(
        "glow_depth",
        [
            "Glow depth",
            "How far in the edge glow reaches, in points.",
            "pt",
        ],
        Group::Light,
        [1.0, 40.0, 0.5, 10.0],
    ),
    Knob {
        key: "see_windows",
        label: "Windows show through",
        help: "Windows under a glass window or panel show through it. The desktop's widgets never do.",
        group: Group::Behind,
        shown: Shown::Switch,
        max: 1.0,
        step: 1.0,
        default: 1.0,
        ..FALLBACK
    },
    slider(
        "pace",
        [
            "Pace",
            "How quickly windows, panels, menus and workspaces move: 1 is Vetro's own pace, 2 twice as quick, 0.5 half as quick. The panels follow once it is applied.",
            "x",
        ],
        Group::Motion,
        [0.5, 2.0, 0.05, 1.0],
    ),
    motion_switch(
        "genie_close",
        [
            "Genie on close",
            "A closing window pours itself into the bottom of the screen, as a genie into its bottle.",
        ],
        false,
    ),
    motion_switch(
        "genie_open",
        [
            "Genie on open",
            "A new window pours out of the bottom of the screen.",
        ],
        false,
    ),
    motion_switch(
        "genie_pointer",
        [
            "Toward the pointer",
            "The genie pours toward the pointer rather than the middle of the bottom edge.",
        ],
        false,
    ),
    slider(
        "genie_time",
        ["Genie time", "How long the genie takes, in seconds.", "s"],
        Group::Motion,
        [0.2, 1.5, 0.05, 0.55],
    ),
    motion_switch(
        "jelly",
        [
            "Jelly on open",
            "A new window lands with a soft squash and settles, like jelly.",
        ],
        false,
    ),
    slider(
        "jelly_squash",
        [
            "Squash",
            "How far a new window squashes before it settles.",
            "%",
        ],
        Group::Motion,
        [0.0, 0.3, 0.01, 0.08],
    ),
    motion_switch(
        "wobble",
        [
            "Wobbly windows",
            "A window dragged around bends behind the pointer and jiggles as it settles.",
        ],
        false,
    ),
    motion_switch(
        "wobble_moves",
        [
            "Wobble when tiled",
            "Windows the layout moves or resizes wobble as well.",
        ],
        false,
    ),
    slider(
        "wobble_time",
        [
            "Wobble time",
            "How long a wobbling window takes to settle, in seconds.",
            "s",
        ],
        Group::Motion,
        [0.2, 1.2, 0.05, 0.55],
    ),
    slider(
        "wobble_bounce",
        [
            "Jiggle",
            "How far a wobbling window swings past where it rests: at 0 it settles without a swing.",
            "%",
        ],
        Group::Motion,
        [0.0, 0.9, 0.05, 0.55],
    ),
];

/// The most lines of a file that are read: a tuning is a few dozen.
pub const MAX_LINES: usize = 512;

/// A whole tuning: one value for every knob, each within its range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    values: [f32; COUNT],
}

impl Default for Tuning {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Tuning {
    /// The glass as it comes: the Author's pick.
    pub const DEFAULT: Self = Self {
        values: changed(defaults(), AUTHORS_PICK),
    };

    /// Every knob at its own default: iOS 27's glass, which each preset
    /// changes from.
    pub const IOS27: Self = Self { values: defaults() };

    /// One knob's value.
    #[must_use]
    pub fn get(&self, id: Id) -> f32 {
        self.values
            .get(id.index())
            .copied()
            .unwrap_or(id.knob().default)
    }

    /// Sets one knob, held to its range; a value that is not a number is ignored.
    pub fn set(&mut self, id: Id, value: f32) {
        let knob = id.knob();
        if !value.is_finite() {
            return;
        }
        let held = value.max(knob.min).min(knob.max);
        let held = if knob.shown == Shown::Switch {
            held.round()
        } else {
            held
        };
        if let Some(slot) = self.values.get_mut(id.index()) {
            *slot = held;
        }
    }

    /// Whether a switch is on.
    #[must_use]
    pub fn is_on(&self, id: Id) -> bool {
        self.get(id) >= 0.5
    }

    /// The tint colour, red, green and blue from 0 to 1.
    #[must_use]
    pub fn tint_colour(&self) -> [f32; 3] {
        [
            self.get(Id::TintRed),
            self.get(Id::TintGreen),
            self.get(Id::TintBlue),
        ]
    }

    /// Every value, in the order of [`KNOBS`].
    #[must_use]
    pub fn values(&self) -> [f32; COUNT] {
        self.values
    }

    /// This tuning with the motion of `other`: a preset changes the glass, and
    /// the way windows move stays as the user left it.
    #[must_use]
    pub fn with_motion_of(mut self, other: &Self) -> Self {
        for id in Id::ALL {
            if id.knob().group == Group::Motion {
                self.set(id, other.get(id));
            }
        }
        self
    }

    /// A tuning from values in the order of [`KNOBS`]: each held to its range,
    /// and a value missing or not a number left at its default.
    #[must_use]
    pub fn from_values(values: &[f32]) -> Self {
        let mut tuning = Self::DEFAULT;
        for (id, value) in Id::ALL.iter().zip(values) {
            tuning.set(*id, *value);
        }
        tuning
    }
}

#[expect(
    clippy::indexing_slicing,
    reason = "a const fn has no iterators; `at` stays below COUNT, the length of both arrays"
)]
const fn defaults() -> [f32; COUNT] {
    let mut values = [0.0_f32; COUNT];
    let mut at = 0;
    while at < COUNT {
        values[at] = KNOBS[at].default;
        at = at.saturating_add(1);
    }
    values
}

/// `values` with these changes made: a preset's, at compile time. The values
/// are the presets' own, in range (the tests hold them to it).
#[expect(
    clippy::indexing_slicing,
    reason = "a const fn has no iterators; `at` stays below the length of `changes`, and an Id's index below COUNT"
)]
const fn changed(mut values: [f32; COUNT], changes: &[(Id, f32)]) -> [f32; COUNT] {
    let mut at = 0;
    while at < changes.len() {
        let (id, value) = changes[at];
        values[id as usize] = value;
        at = at.saturating_add(1);
    }
    values
}

/// Reads a file's text over the defaults. Answers the tuning and how many lines
/// were skipped because their key or value did not read.
#[must_use]
pub fn parse(text: &str) -> (Tuning, usize) {
    let mut tuning = Tuning::DEFAULT;
    let mut skipped = 0_usize;
    for line in text.lines().take(MAX_LINES) {
        if !apply_line(&mut tuning, line) {
            skipped = skipped.saturating_add(1);
        }
    }
    (tuning, skipped)
}

/// Applies one line; false when it is neither a setting nor blank nor a comment.
fn apply_line(tuning: &mut Tuning, line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return true;
    }
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    let (key, value) = (key.trim(), value.trim());
    let Some(id) = Id::from_key(key) else {
        return false;
    };
    match id.knob().shown {
        Shown::Colour => match read_colour(value) {
            Some([red, green, blue]) => {
                tuning.set(Id::TintRed, red);
                tuning.set(Id::TintGreen, green);
                tuning.set(Id::TintBlue, blue);
                true
            }
            None => false,
        },
        Shown::Switch => read_switch(value).is_some_and(|on| {
            tuning.set(id, if on { 1.0 } else { 0.0 });
            true
        }),
        Shown::Slider => read_number(value).is_some_and(|number| {
            tuning.set(id, number);
            true
        }),
        Shown::Part => false,
    }
}

fn read_number(text: &str) -> Option<f32> {
    text.parse::<f32>().ok().filter(|value| value.is_finite())
}

fn read_switch(text: &str) -> Option<bool> {
    match text.to_ascii_lowercase().as_str() {
        "1" | "on" | "yes" | "true" => Some(true),
        "0" | "off" | "no" | "false" => Some(false),
        _ => None,
    }
}

/// `#rrggbb` as three values from 0 to 1.
fn read_colour(text: &str) -> Option<[f32; 3]> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let channel = |at: usize| -> Option<f32> {
        let pair = hex.get(at..at.checked_add(2)?)?;
        u8::from_str_radix(pair, 16)
            .ok()
            .map(|byte| f32::from(byte) / 255.0)
    };
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// The file's text for a tuning: every knob, in the order of [`KNOBS`].
#[must_use]
pub fn format(tuning: &Tuning) -> String {
    let mut out = String::from("# Vetro's glass, as the tuner left it (vetro tune).\n");
    for (id, knob) in Id::ALL.iter().zip(KNOBS.iter()) {
        let line = match knob.shown {
            Shown::Part => continue,
            Shown::Colour => format!("{} = {}\n", knob.key, colour_text(tuning.tint_colour())),
            Shown::Switch => format!(
                "{} = {}\n",
                knob.key,
                if tuning.is_on(*id) { "on" } else { "off" }
            ),
            Shown::Slider => format!(
                "{} = {:.*}\n",
                knob.key,
                decimals(knob.step),
                tuning.get(*id)
            ),
        };
        out.push_str(&line);
    }
    out
}

/// Enough decimals to show a value to its knob's step.
fn decimals(step: f32) -> usize {
    if step >= 1.0 {
        0
    } else if step >= 0.1 {
        1
    } else if step >= 0.01 {
        2
    } else {
        3
    }
}

fn colour_text(rgb: [f32; 3]) -> String {
    let [red, green, blue] = rgb.map(|value| crate::num::unit_to_byte(f64::from(value)));
    format!("#{red:02x}{green:02x}{blue:02x}")
}

/// A named starting point the tuner offers: its changes from iOS 27
/// ([`Tuning::IOS27`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    /// Its name for `vetro tune --preset` and the file.
    pub name: &'static str,
    /// What the tuner calls it.
    pub label: &'static str,
    pub changes: &'static [(Id, f32)],
}

// Where it leaves iOS 27 (2026-10-22: "less cloudy/white blur and a slightly
// darker blur instead so it appears less milky"): no inner light, which is
// a white veil over the whole face; a darker, cooler tint colour, a little
// less of it; a frost just above iOS 27's, so that the bend reads through
// it, with the colour lifted to make up for the darker tint; less glow
// inside the edge; and the whole of a window bending gently ("the inside
// is just blur"), with a slightly deeper lens at the edge to meet it.
// Chosen from lab pictures against the user's own tuning of that evening.
// The glass comes as this.
const AUTHORS_PICK: &[(Id, f32)] = &[
    (Id::Lift, 0.0),
    (Id::Tint, 0.26),
    (Id::TintRed, 0.08),
    (Id::TintGreen, 0.085),
    (Id::TintBlue, 0.10),
    (Id::Frost, 11.0),
    (Id::Saturation, 1.35),
    (Id::Glow, 0.06),
    (Id::BodyBend, 0.6),
    (Id::Lens, 24.0),
    (Id::Bend, 1.1),
];

/// Every preset, the default first.
pub const PRESETS: [Preset; 8] = [
    Preset {
        name: "authors-pick",
        label: "Author's pick",
        changes: AUTHORS_PICK,
    },
    Preset {
        name: "ios27",
        label: "iOS 27",
        changes: &[],
    },
    Preset {
        name: "ultra-clear",
        label: "Ultra clear",
        changes: &[
            (Id::Tint, 0.04),
            (Id::Frost, 2.0),
            (Id::Adaptive, 0.35),
            (Id::Lift, 0.08),
        ],
    },
    Preset {
        name: "clear",
        label: "Clear",
        changes: &[(Id::Tint, 0.15), (Id::Frost, 5.0), (Id::Adaptive, 0.45)],
    },
    Preset {
        name: "tinted",
        label: "Tinted",
        changes: &[
            (Id::Tint, 0.55),
            (Id::Frost, 16.0),
            (Id::Adaptive, 0.35),
            (Id::Lift, 0.2),
        ],
    },
    Preset {
        name: "fully-tinted",
        label: "Fully tinted",
        changes: &[
            (Id::Tint, 0.85),
            (Id::Frost, 30.0),
            (Id::Adaptive, 0.1),
            (Id::Lift, 0.25),
        ],
    },
    Preset {
        name: "ios26",
        label: "iOS 26",
        changes: &[
            (Id::LightAngle, -45.0),
            (Id::DarkEdge, 0.0),
            (Id::Frost, 4.0),
            (Id::Tint, 0.12),
            (Id::Lens, 24.0),
            (Id::Bend, 1.2),
            (Id::Dispersion, 4.0),
            (Id::RimLight, 1.2),
            (Id::SideLight, 0.5),
            (Id::Hairline, 0.6),
            (Id::Lift, 0.05),
        ],
    },
    // The lens of the evening of 2026-10-18 (quartz::Kind::Lens), as near as
    // these knobs come: Apple's colour, a dark tint, a 28-point bezel.
    Preset {
        name: "lens",
        label: "Lens (Oct 18)",
        changes: &[
            (Id::AppleColour, 1.0),
            (Id::Frost, 7.0),
            (Id::Tint, 0.30),
            (Id::Adaptive, 0.5),
            (Id::TintRed, 0.045),
            (Id::TintGreen, 0.05),
            (Id::TintBlue, 0.06),
            (Id::Saturation, 1.3),
            (Id::Lift, 0.0),
            (Id::Lens, 28.0),
            (Id::Dispersion, 4.0),
            (Id::DarkEdge, 0.0),
            (Id::SideLight, 0.5),
            (Id::Hairline, 0.6),
            (Id::Glow, 0.16),
        ],
    },
];

impl Preset {
    /// The preset's tuning.
    #[must_use]
    pub fn tuning(&self) -> Tuning {
        let mut tuning = Tuning::IOS27;
        for (id, value) in self.changes {
            tuning.set(*id, *value);
        }
        tuning
    }

    /// The preset of this name.
    #[must_use]
    pub fn named(name: &str) -> Option<&'static Self> {
        PRESETS.iter().find(|preset| preset.name == name)
    }
}

#[cfg(test)]
mod tests;
