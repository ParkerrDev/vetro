#!/bin/bash
# The launch addon: the next SpaceX launches, and the weather, as two cards.
# A daemon (vetro-launchd, a user service written in Rust) keeps the launch
# data the cards, the lock screen and the login screen read. Nothing else in
# Vetro needs it; leave it out with `install.sh --without launch`.
#
#   addon.sh install            build the daemon (cargo), link the commands,
#                               the service, its settings, the cards' plugin,
#                               and the Homescreen widgets where Homescreen is
#                               installed
#   addon.sh uninstall [--purge]  take them out; --purge also removes the
#                               launch cache, the daemon's build and its state
#   addon.sh on                 enable the cards and (re)start the daemon
#   addon.sh off                stop the daemon and disable the cards
#   addon.sh status
#
# vetro on and vetro off call on and off for every addon that is installed.

# shellcheck source-path=SCRIPTDIR
set -euo pipefail

HERE=$(dirname "$(readlink -f "$0")")
ROOT=$(cd "$HERE/../.." && pwd -P)
# shellcheck source=../../lib/common.sh
. "$ROOT/lib/common.sh"

NAME=launch
UNIT=vetro-launchd.service
UNIT_DIR=$CONFIG/systemd/user
XDG_CACHE=${XDG_CACHE_HOME:-$HOME/.cache}
XDG_DATA=${XDG_DATA_HOME:-$HOME/.local/share}

in_hyprland() { [ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]; }

BUILD=$XDG_CACHE/vetro/launchd-target
BUILD_LOG=$XDG_CACHE/vetro/launchd-build.log
# The built daemon is kept in Vetro's state folder, the place for files made
# for this machine, and linked onto the PATH from there.
DAEMON=$STATE/bin/vetro-launchd

reload_units() {
  systemctl --user daemon-reload 2>/dev/null || true
}

# Built from launchd/ with the versions in its Cargo.lock, and moved into
# place whole: a running daemon keeps the file it started from.
build_daemon() {
  mkdir -p "$STATE/bin" "$XDG_CACHE/vetro"
  cargo build --release --locked --quiet --manifest-path "$HERE/launchd/Cargo.toml" \
    --target-dir "$BUILD" >"$BUILD_LOG" 2>&1 || return 1
  [ -s "$BUILD/release/vetro-launchd" ] || return 1
  cp "$BUILD/release/vetro-launchd" "$DAEMON.new"
  chmod 0755 "$DAEMON.new"
  mv -f "$DAEMON.new" "$DAEMON"
}

