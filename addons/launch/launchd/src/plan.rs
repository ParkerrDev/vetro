//! When to ask Launch Library again. The daemon sleeps until the time
//! `wake_at` gives and asks then for what `due` says, so nothing wakes it in
//! between: no tick, no polling.

use crate::cache::{Cache, postroll};
use crate::config::Settings;
use crate::launch;
use serde_json::Value;

/// The recent launches are looked at again this long after each lift-off,
/// so that the time it really lifted off lands.
const AFTER_LAUNCH: i64 = 30 * 60;
/// And at least this often.
const PREVIOUS_EVERY: i64 = 6 * 3600;
/// Only the first few launches decide whether one is near.
const NEAR_CANDIDATES: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Due {
    pub upcoming: bool,
    pub previous: bool,
}

impl Due {
    #[must_use]
    pub fn any(self) -> bool {
        self.upcoming || self.previous
    }
}

fn minutes(count: i64) -> i64 {
    count.saturating_mul(60)
}

fn near(candidate: &Value, settings: &Settings, now: i64) -> bool {
    let flying = candidate.get("webcast_live").and_then(Value::as_bool) == Some(true)
        || candidate.get("status").and_then(Value::as_str) == Some("In Flight");
    let close = launch::net(candidate).is_some_and(|net| {
        let ahead = net.saturating_sub(now);
        ahead >= postroll(candidate).saturating_neg()
            && ahead <= minutes(settings.near_launch_minutes)
    });
    flying || close
}

/// Whether a launch is close (or flying), when the list is looked at more often.
#[must_use]
pub fn near_launch(cache: &Cache, settings: &Settings, now: i64) -> bool {
    cache
        .upcoming
        .iter()
        .take(NEAR_CANDIDATES)
        .any(|candidate| near(candidate, settings, now))
}

fn interval(cache: &Cache, settings: &Settings, now: i64) -> i64 {
    if near_launch(cache, settings, now) {
        minutes(settings.refresh_minutes_near_launch)
    } else {
        minutes(settings.refresh_minutes)
    }
}

/// The latest lift-off already past among the upcoming launches, if any.
fn recent_launch(cache: &Cache, now: i64) -> Option<i64> {
    cache
        .upcoming
        .iter()
        .filter_map(launch::net)
        .filter(|&net| net < now)
        .max()
}

/// What to fetch at `now`.
#[must_use]
pub fn due(cache: &Cache, settings: &Settings, now: i64) -> Due {
    if now < cache.throttled_until || now < cache.retry_at {
        return Due::default();
    }
    let upcoming = now.saturating_sub(cache.fetched_upcoming) >= interval(cache, settings, now);
    let landed = recent_launch(cache, now).is_some_and(|net| {
        let look = net.saturating_add(AFTER_LAUNCH);
        cache.fetched_previous < look && look <= now
    });
    let previous = now.saturating_sub(cache.fetched_previous) >= PREVIOUS_EVERY || landed;
    Due { upcoming, previous }
}

/// The earliest time the upcoming list falls due, whether or not a launch
/// comes near before then.
fn upcoming_at(cache: &Cache, settings: &Settings, now: i64) -> i64 {
    let fetched = cache.fetched_upcoming;
    let mut earliest = fetched.saturating_add(interval(cache, settings, now));
    for candidate in cache.upcoming.iter().take(NEAR_CANDIDATES) {
        if let Some(net) = launch::net(candidate) {
            let comes_near = net.saturating_sub(minutes(settings.near_launch_minutes));
            if comes_near > now {
                earliest = earliest
                    .min(comes_near.max(
                        fetched.saturating_add(minutes(settings.refresh_minutes_near_launch)),
                    ));
            }
        }
    }
    earliest
}

/// The earliest time the recent list falls due: every six hours, and half an
/// hour after each lift-off it has not been read since.
fn previous_at(cache: &Cache) -> i64 {
    let mut earliest = cache.fetched_previous.saturating_add(PREVIOUS_EVERY);
    for net in cache
        .upcoming
        .iter()
        .take(crate::cache::MAX_LAUNCHES)
        .filter_map(launch::net)
    {
        let look = net.saturating_add(AFTER_LAUNCH);
        if cache.fetched_previous < look {
            earliest = earliest.min(look);
        }
    }
    earliest
}

/// When to wake next: when something falls due, never before `now`, and not
/// before a rate limit or a retry delay is over.
#[must_use]
pub fn wake_at(cache: &Cache, settings: &Settings, now: i64) -> i64 {
    let earliest = upcoming_at(cache, settings, now).min(previous_at(cache));
    earliest
        .max(now)
        .max(cache.throttled_until)
        .max(cache.retry_at)
}

#[cfg(test)]
mod tests {
    use super::{Due, due, near_launch, wake_at};
    use crate::cache::Cache;
    use crate::config::Settings;
    use serde_json::json;

    const NOW: i64 = 1_000_000;

    fn fetched_now() -> Cache {
        Cache {
            fetched_upcoming: NOW,
            fetched_previous: NOW,
            ..Cache::default()
        }
    }

    #[test]
    fn a_fresh_cache_wakes_after_the_interval() {
        let settings = Settings::default();
        assert_eq!(due(&fetched_now(), &settings, NOW), Due::default());
        assert_eq!(wake_at(&fetched_now(), &settings, NOW), NOW + 15 * 60);
        assert!(due(&fetched_now(), &settings, NOW + 15 * 60).upcoming);
    }

    #[test]
    fn an_empty_cache_is_due_at_once() {
        let settings = Settings::default();
        assert_eq!(
            due(&Cache::default(), &settings, NOW),
            Due {
                upcoming: true,
                previous: true
            }
        );
        assert_eq!(wake_at(&Cache::default(), &settings, NOW), NOW);
    }

    #[test]
    fn a_launch_coming_near_wakes_it_sooner() {
        let settings = Settings::default();
        let mut cache = fetched_now();
        cache.upcoming = vec![json!({"net_epoch": NOW + 2 * 3600 + 60})];
        assert!(!near_launch(&cache, &settings, NOW));
        assert_eq!(wake_at(&cache, &settings, NOW), NOW + 5 * 60);
        assert!(near_launch(&cache, &settings, NOW + 60));
    }

    #[test]
    fn the_recent_list_is_read_again_after_a_launch() {
        let settings = Settings::default();
        let mut cache = fetched_now();
        cache.fetched_upcoming = NOW + 3600;
        cache.upcoming = vec![json!({"net_epoch": NOW + 3600})];
        let woke = wake_at(&cache, &settings, NOW + 3600);
        assert!(woke <= NOW + 3600 + 30 * 60, "{woke}");
        assert!(due(&cache, &settings, NOW + 3600 + 30 * 60).previous);
    }

    #[test]
    fn a_rate_limit_holds_everything() {
        let settings = Settings::default();
        let cache = Cache {
            throttled_until: NOW + 900,
            ..Cache::default()
        };
        assert_eq!(due(&cache, &settings, NOW), Due::default());
        assert_eq!(wake_at(&cache, &settings, NOW), NOW + 900);
    }
}
