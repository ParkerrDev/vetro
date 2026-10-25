#!/bin/bash
# Puts Vetro in place on Omarchy 4. It needs no root, changes nothing you can
# see until you switch it on (`vetro on`), and is safe to run again.
#
# On a home folder that has the two projects Vetro replaces installed it is also
# the move to Vetro (docs/history.md says which): it takes their links, blocks,
# service and state over to Vetro's names (migrate.sh), builds nothing the old
# engine already is, and leaves the look as it was, on or off.
#
#   ./install.sh                  link everything, then build the glass engine
#   ./install.sh --no-build       link everything and leave the engine and the
#                                 notification daemon for later (engine/build.sh,
#                                 install.sh again); until then the panes have
#                                 plain blur and Omarchy's notifications stay
#   ./install.sh --without NAME   leave an addon out (launch, theme); the core
#                                 cannot be left out
#   ./install.sh --on             and then switch it on (vetro on)
#
# Piped from the web it fetches Vetro first, into ~/.local/share/vetro (or
# updates the copy there), and runs that copy's install.sh:
#
#   curl -fsSL https://raw.githubusercontent.com/ParkerrDev/vetro/master/install.sh | bash -s -- --on
#
# Everything but the settings lands as a symlink into this folder, so editing
# the folder changes the desktop and removing it takes one script
# (uninstall.sh). A real file already at one of these names is kept as
# NAME.pre-vetro. The settings are copied, once, and are yours to change.
#
#   ~/.local/bin/vetro                       the command
#   ~/.local/bin/vetro-focus-or-open         what the menu starts apps with
#   ~/.local/bin/vetro-notifyd               the notification daemon, built here
#   ~/.local/share/applications/dev.vetro.Glass.desktop
#                                            the tuning window (vetro tune)
#   ~/.config/hypr/vetro.lua                 the glass, the blur, the corners
#   ~/.config/omarchy/plugins/vetro.*        the shell plugins, and the library
#                                            they share (vetro.lib)
#   ~/.config/omarchy/hooks/                 after a change of theme, after an
#                                            update
#   ~/.config/vetro/shell.json               the shell's settings (a copy)
#   ~/.local/share/fonts/vetro               D-DIN and the icon fonts
#
# and what each addon in addons/ links. The one file it edits is
# ~/.config/hypr/hyprland.lua, which gains a marked block of a single line at
# its end.
#
# Then: vetro on

# shellcheck source-path=SCRIPTDIR
set -euo pipefail

# From a pipe there is no folder beside this script. Rust, which the engine
# and the notification daemon are built with, comes from Omarchy's own command
# when it is missing (rustup, in the home folder, no root).
if [ ! -f "$(dirname "$(readlink -f "$0")")/lib/common.sh" ]; then
  dest=${VETRO_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/vetro}
  command -v git >/dev/null || { echo "Vetro needs git" >&2; exit 1; }
  if [ -d "$dest/.git" ]; then
    echo "Updating $dest"
    git -C "$dest" pull --quiet --ff-only
  else
    echo "Fetching Vetro into $dest"
    git clone --quiet https://github.com/ParkerrDev/vetro "$dest"
  fi
  export PATH=$HOME/.cargo/bin:$PATH
  if ! command -v cargo >/dev/null; then
    command -v omarchy-install-dev-env >/dev/null || { echo "Vetro is for Omarchy 4" >&2; exit 1; }
    echo "Installing Rust (omarchy-install-dev-env rust)"
    omarchy-install-dev-env rust </dev/null
  fi
  exec "$dest/install.sh" "$@" </dev/null
fi

ROOT=$(dirname "$(readlink -f "$0")")
# shellcheck source=lib/common.sh
. "$ROOT/lib/common.sh"
# shellcheck source=lib/lay.sh
. "$ROOT/lib/lay.sh"

usage() {
  awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"
}

build=yes
on=no
without=()
while [ $# -gt 0 ]; do
  case $1 in
    --no-build) build=no ;;
    --on) on=yes ;;
    --without)
      shift
      without+=("${1:?--without needs the name of an addon}")
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
  shift
done

[ "$(id -u)" != 0 ] || die "run it as yourself, not as root"
[ -f "$HYPR_CONFIG" ] || die "$HYPR_CONFIG is missing. Vetro is for Omarchy 4, whose Hyprland config is Lua."
command -v jq >/dev/null || die "jq is not installed"