# weather_from_shell FILE: the weather settings an earlier Vetro kept with the
# shell's (install.sh left them in the state folder) go into the [weather]
# section of FILE, where they are now, unless FILE says where already.
weather_from_shell() {
  local file=$1 old=$STATE/weather-from-shell.json location units=fahrenheit
  [ -f "$old" ] || return 0
  location=$(jq -r '.location // ""' "$old")
  [ "$(jq -r '.celsius // false' "$old")" = true ] && units=celsius
  if ! grep -Eq '^[[:space:]]*(location|units)[[:space:]]*=' "$file"; then
    awk -v location="$location" -v units="$units" '
      { print }
      /^\[weather\][[:space:]]*$/ {
        gsub(/["\\]/, "", location)
        printf "location = \"%s\"\nunits = \"%s\"\n", location, units
      }' "$file" >"$file.new" && mv "$file.new" "$file"
    echo "  $file: the weather is for ${location:-where this machine is}, in $units"
  fi
  rm -f "$old"
}

# The daemon is built with cargo and fetches with curl: without them it could
# not be made, or would only fail and be started again, so the addon is left
# out until they are there.
install() {
  local tool
  for tool in cargo curl; do
    if ! command -v "$tool" >/dev/null 2>&1; then
      echo "  $tool is not installed: the launch addon is left out until it is (install.sh again)"
      return 0
    fi
  done
  if ! build_daemon; then
    echo "  the launch daemon did not build ($BUILD_LOG): the launch addon is left out"
    return 0
  fi
  link "$HERE/bin/vetro-launch" "$HOME/.local/bin/vetro-launch"
  link "$DAEMON" "$HOME/.local/bin/vetro-launchd"
  seed "$HERE/launches.toml" "$CONFIG/vetro/launches.toml"
  weather_from_shell "$CONFIG/vetro/launches.toml"
  # the settings of the video wallpaper, which is gone (the daemon reads its
  # [launches] section where launches.toml is not there)
  unlink_ours "$CONFIG/vetro/wallpaper.toml"
  link "$HERE/plugin/vetro.cards" "$PLUGINS_DIR/vetro.cards"
  # The two cards as widgets, where Homescreen is installed to place them.
  if [ -d "$CONFIG/homescreen/widgets" ]; then
    link "$HERE/widget" "$CONFIG/homescreen/widgets/launches"
    link "$HERE/weather-widget" "$CONFIG/homescreen/widgets/forecast"
  fi
  link "$HERE/systemd/$UNIT" "$UNIT_DIR/$UNIT"
  # the video's decoder settings, gone with it
  unlink_ours "$UNIT_DIR/$UNIT.d/intel-decode.conf"
  prune "$UNIT_DIR/$UNIT.d"
  reload_units
  mkdir -p "$ADDON_MARKS"
  : >"$ADDON_MARKS/$NAME"
}

uninstall() {
  systemctl --user stop "$UNIT" 2>/dev/null || true
  unlink_ours "$HOME/.local/bin/vetro-launch"
  unlink_ours "$HOME/.local/bin/vetro-launchd"
  rm -f "$DAEMON"
  prune "$STATE/bin"
  # the settings go if they are as Vetro made them (and a link an earlier
  # Vetro made there goes)
  unlink_ours "$CONFIG/vetro/launches.toml"
  unseed "$HERE/launches.toml" "$CONFIG/vetro/launches.toml"
  unlink_ours "$CONFIG/vetro/wallpaper.toml"
  unlink_ours "$PLUGINS_DIR/vetro.cards"
  unlink_ours "$CONFIG/homescreen/widgets/launches"
  unlink_ours "$CONFIG/homescreen/widgets/forecast"
  unlink_ours "$UNIT_DIR/$UNIT.d/intel-decode.conf"
  prune "$UNIT_DIR/$UNIT.d"
  unlink_ours "$UNIT_DIR/$UNIT"
  prune "$UNIT_DIR" "$CONFIG/systemd"
  reload_units
  # the daemon's build is made again by an install, in a few seconds
  rm -rf "$BUILD" "$BUILD_LOG"
  if [ "${1:-}" = --purge ]; then
    rm -rf "$XDG_DATA/vetro/clips" "$XDG_CACHE/vetro/launches.json" "$XDG_CACHE/vetro/weather.json" "$STATE/state.json"
    echo "  removed the launch cache and the daemon's state"
  fi
  prune "$XDG_CACHE/vetro" "$XDG_CACHE"
  rm -f "$ADDON_MARKS/$NAME"
  prune "$CONFIG/vetro"
}

on() {
  if shell_alive; then
    omarchy-shell shell rescanPlugins >/dev/null 2>&1 || true
    wait_for_plugin vetro.cards || warn "the shell does not know vetro.cards"
    omarchy plugin enable vetro.cards >/dev/null 2>&1 || warn "could not enable vetro.cards"
  fi
  if in_hyprland; then
    systemctl --user restart "$UNIT" 2>/dev/null || warn "could not start $UNIT"
  fi
}

off() {
  systemctl --user stop "$UNIT" 2>/dev/null || true
  if shell_alive; then
    omarchy plugin disable vetro.cards >/dev/null 2>&1 || true
  fi
}

status() {
  if systemctl --user is-active --quiet "$UNIT" 2>/dev/null; then
    echo "launch: installed, $UNIT is running"
  else
    echo "launch: installed, $UNIT is not running"
  fi
}

case ${1:-} in
  install) install ;;
  uninstall) uninstall "${2:-}" ;;
  on) on ;;
  off) off ;;
  status) status ;;
  *)
    awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"
    exit 2
    ;;
esac
