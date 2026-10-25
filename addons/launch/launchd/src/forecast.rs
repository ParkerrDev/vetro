//! The weather card's forecast, kept in `~/.cache/vetro/weather.json`: looked
//! up every ten minutes, a minute after a failure, and at once when the
//! settings change (`vetro-launch reload`).

use crate::{Hub, Paths, get, lock, say, write_atomic};
use serde_json::Value;
use std::fs;
use std::sync::PoisonError;
use std::time::Duration;
use vetro_launchd::clock;
use vetro_launchd::config::Settings;
use vetro_launchd::weather::{self, Place, Units};

const EVERY: u64 = 10 * 60;
const AFTER_FAILURE: u64 = 60;
/// With the card turned off, how often to see whether that changed.
const WHILE_OFF: u64 = 3600;

/// Reads a place out of an answer about this machine's address.
type Reader = fn(&Value) -> Option<Place>;

struct Wanted {
    on: bool,
    location: String,
    units: Units,
}

fn wanted(settings: &Settings) -> Wanted {
    let card = &settings.weather;
    Wanted {
        on: card.get("enabled").and_then(Value::as_bool) != Some(false),
        location: card
            .get("location")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_owned(),
        units: Units::named(card.get("units").and_then(Value::as_str).unwrap_or("")),
    }
}

fn get_json(url: &str) -> Result<Value, String> {
    let (status, body) = get(url).map_err(|error| format!("{error:#}"))?;
    if status != 200 {
        return Err(format!(
            "HTTP {status} from {}",
            url.split('?').next().unwrap_or(url)
        ));
    }
    serde_json::from_str(&body).map_err(|_| "an unreadable answer".to_owned())
}

/// Where the location is, or with none set, where this machine is.
fn find(location: &str) -> Result<Place, String> {
    if location.is_empty() {
        let readers: [(&str, Reader); 2] = [
            (weather::GEOJS, weather::from_geojs),
            (weather::IPINFO, weather::from_ipinfo),
        ];
        return readers
            .iter()
            .find_map(|(url, read)| get_json(url).ok().and_then(|answer| read(&answer)))
            .ok_or_else(|| "could not tell where this machine is".to_owned());
    }
    for (name, hint) in weather::attempts(location) {
        let answer = get_json(&weather::geocode_url(&name))?;
        if let Some(place) = weather::pick(&answer, &hint) {
            return Ok(place);
        }
    }
    Err(format!("could not find {location:?}"))
}

/// One look: the place (found again only when the location changed), then
/// its forecast.
fn look(want: &Wanted, found: &mut Option<(String, Place)>) -> Result<Value, String> {
    let place = match found {
        Some((location, place)) if *location == want.location => place.clone(),
        _ => {
            let place = find(&want.location)?;
            *found = Some((want.location.clone(), place.clone()));
            place
        }
    };
    let answer = get_json(&weather::forecast_url(&place, want.units))?;
    weather::forecast(&answer, &place, want.units, clock::now()).map_err(|error| error.to_string())
}

/// Meant never to return: it is the forecast's whole life.
pub fn keep(hub: &Hub, paths: &Paths) {
    let mut found: Option<(String, Place)> = None;
    let mut last: Option<Value> = fs::read_to_string(&paths.weather)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok());
    loop {
        let (card, asked) = {
            let shared = lock(hub);
            (wanted(&shared.settings), shared.asked)
        };
        let wait = if card.on {
            let (file, pause) = match look(&card, &mut found) {
                Ok(file) => (file, EVERY),
                Err(why) => {
                    say(&format!("weather: {why}"));
                    (weather::failed(last.as_ref(), &why), AFTER_FAILURE)
                }
            };
            if let Err(error) = write_atomic(&paths.weather, &file) {
                say(&format!("{error:#}"));
            }
            last = Some(file);
            pause
        } else {
            WHILE_OFF
        };
        // until it is due, or the settings are read again
        let shared = lock(hub);
        drop(
            hub.1
                .wait_timeout_while(shared, Duration::from_secs(wait), |shared| {
                    shared.asked == asked
                })
                .unwrap_or_else(PoisonError::into_inner),
        );
    }
}