# left_out NAME: whether --without named this addon.
left_out() {
  local name
  for name in ${without[@]+"${without[@]}"}; do
    [ "$name" = "$1" ] && return 0
  done
  return 1
}

link_core() {
  local plugin
  say "The command and the Lua file"
  link "$ROOT/bin/vetro" "$HOME/.local/bin/vetro"
  link "$ROOT/bin/vetro-focus-or-open" "$HOME/.local/bin/vetro-focus-or-open"
  link "$ROOT/hypr/vetro.lua" "$CONFIG/hypr/vetro.lua"
  # the tuning window (vetro tune), which builds itself the first time it opens
  link "$ROOT/apps/glass/vetro-glass.desktop" "$HOME/.local/share/applications/dev.vetro.Glass.desktop"
  link "$ROOT/assets/branding/vetro.svg" "$HOME/.local/share/icons/hicolor/scalable/apps/dev.vetro.Glass.svg"
  link "$ROOT/assets/branding/vetro-symbolic.svg" "$HOME/.local/share/icons/hicolor/symbolic/apps/dev.vetro.Glass-symbolic.svg"

  say "Hooks"
  link "$ROOT/hooks/theme-set.d/50-vetro.sh" "$CONFIG/omarchy/hooks/theme-set.d/50-vetro.sh"
  link "$ROOT/engine/post-update.sh" "$CONFIG/omarchy/hooks/post-update.d/50-vetro-glass.sh"

  say "Shell plugins and fonts"
  unlink_gone "$PLUGINS_DIR"
  link "$ROOT/shell/vetro.lib" "$PLUGINS_DIR/vetro.lib"
  while IFS= read -r plugin; do
    link "$ROOT/shell/$plugin" "$PLUGINS_DIR/$plugin"
  done < <(shell_plugins)
  bar_build || true
  link "$ROOT/shell/fonts" "$HOME/.local/share/fonts/vetro"
  if command -v fc-cache >/dev/null; then
    fc-cache -f "$HOME/.local/share/fonts/vetro" >/dev/null 2>&1 || true
  fi
  say "Settings"
  seed "$ROOT/shell/shell.json" "$CONFIG/vetro/shell.json"
  settings_now "$CONFIG/vetro/shell.json"
}

