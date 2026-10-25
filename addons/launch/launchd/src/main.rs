//! vetro-launchd: keeps `~/.cache/vetro/launches.json` (and the login screen's
//! copy in `/var/lib/vetro`) and the weather card's `~/.cache/vetro/weather.json`
//! up to date, and answers `vetro-launch` on `$XDG_RUNTIME_DIR/vetro/launchd.sock`.
//!
//!   vetro-launchd                                 the daemon (the user service)
//!   vetro-launchd fetch [upcoming|previous] [N]   print launches and exit
//!
//! It sleeps until the next fetch falls due (a quarter of an hour apart, five
//! minutes near a launch) or a request comes: nothing wakes it in between.

mod forecast;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use vetro_launchd::cache::Cache;
use vetro_launchd::config::{self, Settings};
use vetro_launchd::library::{self, Kind};
use vetro_launchd::plan::{self, Due};
use vetro_launchd::state::{self, Request};
use vetro_launchd::{clock, launch};

const USER_AGENT: &str = "vetro-launchd/2.0 (personal desktop launch card)";
/// The longest sleep: a clock that jumps (a suspend) is noticed at least this often.
const MAX_SLEEP: i64 = 6 * 3600;
/// After a failed fetch, the next try waits this long.
const RETRY_AFTER: i64 = 120;
/// A request line is never longer than this.
const MAX_REQUEST: u64 = 64 * 1024;
const RECENT_COUNT: i64 = 5;

struct Paths {
    settings: PathBuf,
    old_settings: PathBuf,
    cache: PathBuf,
    weather: PathBuf,
    state: PathBuf,
    socket: PathBuf,
    shared: PathBuf,
    home: PathBuf,
}

struct Shared {
    settings: Settings,
    cache: Cache,
    refresh: bool,
    /// Counts the times a fresh look was asked for, by a refresh or by the
    /// settings being read again: the forecast looks again when it moves.
    asked: u64,
}

type Hub = Arc<(Mutex<Shared>, Condvar)>;

fn say(message: &str) {
    // stderr is the journal; if it is gone there is nowhere left to say it
    writeln!(io::stderr(), "{message}").ok();
}

fn xdg(variable: &str, home: &Path, fallback: &str) -> PathBuf {
    std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map_or_else(|| home.join(fallback), PathBuf::from)
}

impl Paths {
    fn new() -> Result<Self> {
        let home = PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?);
        let config = xdg("XDG_CONFIG_HOME", &home, ".config").join("vetro");
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .context("XDG_RUNTIME_DIR is not set")?
            .join("vetro");
        Ok(Self {
            settings: config.join("launches.toml"),
            old_settings: config.join("wallpaper.toml"),
            cache: xdg("XDG_CACHE_HOME", &home, ".cache")
                .join("vetro")
                .join("launches.json"),
            weather: xdg("XDG_CACHE_HOME", &home, ".cache")
                .join("vetro")
                .join("weather.json"),
            state: runtime.join("state.json"),
            socket: runtime.join("launchd.sock"),
            // where the login screen reads it; tests point it elsewhere
            shared: std::env::var_os("VETRO_SHARED_DIR")
                .map_or_else(|| PathBuf::from("/var/lib/vetro"), PathBuf::from),
            home,
        })
    }
}

fn lock(hub: &Hub) -> MutexGuard<'_, Shared> {
    // with panic = "abort" no holder can die mid-change, so a poisoned lock holds whole data
    hub.0.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The settings file, or the one the video wallpaper used before it, or the defaults.
fn read_settings(paths: &Paths) -> Settings {
    let file = [&paths.settings, &paths.old_settings]
        .into_iter()
        .find(|path| path.exists());
    let Some(file) = file else {
        return Settings::default();
    };
    match fs::read_to_string(file)
        .map_err(|error| error.to_string())
        .and_then(|text| config::parse(&text).map_err(|error| error.to_string()))
    {
        Ok(settings) => settings,
        Err(error) => {
            say(&format!("{}: {error}; using the defaults", file.display()));
            Settings::default()
        }
    }
}

fn read_cache(path: &Path) -> Cache {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .map_or_else(Cache::default, |value| Cache::from_json(&value))
}

/// Written whole and renamed into place, so a reader never sees half of it.
fn write_atomic(path: &Path, value: &Value) -> Result<()> {
    let folder = path.parent().context("a file with no folder")?;
    fs::create_dir_all(folder)?;
    let name = path
        .file_name()
        .context("a file with no name")?
        .to_string_lossy();
    let temporary = folder.join(format!(".{name}.{}", std::process::id()));
    let text = serde_json::to_string_pretty(value)?;
    fs::write(&temporary, format!("{text}\n"))?;
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o644))?;
    fs::rename(&temporary, path).with_context(|| format!("could not write {}", path.display()))
}

