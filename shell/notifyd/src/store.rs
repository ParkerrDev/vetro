//! The notifications kept, newest first, and which of them are up as toasts.
//!
//! A toast stays up for its time (five seconds unless it names its own, until
//! put away when it is critical or names none), and its count stops while the
//! pointer is over the toasts. When its time is up the toast goes, and the
//! notification stays in the centre until it is dismissed there; one marked
//! transient was only ever a toast, and is closed. Do Not Disturb keeps toasts
//! down, but for Omarchy's own confirmations and a critical `notify-send`, as
//! Omarchy's notifications have it.
//!
//! Time is the caller's: milliseconds on a clock that only goes forward.
//! Every change says what the shell is to be told and what is to be said on
//! the bus, in an [`Outcome`].

use crate::note::{Note, Timeout, Urgency};

/// The most kept; past it the oldest is closed.
pub const CAPACITY: usize = 100;
/// The most brought back up at once by [`Store::reshow`].
pub const RESHOW: usize = 10;

/// Why a notification closed, as the specification numbers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Expired = 1,
    Dismissed = 2,
    Closed = 3,
    Undefined = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toast {
    Down,
    /// Up until it is put away.
    Pinned,
    /// Up, and going at `due`.
    Running {
        due: u64,
        total: u64,
    },
    /// Up, its count stopped with `left` to go.
    Held {
        left: u64,
        total: u64,
    },
}

impl Toast {
    #[must_use]
    pub fn up(self) -> bool {
        self != Self::Down
    }

    /// Up, left, total, running: what the shell draws its time bar from.
    #[must_use]
    pub fn clock(self, now: u64) -> (bool, u64, u64, bool) {
        match self {
            Self::Down => (false, 0, 0, false),
            Self::Pinned => (true, 0, 0, false),
            Self::Running { due, total } => (true, due.saturating_sub(now), total, true),
            Self::Held { left, total } => (true, left, total, false),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: u32,
    pub note: Note,
    /// The file written from the picture it brought in memory, if any.
    pub picture: String,
    /// When it came, in seconds since 1970.
    pub time: i64,
    pub toast: Toast,
}

/// What the shell is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// New or replaced: all of it.
    Note(u32),
    /// Its toast came up, went, or its count stopped or started.
    Toast(u32),
    Closed(u32),
    Dnd(bool),
}

/// What is said on the bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    Closed { id: u32, reason: Reason },
    Action { id: u32, key: String },
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub events: Vec<Event>,
    pub signals: Vec<Signal>,
    /// Picture files nothing shows any more.
    pub pictures: Vec<String>,
}

impl Outcome {
    fn add(&mut self, other: Self) {
        self.events.extend(other.events);
        self.signals.extend(other.signals);
        self.pictures.extend(other.pictures);
    }
}

/// Omarchy's rule for what Do Not Disturb lets through.
fn breaks_silence(note: &Note) -> bool {
    note.app == "omarchy-action" || (note.app == "notify-send" && note.urgency == Urgency::Critical)
}

/// Omarchy's rule for what is not worth keeping when it is silenced.
fn passing(note: &Note) -> bool {
    note.transient || note.app == "notify-send" || note.app == "omarchy-action"
}

#[derive(Debug, Default)]
pub struct Store {
    entries: Vec<Entry>,
    last_id: u32,
    holds: u32,
    dnd: bool,
}