# settings_now FILE: an earlier Vetro's settings, in the names they have now.
# Its weather settings belong to the launch addon's file since: they are kept
# in the state folder for the addon to take (addons/launch/addon.sh).
settings_now() {
  local file=$1 new
  jq -e '.fonts != null or .misc != null or .panels.baseOpacity? != null' "$file" >/dev/null 2>&1 || return 0
  mkdir -p "$STATE"
  jq '{location: (.bar.weatherLocation // "" | if . == "None" then "" else . end),
       celsius: (.bar.weatherTempInCelcius // false)}' "$file" >"$STATE/weather-from-shell.json"
  new=$(jq '{
    gaps: {edge: (.bar.verticalGap // 5), sides: (.bar.horizontalGap // 5)},
    bar: {radius: (.bar.radius // 15)},
    panels: {radius: (.panels.radius // 20), opacity: (.panels.baseOpacity // 0.2),
             frame: (.panels.borders // false), frameWidth: (.panels.bordersSize // 1)},
    text: {size: (.fonts.basePointSize // 10), nativeRendering: (.fonts.useNativeRendering // false)},
    clock: {twelveHour: ((.misc.dateFormat // 0) as $f | [2, 3, 4] | index($f) != null)},
    osd: {timeout: (.osd.timeoutDuration // 700)},
    motion: {scale: 1},
    screens: {only: [], except: []}
  }' "$file") || return 0
  cp "$file" "$file.pre-vetro"
  printf '%s\n' "$new" >"$file"
  echo "  $file: in the names Vetro has for them now (the old file is $file.pre-vetro)"
}

# notifyd: the notification daemon, built from shell/notifyd with the versions
# in its Cargo.lock and moved into place whole (a running daemon keeps the file
# it started from). Without cargo, or with --no-build, it is not built, and
# Omarchy's notifications stay (lay.sh does not enable Vetro's without it).
notifyd() {
  local cache=${XDG_CACHE_HOME:-$HOME/.cache}/vetro binary=$STATE/bin/vetro-notifyd
  local target=$cache/notifyd-target log=$cache/notifyd-build.log
  say "The notification daemon"
  if [ "$build" = no ]; then
    echo "  not built now (install.sh builds it)"
    return 0
  fi
  if ! command -v cargo >/dev/null 2>&1; then
    echo "  cargo is not installed: Omarchy's notifications stay until it is (install.sh again)"
    return 0
  fi
  mkdir -p "$STATE/bin" "$cache"
  if ! cargo build --release --locked --quiet --manifest-path "$ROOT/shell/notifyd/Cargo.toml" \
    --target-dir "$target" >"$log" 2>&1 || [ ! -s "$target/release/vetro-notifyd" ]; then
    echo "  it did not build ($log): Omarchy's notifications stay"
    return 0
  fi
  cp "$target/release/vetro-notifyd" "$binary.new"
  chmod 0755 "$binary.new"
  mv -f "$binary.new" "$binary"
  link "$binary" "$HOME/.local/bin/vetro-notifyd"
}

# know_plugins: a shell that is running is told to look at its plugin folder
# again, and waited for. The plugin ids migrate.sh writes into shell.json have
# to be ones it knows.
know_plugins() {
  local plugin
  shell_alive || return 0
  omarchy-shell shell rescanPlugins >/dev/null 2>&1 || true
  while IFS= read -r plugin; do
    wait_for_plugin "$plugin" || warn "the shell does not know $plugin yet"
  done < <(shell_plugins)
}

hyprland() {
  say "Hyprland"
  if block_has "$HYPR_CONFIG" "$HYPR_BEGIN"; then
    echo "  $HYPR_CONFIG (already loads it)"
  else
    hypr_block_put
    echo "  $HYPR_CONFIG now loads it"
  fi
}

# settle_state: Vetro is off until it is asked for. A home folder that has
# nothing of it laid, and was not on before, starts off, so that installing it
# changes nothing on screen; one that was on stays on.
settle_state() {
  mkdir -p "$STATE"
  [ -e "$STATE/resume-on" ] && return 0
  if [ ! -e "$FLAG" ] && ! block_has "$SHELL_TOML" "$LOOK_BEGIN"; then
    : >"$FLAG"
  fi
}

install_addons() {
  local script name
  for script in "$ROOT"/addons/*/addon.sh; do
    [ -x "$script" ] || continue
    name=$(basename "$(dirname "$script")")
    if left_out "$name"; then
      echo "  $name: left out"
      continue
    fi
    say "Addon: $name"
    "$script" install
  done
}

build_engine() {
  say "Glass engine"
  if [ "$build" = no ]; then
    echo "  not built now (engine/build.sh builds it)"
    return 0
  fi
  # Not having it is a supported state, so a failed build ends nothing.
  "$ROOT/engine/build.sh" || echo "  The glass engine was not built. The panes will have plain blur; engine/build.sh tries again."
}

say "Linking Vetro into $HOME"
link_core
notifyd
know_plugins

say "Earlier projects"
"$ROOT/migrate.sh"

hyprland
settle_state
install_addons
build_engine

if [ -e "$STATE/resume-on" ]; then
  say "Vetro was on: laying it again under its new names"
  "$ROOT/bin/vetro" on
  rm -f "$STATE/resume-on"
elif [ ! -e "$FLAG" ]; then
  # A Vetro that is on gets the plugins this version added, enabled as
  # `vetro on` enables them. Those it has already are left as they are, and
  # nothing is reloaded.
  say "Vetro is on: its plugins"
  lay_plugins
  # A bar the shell was switched to is whole only after the shell has started
  # afresh. Not while the screen is locked: the lock is the shell's.
  if [ "$BAR_SWITCHED" = yes ]; then
    if command -v omarchy-hyprland-session-locked >/dev/null 2>&1 && omarchy-hyprland-session-locked; then
      echo "  The bar changes when the shell next starts (the screen is locked now)."
    else
      omarchy-restart-shell >/dev/null 2>&1 || true
    fi
  fi
fi

if [ "$on" = yes ] && [ -e "$FLAG" ]; then
  say "Switching it on"
  "$ROOT/bin/vetro" on
fi

say "Done."
if [ -e "$FLAG" ]; then
  cat <<'EOF'
  Nothing has changed on screen yet.
    vetro on        lay the glass over your theme
    vetro off       take it off again
    vetro doctor    check the installation
EOF
else
  echo "  Vetro is on. vetro off takes it off again; vetro doctor checks the installation."
fi
