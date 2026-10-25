//! vetro-notifyd: the notification server (`org.freedesktop.Notifications`)
//! under Vetro's toasts and notification centre.
//!
//!   vetro-notifyd      the daemon
//!
//! The vetro.notifications shell plugin starts it and reads its output; it
//! ends when that plugin closes its input (the shell stopped, or restarted to
//! change themes), and the name on the bus goes with it. Pictures handed over
//! in memory are written to `$XDG_RUNTIME_DIR/vetro/notifications`, Do Not
//! Disturb to `~/.local/state/vetro/notifications.json`.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use vetro_notifyd::lines::{self, Command};
use vetro_notifyd::note::{Asked, Note, Pixels};
use vetro_notifyd::store::{Outcome, Reason, Signal, Store};
use vetro_notifyd::{picture, saved};
use zbus::blocking::{Connection, MessageIterator, connection};
use zbus::fdo::{RequestNameFlags, RequestNameReply};
use zbus::zvariant::OwnedValue;

const NAME: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";
/// A command line is never longer than this.
const MAX_LINE: u64 = 64 * 1024;
/// The longest the clock thread sleeps with no toast counting.
const MAX_SLEEP_MS: u64 = 3_600_000;
/// The most stale pictures cleared at start.
const MAX_STALE: usize = 4096;

struct Paths {
    saved: PathBuf,
    pictures: PathBuf,
}

/// What every thread shares: the store, the clock, the shell's output and
/// the signals waiting to go out on the bus.
struct Core {
    store: Mutex<Store>,
    wake: Condvar,
    start: Instant,
    out: Mutex<io::Stdout>,
    signals: Mutex<Sender<Signal>>,
    paths: Paths,
    serial: Mutex<u64>,
}

fn say(message: &str) {
    // stderr is the shell's log; if it is gone there is nowhere left to say it
    writeln!(io::stderr(), "vetro-notifyd: {message}").ok();
}

fn guard<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // with panic = "abort" no holder can die mid-change, so a poisoned lock holds whole data
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Paths {
    fn new() -> Result<Self> {
        let home = PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?);
        let state = std::env::var_os("XDG_STATE_HOME")
            .filter(|value| !value.is_empty())
            .map_or_else(|| home.join(".local/state"), PathBuf::from);
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .context("XDG_RUNTIME_DIR is not set")?;
        Ok(Self {
            saved: state.join("vetro").join("notifications.json"),
            pictures: runtime.join("vetro").join("notifications"),
        })
    }
}

impl Core {
    fn now(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// One line to the shell. Its end gone, there is no one left to serve.
    fn tell(&self, line: &serde_json::Value) {
        let mut out = guard(&self.out);
        if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
            std::process::exit(0);
        }
    }

    /// An outcome carried out: the shell told, the bus told, old pictures removed.
    fn carry_out(&self, store: &Store, outcome: Outcome, now: u64) {
        for event in &outcome.events {
            if let Some(line) = lines::event(store, event, now) {
                self.tell(&line);
            }
        }
        let sender = guard(&self.signals);
        for signal in outcome.signals {
            sender.send(signal).ok();
        }
        for file in outcome.pictures {
            fs::remove_file(file).ok();
        }
        self.wake.notify_all();
    }

    /// A picture handed over in memory, as a file: a new name each time, so
    /// the shell never shows one it loaded before under the same name.
    fn write_picture(&self, id: u32, pixels: &Pixels) -> Option<String> {
        let bytes = picture::png(pixels)
            .map_err(|error| say(&format!("a picture of notification {id}: {error}")))
            .ok()?;
        let serial = {
            let mut serial = guard(&self.serial);
            *serial = serial.wrapping_add(1);
            *serial
        };
        let path = self.paths.pictures.join(format!("{id}-{serial}.png"));
        fs::create_dir_all(&self.paths.pictures).ok()?;
        fs::write(&path, bytes)
            .map_err(|error| say(&format!("{}: {error}", path.display())))
            .ok()?;
        Some(path.to_string_lossy().into_owned())
    }

    fn save_dnd(&self, on: bool) {
        let file = &self.paths.saved;
        let written = file
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(file, saved::text(on)));
        if let Err(error) = written {
            say(&format!("{}: {error}", file.display()));
        }
    }
}

fn seconds_since_1970() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0))
}

struct Server {
    core: Arc<Core>,
}

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    #[expect(
        clippy::too_many_arguments,
        reason = "Notify's arguments are the specification's"
    )]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the bus hands its arguments over owned"
    )]
    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let mut note = Note::new(&Asked {
            app: &app_name,
            icon: &app_icon,
            summary: &summary,
            body: &body,
            actions: &actions,
            hints: &hints,
            expire_timeout,
        });
        let core = &self.core;
        let mut store = guard(&core.store);
        let now = core.now();
        let pixels = note.pixels.take();
        let (id, mut outcome) = store.notify(note, replaces_id, now, seconds_since_1970());
        if let Some(path) = pixels.and_then(|pixels| core.write_picture(id, &pixels)) {
            outcome.pictures.extend(store.attach_picture(id, path));
        }
        core.carry_out(&store, outcome, now);
        id
    }

    fn close_notification(&self, id: u32) {
        let core = &self.core;
        let mut store = guard(&core.store);
        let outcome = store.close(id, Reason::Closed);
        core.carry_out(&store, outcome, core.now());
    }

    #[expect(clippy::unused_self, reason = "the bus calls it on the server")]
    fn get_capabilities(&self) -> Vec<String> {
        [
            "actions",
            "body",
            "body-markup",
            "icon-static",
            "persistence",
        ]
        .map(str::to_owned)
        .to_vec()
    }

    #[zbus(out_args("name", "vendor", "version", "spec_version"))]
    #[expect(clippy::unused_self, reason = "the bus calls it on the server")]
    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "Vetro".to_owned(),
            "Vetro".to_owned(),
            env!("CARGO_PKG_VERSION").to_owned(),
            "1.2".to_owned(),
        )
    }
}