/// One GET with curl: the HTTP status and the body.
fn get(url: &str) -> Result<(u16, String)> {
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--max-time",
            "30",
            "--user-agent",
            USER_AGENT,
        ])
        .args([
            "--header",
            "Accept: application/json",
            "--write-out",
            "\n%{http_code}",
            url,
        ])
        .output()
        .context("curl could not be run")?;
    if !output.status.success() {
        bail!("curl: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let (body, code) = text.rsplit_once('\n').context("curl wrote no status")?;
    Ok((
        code.trim().parse().context("curl wrote no status")?,
        body.to_owned(),
    ))
}

enum Failure {
    Throttled(i64),
    Other(String),
}

fn fetch(settings: &Settings, kind: Kind, limit: i64) -> Result<Vec<Value>, Failure> {
    let (status, body) = get(&library::url(settings, kind, limit))
        .map_err(|error| Failure::Other(format!("{error:#}")))?;
    library::launches(status, &body).map_err(|error| match error {
        library::Error::Throttled(seconds) => Failure::Throttled(seconds),
        other => Failure::Other(other.to_string()),
    })
}

fn fetch_due(
    settings: &Settings,
    due: Due,
) -> (Option<Vec<Value>>, Option<Vec<Value>>, Option<Failure>) {
    let mut failure = None;
    let mut take = |wanted: bool, kind: Kind, limit: i64| -> Option<Vec<Value>> {
        if !wanted || failure.is_some() {
            return None;
        }
        fetch(settings, kind, limit)
            .map_err(|error| failure = Some(error))
            .ok()
    };
    let upcoming = take(due.upcoming, Kind::Upcoming, settings.upcoming_count);
    let previous = take(due.previous, Kind::Previous, RECENT_COUNT);
    (upcoming, previous, failure)
}

fn apply(
    cache: &mut Cache,
    fetched: (Option<Vec<Value>>, Option<Vec<Value>>, Option<Failure>),
    now: i64,
) {
    let (upcoming, previous, failure) = fetched;
    if let Some(list) = upcoming {
        cache.upcoming = list;
        cache.fetched_upcoming = now;
    }
    if let Some(list) = previous {
        cache.previous = list;
        cache.fetched_previous = now;
    }
    cache.error = match failure {
        None => None,
        Some(Failure::Throttled(seconds)) => {
            cache.throttled_until = now.saturating_add(seconds).saturating_add(5);
            Some(format!(
                "Launch Library rate limit, retrying at {}",
                clock::format(cache.throttled_until).unwrap_or_default()
            ))
        }
        Some(Failure::Other(error)) => {
            cache.retry_at = now.saturating_add(RETRY_AFTER);
            Some(error)
        }
    };
}

/// The files the cards read, after a fetch or a change of settings.
fn publish(paths: &Paths, settings: &Settings, cache: &Cache) {
    let now = clock::now();
    let file = cache.to_json(now);
    for (path, value) in [
        (&paths.cache, &file),
        (&paths.state, &state::state(settings, cache, now)),
    ] {
        if let Err(error) = write_atomic(path, value) {
            say(&format!("{error:#}"));
        }
    }
    // the login screen's copy, where its installer made a place for it
    if paths.shared.is_dir() {
        write_atomic(&paths.shared.join("launches.json"), &file).ok();
    }
}

/// The login screen shows the user's picture from its own folder: copied
/// there when it is new or has changed.
fn share_avatar(paths: &Paths) {
    let picture = [".face", "avatar.jpg"]
        .into_iter()
        .map(|name| paths.home.join(name))
        .find(|path| path.is_file());
    let (Some(picture), true) = (picture, paths.shared.is_dir()) else {
        return;
    };
    let target = paths.shared.join("avatar.jpg");
    let same = match (fs::metadata(&picture), fs::metadata(&target)) {
        (Ok(from), Ok(to)) => from.len() == to.len() && from.modified().ok() <= to.modified().ok(),
        _ => false,
    };
    if !same && fs::copy(&picture, &target).is_ok() {
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).ok();
    }
}

/// Sleeps until a fetch falls due or a request asks for one, and says which.
/// Meant never to return otherwise: it is the fetcher's whole life.
fn wait_until_due(hub: &Hub) -> (Settings, Due) {
    let mut shared = lock(hub);
    loop {
        if shared.refresh {
            shared.refresh = false;
            shared.cache.fetched_upcoming = 0;
            shared.cache.fetched_previous = 0;
            shared.cache.retry_at = 0;
        }
        let now = clock::now();
        let due = plan::due(&shared.cache, &shared.settings, now);
        if due.any() {
            return (shared.settings.clone(), due);
        }
        let wait = plan::wake_at(&shared.cache, &shared.settings, now)
            .saturating_sub(now)
            .clamp(1, MAX_SLEEP);
        let seconds = u64::try_from(wait).unwrap_or(1);
        shared = hub
            .1
            .wait_timeout(shared, Duration::from_secs(seconds))
            .unwrap_or_else(PoisonError::into_inner)
            .0;
    }
}

