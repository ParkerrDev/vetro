# The launch addon

The next SpaceX launches, and the weather, as two glass cards. Nothing else in
Vetro needs it; `./install.sh --without launch` leaves it out.

- **Two glass cards**: the next launch, with the ones after it, and the weather.
  Where Homescreen is installed they are widgets too, which can be placed and
  moved on its grid.
- **The lock screen** (the shell plugin `vetro.lock`) shows the next launch, and
  the login screen (`addons/login`) shows the same.

The video wallpaper the addon used to play (a clip of the latest launch, and the
live webcast) was removed on 2026-10-18. The cards stay.

## The parts

```
addon.sh                 install, uninstall, on, off, status (vetro on and off call them)
launchd/                 the daemon, in Rust: a library that never prints or touches the
                         network (settings, launches, when to fetch) and a thin binary over it
bin/vetro-launch         the command that talks to it
plugin/vetro.cards/      the shell plugin with the two cards (their QML is in launch/)
widget/                  the launch card as a Homescreen widget (`launches`): the same card, on its grid, where it can be moved
weather-widget/          the weather card the same way (`forecast`). While either is placed, the one in the corner stands down
systemd/                 the service
launches.toml            the settings, all optional and commented
```

## The daemon

`vetro-launchd` runs as a user service, started at login by `hypr/vetro.lua` and
by `vetro on`. `addon.sh install` builds it with cargo, from the versions in
`launchd/Cargo.lock`, and keeps the binary in `~/.local/state/vetro/bin`. It:

- asks [Launch Library 2](https://thespacedevs.com/llapi) for upcoming and recent
  SpaceX launches, with curl, every quarter of an hour (every five minutes from
  two hours before a launch), and half an hour after each lift-off for the time
  it really flew; it writes them to `~/.cache/vetro/launches.json`, which the
  cards and the lock screen read;
- sleeps in between: nothing wakes it until a fetch is due or a request comes
  (the Python daemon before it woke every second);
- keeps to Launch Library's rate limit (15 calls an hour) and, after a failed
  fetch, tries again in two minutes;
- copies the launch data and your picture to `/var/lib/vetro` for the login
  screen, where that folder exists.

It uses about 5 MB of memory. Settings are in `~/.config/vetro/launches.toml`
(an older `wallpaper.toml` is read for its `[launches]` section where that file
is missing). Apply changes with `vetro-launch reload`. Logs:
`journalctl --user -u vetro-launchd -f`.

```
vetro-launch status          the next launch, and what went wrong if anything
vetro-launch refresh         fetch launch data now
vetro-launch reload          read the settings again
vetro-launch next [--short]  one line about the next launch
vetro-launchd fetch [upcoming|previous] [N]   print launches straight from Launch Library
```

## What it uses

`cargo` to build the daemon and `curl` to fetch, without which `install.sh` leaves
the addon out (and says so).