/// The signals, sent from a thread of their own so that no bus handler
/// waits on the bus. Ends when every sender has gone.
fn send_signals(connection: &Connection, signals: &Receiver<Signal>) {
    for signal in signals {
        let sent = match signal {
            Signal::Closed { id, reason } => connection.emit_signal(
                None::<()>,
                PATH,
                NAME,
                "NotificationClosed",
                &(id, reason as u32),
            ),
            Signal::Action { id, key } => {
                connection.emit_signal(None::<()>, PATH, NAME, "ActionInvoked", &(id, key))
            }
        };
        if let Err(error) = sent {
            say(&format!("a signal: {error}"));
        }
    }
}

/// Toasts go when their time is up. Meant never to return: it is the
/// clock's whole life.
fn keep_time(core: &Core) {
    let mut store = guard(&core.store);
    loop {
        let now = core.now();
        let outcome = store.tick(now);
        core.carry_out(&store, outcome, now);
        let wait = store.next_due().map_or(MAX_SLEEP_MS, |due| {
            due.saturating_sub(now).clamp(1, MAX_SLEEP_MS)
        });
        store = core
            .wake
            .wait_timeout(store, Duration::from_millis(wait))
            .unwrap_or_else(PoisonError::into_inner)
            .0;
    }
}

/// The shell is told when this daemon gets the name, or loses it to another
/// server. Meant never to end while the bus is there.
fn watch_name(connection: &Connection, core: &Core) -> Result<()> {
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender("org.freedesktop.DBus")?
        .interface("org.freedesktop.DBus")?
        .build();
    for message in MessageIterator::for_match_rule(rule, connection, Some(16))? {
        let message = message?;
        let header = message.header();
        let member = header.member().map_or("", |member| member.as_str());
        if !matches!(member, "NameAcquired" | "NameLost") {
            continue;
        }
        if message
            .body()
            .deserialize::<String>()
            .is_ok_and(|name| name == NAME)
        {
            core.tell(&lines::bus(member == "NameAcquired"));
        }
    }
    Ok(())
}

fn obey(core: &Core, command: Command) {
    let mut store = guard(&core.store);
    let now = core.now();
    let outcome = match command {
        Command::Dismiss(id) => store.close(id, Reason::Dismissed),
        Command::Hide(id) => store.hide(id),
        Command::Invoke(id, key) => store.invoke(id, key.as_deref()),
        Command::HideAll => store.hide_all(),
        Command::Clear => store.clear(),
        Command::Reshow => store.reshow(now),
        Command::Hold => store.hold(now),
        Command::Release => store.release(now),
        Command::Dnd(on) => {
            let outcome = store.set_dnd(on);
            if !outcome.events.is_empty() {
                core.save_dnd(on);
            }
            outcome
        }
        Command::Other(what) => {
            say(&format!("ignored {what:?}"));
            Outcome::default()
        }
    };
    core.carry_out(&store, outcome, now);
}

/// The shell's commands, until it closes the daemon's input.
fn listen(core: &Core) -> Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    // meant to end only when the shell closes our input
    loop {
        let mut line = String::new();
        let read = (&mut input).take(MAX_LINE).read_line(&mut line)?;
        if read == 0 {
            return Ok(());
        }
        obey(core, lines::command(line.trim()));
    }
}

/// Pictures a daemon before this one left behind.
fn clear_pictures(folder: &Path) {
    let Ok(listing) = fs::read_dir(folder) else {
        return;
    };
    for entry in listing.take(MAX_STALE).flatten() {
        fs::remove_file(entry.path()).ok();
    }
}

fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> Result<()> {
    std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(work)
        .map(drop)
        .with_context(|| format!("could not start the {name} thread"))
}

fn main() -> Result<()> {
    let paths = Paths::new()?;
    clear_pictures(&paths.pictures);
    let dnd = fs::read_to_string(&paths.saved).is_ok_and(|text| saved::dnd(&text));
    let (sender, receiver) = mpsc::channel();
    let core = Arc::new(Core {
        store: Mutex::new(Store::new(dnd)),
        wake: Condvar::new(),
        start: Instant::now(),
        out: Mutex::new(io::stdout()),
        signals: Mutex::new(sender),
        paths,
        serial: Mutex::new(0),
    });
    let connection = connection::Builder::session()?
        .serve_at(
            PATH,
            Server {
                core: Arc::clone(&core),
            },
        )?
        .build()
        .context("could not reach the session bus")?;
    // The newest server wins: one a shell restart left behind gives way.
    let flags = RequestNameFlags::ReplaceExisting | RequestNameFlags::AllowReplacement;
    let reply = connection.request_name_with_flags(NAME, flags)?;
    core.tell(&lines::bus(matches!(
        reply,
        RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner
    )));
    core.tell(
        &lines::event(
            &guard(&core.store),
            &vetro_notifyd::store::Event::Dnd(dnd),
            0,
        )
        .unwrap_or_default(),
    );
    let signals = connection.clone();
    spawn("signals", move || send_signals(&signals, &receiver))?;
    let clock = Arc::clone(&core);
    spawn("clock", move || keep_time(&clock))?;
    let (bus, told) = (connection.clone(), Arc::clone(&core));
    spawn("name", move || {
        if let Err(error) = watch_name(&bus, &told) {
            say(&format!("watching the name: {error:#}"));
        }
    })?;
    let result = listen(&core);
    clear_pictures(&core.paths.pictures);
    result
}