fn fetcher(hub: &Hub, paths: &Paths) {
    loop {
        let (settings, due) = wait_until_due(hub);
        let fetched = fetch_due(&settings, due);
        let mut shared = lock(hub);
        apply(&mut shared.cache, fetched, clock::now());
        if let Some(error) = &shared.cache.error {
            say(&format!("launch data: {error}"));
        }
        publish(paths, &shared.settings, &shared.cache);
    }
}

fn answer(hub: &Hub, paths: &Paths, stream: &UnixStream) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut line = String::new();
    BufReader::new(stream.take(MAX_REQUEST)).read_line(&mut line)?;
    let request = state::request(&line);
    let reply = {
        let mut shared = lock(hub);
        match &request {
            Request::Status => state::status(&shared.settings, &shared.cache, clock::now()),
            Request::Refresh => {
                shared.refresh = true;
                shared.asked = shared.asked.wrapping_add(1);
                hub.1.notify_all();
                state::reply(&request)
            }
            Request::Reload => {
                shared.settings = read_settings(paths);
                shared.asked = shared.asked.wrapping_add(1);
                publish(paths, &shared.settings, &shared.cache);
                hub.1.notify_all();
                state::reply(&request)
            }
            Request::Removed(_) | Request::Unknown(_) => state::reply(&request),
        }
    };
    let mut writer = stream;
    writeln!(writer, "{reply}")?;
    Ok(())
}

fn serve(hub: &Hub, paths: &Paths) -> Result<()> {
    if let Some(folder) = paths.socket.parent() {
        fs::create_dir_all(folder)?;
    }
    // a socket left by a daemon that did not get to clean up
    match fs::remove_file(&paths.socket) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => {
            say(&format!("{}: {error}", paths.socket.display()));
        }
        _ => {}
    }
    let listener = UnixListener::bind(&paths.socket)
        .with_context(|| format!("could not listen on {}", paths.socket.display()))?;
    // meant never to end: the daemon answers until it is stopped
    for stream in listener.incoming() {
        let result = stream
            .map_err(anyhow::Error::from)
            .and_then(|stream| answer(hub, paths, &stream));
        if let Err(error) = result {
            say(&format!("control socket: {error:#}"));
        }
    }
    Ok(())
}

fn daemon() -> Result<()> {
    let paths = Arc::new(Paths::new()?);
    let settings = read_settings(&paths);
    let cache = read_cache(&paths.cache);
    publish(&paths, &settings, &cache);
    share_avatar(&paths);
    let hub: Hub = Arc::new((
        Mutex::new(Shared {
            settings,
            cache,
            refresh: false,
            asked: 0,
        }),
        Condvar::new(),
    ));
    say(&format!(
        "vetro-launchd {} started",
        env!("CARGO_PKG_VERSION")
    ));
    let (worker_hub, worker_paths) = (Arc::clone(&hub), Arc::clone(&paths));
    std::thread::Builder::new()
        .name("fetch".to_owned())
        .spawn(move || fetcher(&worker_hub, &worker_paths))?;
    let (weather_hub, weather_paths) = (Arc::clone(&hub), Arc::clone(&paths));
    std::thread::Builder::new()
        .name("weather".to_owned())
        .spawn(move || forecast::keep(&weather_hub, &weather_paths))?;
    serve(&hub, &paths)
}

fn print_launches(kind: Kind, limit: i64) -> Result<()> {
    let paths = Paths::new()?;
    let launches = fetch(&read_settings(&paths), kind, limit).map_err(|failure| match failure {
        Failure::Throttled(seconds) => {
            anyhow::anyhow!("Launch Library rate limit: try again in {seconds}s")
        }
        Failure::Other(error) => anyhow::anyhow!(error),
    })?;
    let mut out = io::stdout().lock();
    for found in &launches {
        let field = |key: &str| {
            found
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        };
        let net = launch::net(found)
            .and_then(clock::format)
            .unwrap_or_default();
        writeln!(
            out,
            "{net}  {:<10} {:<3} {}",
            field("status"),
            field("rocket_short"),
            field("title")
        )?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).take(4).collect();
    match args.first().map(String::as_str) {
        None | Some("run") => daemon(),
        Some("fetch") => {
            let kind = if args.get(1).map(String::as_str) == Some("previous") {
                Kind::Previous
            } else {
                Kind::Upcoming
            };
            let limit = args
                .get(2)
                .and_then(|n| n.parse().ok())
                .filter(|n| (1..=50).contains(n))
                .unwrap_or(5);
            print_launches(kind, limit)
        }
        Some(other) => {
            bail!("vetro-launchd: no command {other:?} (run, fetch [upcoming|previous] [N])")
        }
    }
}