impl Store {
    #[must_use]
    pub fn new(dnd: bool) -> Self {
        Self {
            dnd,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn dnd(&self) -> bool {
        self.dnd
    }

    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    #[must_use]
    pub fn get(&self, id: u32) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    fn at(&self, id: u32) -> Option<usize> {
        self.entries.iter().position(|entry| entry.id == id)
    }

    /// A toast raised now for this long.
    fn raise(&self, timeout: Timeout, now: u64) -> Toast {
        match timeout {
            Timeout::Never => Toast::Pinned,
            Timeout::After(total) if self.holds > 0 => Toast::Held { left: total, total },
            Timeout::After(total) => Toast::Running {
                due: now.saturating_add(total),
                total,
            },
        }
    }

    /// The next free id after the last one given, never 0. There are never
    /// more than [`CAPACITY`] in use, so one is found within that many tries.
    fn fresh_id(&mut self) -> u32 {
        let mut id = self.last_id;
        for _ in 0..=CAPACITY {
            id = id.wrapping_add(1).max(1);
            if self.at(id).is_none() {
                break;
            }
        }
        self.last_id = id;
        id
    }

    /// A notification arrives, new or in place of `replaces`: its id, and what
    /// to say. `now` is the clock's, `time` the wall clock's.
    pub fn notify(&mut self, note: Note, replaces: u32, now: u64, time: i64) -> (u32, Outcome) {
        let mut outcome = Outcome::default();
        let old = (replaces != 0).then(|| self.at(replaces)).flatten();
        let id = match (replaces, old) {
            (0, _) => self.fresh_id(),
            (id, _) => id,
        };
        if let Some(index) = old {
            if index < self.entries.len() {
                let gone = self.entries.remove(index);
                if !gone.picture.is_empty() {
                    outcome.pictures.push(gone.picture);
                }
            }
        }
        let silenced = self.dnd && !breaks_silence(&note);
        if silenced && passing(&note) {
            if old.is_some() {
                outcome.events.push(Event::Closed(id));
            }
            outcome.signals.push(Signal::Closed {
                id,
                reason: Reason::Expired,
            });
            return (id, outcome);
        }
        let toast = if silenced {
            Toast::Down
        } else {
            self.raise(note.timeout, now)
        };
        self.entries.insert(
            0,
            Entry {
                id,
                note,
                picture: String::new(),
                time,
                toast,
            },
        );
        outcome.events.push(Event::Note(id));
        if self.entries.len() > CAPACITY {
            if let Some(oldest) = self.entries.last().map(|entry| entry.id) {
                outcome.add(self.close(oldest, Reason::Undefined));
            }
        }
        (id, outcome)
    }

    /// The file a notification's picture was written to: the one it had
    /// before, if any, to be removed.
    pub fn attach_picture(&mut self, id: u32, path: String) -> Option<String> {
        let index = self.at(id)?;
        let entry = self.entries.get_mut(index)?;
        let old = std::mem::replace(&mut entry.picture, path);
        (!old.is_empty()).then_some(old)
    }

    pub fn close(&mut self, id: u32, reason: Reason) -> Outcome {
        let mut outcome = Outcome::default();
        let Some(index) = self.at(id) else {
            return outcome;
        };
        if index < self.entries.len() {
            let gone = self.entries.remove(index);
            if !gone.picture.is_empty() {
                outcome.pictures.push(gone.picture);
            }
        }
        outcome.events.push(Event::Closed(id));
        outcome.signals.push(Signal::Closed { id, reason });
        outcome
    }

    /// Its toast taken off the screen; the notification stays in the centre,
    /// unless it was only ever a toast.
    pub fn hide(&mut self, id: u32) -> Outcome {
        let Some(entry) = self.at(id).and_then(|index| self.entries.get_mut(index)) else {
            return Outcome::default();
        };
        if entry.note.transient {
            return self.close(id, Reason::Expired);
        }
        if !entry.toast.up() {
            return Outcome::default();
        }
        entry.toast = Toast::Down;
        Outcome {
            events: vec![Event::Toast(id)],
            ..Outcome::default()
        }
    }

    pub fn hide_all(&mut self) -> Outcome {
        let up: Vec<u32> = self.up_ids();
        let mut outcome = Outcome::default();
        for id in up {
            outcome.add(self.hide(id));
        }
        outcome
    }

    fn up_ids(&self) -> Vec<u32> {
        self.entries
            .iter()
            .filter(|entry| entry.toast.up())
            .map(|entry| entry.id)
            .collect()
    }

    /// One of its actions chosen: `key`, or with none named its default
    /// (keyed "default", or the first). It closes then, unless it is resident.
    pub fn invoke(&mut self, id: u32, key: Option<&str>) -> Outcome {
        let Some(entry) = self.get(id) else {
            return Outcome::default();
        };
        let actions = &entry.note.actions;
        let chosen = match key {
            Some(wanted) => actions.iter().find(|action| action.key == wanted),
            None => actions
                .iter()
                .find(|action| matches!(action.key.as_str(), "default" | "Default" | "0"))
                .or_else(|| actions.first()),
        };
        let Some(chosen) = chosen.map(|action| action.key.clone()) else {
            return Outcome::default();
        };
        let resident = entry.note.resident;
        let mut outcome = Outcome {
            signals: vec![Signal::Action { id, key: chosen }],
            ..Outcome::default()
        };
        outcome.add(if resident {
            self.hide(id)
        } else {
            self.close(id, Reason::Dismissed)
        });
        outcome
    }

    /// Every one dismissed.
    pub fn clear(&mut self) -> Outcome {
        let ids: Vec<u32> = self.entries.iter().map(|entry| entry.id).collect();
        let mut outcome = Outcome::default();
        for id in ids {
            outcome.add(self.close(id, Reason::Dismissed));
        }
        outcome
    }

    /// The newest [`RESHOW`] back up as toasts, their time begun again.
    pub fn reshow(&mut self, now: u64) -> Outcome {
        let mut outcome = Outcome::default();
        let raised: Vec<(u32, Toast)> = self
            .entries
            .iter()
            .take(RESHOW)
            .map(|entry| (entry.id, self.raise(entry.note.timeout, now)))
            .collect();
        for (id, toast) in raised {
            if let Some(entry) = self.at(id).and_then(|index| self.entries.get_mut(index)) {
                entry.toast = toast;
                outcome.events.push(Event::Toast(id));
            }
        }
        outcome
    }

    /// The pointer came over the toasts (on one screen of several, each
    /// telling it): every count stops.
    pub fn hold(&mut self, now: u64) -> Outcome {
        self.holds = self.holds.saturating_add(1);
        if self.holds > 1 {
            return Outcome::default();
        }
        self.retime(|toast| match toast {
            Toast::Running { due, total } => Some(Toast::Held {
                left: due.saturating_sub(now),
                total,
            }),
            _ => None,
        })
    }

    /// The pointer left them: once no screen holds them, the counts go on.
    pub fn release(&mut self, now: u64) -> Outcome {
        if self.holds == 0 {
            return Outcome::default();
        }
        self.holds = self.holds.saturating_sub(1);
        if self.holds > 0 {
            return Outcome::default();
        }
        self.retime(|toast| match toast {
            Toast::Held { left, total } => Some(Toast::Running {
                due: now.saturating_add(left),
                total,
            }),
            _ => None,
        })
    }

    fn retime(&mut self, change: impl Fn(Toast) -> Option<Toast>) -> Outcome {
        let mut outcome = Outcome::default();
        for entry in &mut self.entries {
            if let Some(next) = change(entry.toast) {
                entry.toast = next;
                outcome.events.push(Event::Toast(entry.id));
            }
        }
        outcome
    }

    /// Do Not Disturb on or off. On, every toast goes at once.
    pub fn set_dnd(&mut self, on: bool) -> Outcome {
        if self.dnd == on {
            return Outcome::default();
        }
        self.dnd = on;
        let mut outcome = Outcome {
            events: vec![Event::Dnd(on)],
            ..Outcome::default()
        };
        if on {
            outcome.add(self.hide_all());
        }
        outcome
    }

    /// The toasts whose time is up at `now` go.
    pub fn tick(&mut self, now: u64) -> Outcome {
        let due: Vec<u32> = self
            .entries
            .iter()
            .filter(|entry| matches!(entry.toast, Toast::Running { due, .. } if due <= now))
            .map(|entry| entry.id)
            .collect();
        let mut outcome = Outcome::default();
        for id in due {
            outcome.add(self.hide(id));
        }
        outcome
    }

    /// When the next toast goes, if one is counting.
    #[must_use]
    pub fn next_due(&self) -> Option<u64> {
        self.entries
            .iter()
            .filter_map(|entry| match entry.toast {
                Toast::Running { due, .. } => Some(due),
                _ => None,
            })
            .min()
    }
}

#[cfg(test)]
mod tests {
    use super::{CAPACITY, Event, Reason, Signal, Store, Toast};
    use crate::note::{Action, Note, Timeout, Urgency};

    fn note(app: &str, timeout: Timeout) -> Note {
        Note {
            app: app.to_owned(),
            summary: "Liquid glass".to_owned(),
            body: String::new(),
            icon: String::new(),
            image: String::new(),
            pixels: None,
            desktop_entry: String::new(),
            urgency: Urgency::Normal,
            actions: Vec::new(),
            exec: Vec::new(),
            resident: false,
            transient: false,
            timeout,
        }
    }

    #[test]
    fn a_toast_goes_when_its_time_is_up_and_the_note_stays() {
        let mut store = Store::new(false);
        let (id, outcome) = store.notify(note("Vetro", Timeout::After(5000)), 0, 100, 0);
        assert_eq!(id, 1);
        assert_eq!(outcome.events, vec![Event::Note(1)]);
        assert_eq!(store.next_due(), Some(5100));
        assert_eq!(store.tick(5099).events.len(), 0);
        assert_eq!(store.tick(5100).events, vec![Event::Toast(1)]);
        assert_eq!(store.get(1).map(|entry| entry.toast), Some(Toast::Down));
        assert_eq!(store.next_due(), None);
    }

    #[test]
    fn a_transient_one_closes_with_its_toast() {
        let mut store = Store::new(false);
        let mut passing = note("Vetro", Timeout::After(10));
        passing.transient = true;
        let (id, _) = store.notify(passing, 0, 0, 0);
        let outcome = store.tick(10);
        assert_eq!(
            outcome.signals,
            vec![Signal::Closed {
                id,
                reason: Reason::Expired
            }]
        );
        assert!(store.get(id).is_none());
    }

    #[test]
    fn the_pointer_stops_the_count() {
        let mut store = Store::new(false);
        store.notify(note("Vetro", Timeout::After(1000)), 0, 0, 0);
        store.hold(400);
        store.hold(450);
        assert_eq!(
            store.get(1).map(|entry| entry.toast),
            Some(Toast::Held {
                left: 600,
                total: 1000
            })
        );
        assert_eq!(store.release(900).events.len(), 0);
        store.release(1000);
        assert_eq!(store.next_due(), Some(1600));
    }

    #[test]
    fn a_replacement_keeps_its_id_and_a_named_one_is_used() {
        let mut store = Store::new(false);
        store.notify(note("Vetro", Timeout::Never), 0, 0, 0);
        let (id, outcome) = store.notify(note("Vetro", Timeout::Never), 1, 5, 0);
        assert_eq!((id, store.entries().len()), (1, 1));
        assert_eq!(outcome.events, vec![Event::Note(1)]);
        let (named, _) = store.notify(note("Vetro", Timeout::Never), 40, 5, 0);
        assert_eq!(named, 40);
        let (next, _) = store.notify(note("Vetro", Timeout::Never), 0, 5, 0);
        assert_eq!(next, 2);
    }

    #[test]
    fn do_not_disturb_keeps_toasts_down_but_for_omarchy() {
        let mut store = Store::new(true);
        let (kept, _) = store.notify(note("Slack", Timeout::After(5000)), 0, 0, 0);
        assert_eq!(store.get(kept).map(|entry| entry.toast), Some(Toast::Down));
        let (dropped, outcome) = store.notify(note("notify-send", Timeout::After(5000)), 0, 0, 0);
        assert!(store.get(dropped).is_none());
        assert_eq!(
            outcome.signals,
            vec![Signal::Closed {
                id: dropped,
                reason: Reason::Expired
            }]
        );
        let (shown, _) = store.notify(note("omarchy-action", Timeout::After(5000)), 0, 0, 0);
        assert!(store.get(shown).is_some_and(|entry| entry.toast.up()));
    }

    #[test]
    fn turning_do_not_disturb_on_takes_the_toasts_down() {
        let mut store = Store::new(false);
        store.notify(note("Vetro", Timeout::Never), 0, 0, 0);
        let outcome = store.set_dnd(true);
        assert_eq!(outcome.events, vec![Event::Dnd(true), Event::Toast(1)]);
        assert_eq!(store.set_dnd(true).events.len(), 0);
    }

    #[test]
    fn an_action_closes_it_unless_it_is_resident() {
        let mut store = Store::new(false);
        let mut asked = note("Vetro", Timeout::Never);
        asked.actions = vec![
            Action {
                key: "later".to_owned(),
                label: "Later".to_owned(),
            },
            Action {
                key: "default".to_owned(),
                label: "Open".to_owned(),
            },
        ];
        let mut resident = asked.clone();
        resident.resident = true;
        store.notify(asked, 0, 0, 0);
        let outcome = store.invoke(1, None);
        assert_eq!(
            outcome.signals.first(),
            Some(&Signal::Action {
                id: 1,
                key: "default".to_owned()
            })
        );
        assert!(store.get(1).is_none());
        store.notify(resident, 0, 0, 0);
        store.invoke(2, Some("later"));
        assert_eq!(store.get(2).map(|entry| entry.toast), Some(Toast::Down));
        assert_eq!(store.invoke(2, Some("nothing")).signals.len(), 0);
    }

    #[test]
    fn keeps_no_more_than_its_capacity() {
        let mut store = Store::new(false);
        for _ in 0..=CAPACITY {
            store.notify(note("Vetro", Timeout::Never), 0, 0, 0);
        }
        assert_eq!(store.entries().len(), CAPACITY);
        assert!(store.get(1).is_none());
        let outcome = store.clear();
        assert_eq!(outcome.signals.len(), CAPACITY);
        assert_eq!(store.entries().len(), 0);
    }

    #[test]
    fn reshow_raises_the_newest_again() {
        let mut store = Store::new(false);
        store.notify(note("Vetro", Timeout::After(10)), 0, 0, 0);
        store.tick(10);
        let outcome = store.reshow(20);
        assert_eq!(outcome.events, vec![Event::Toast(1)]);
        assert_eq!(store.next_due(), Some(30));
    }
}
