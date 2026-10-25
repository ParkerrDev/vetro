# shellcheck shell=bash
# shellcheck disable=SC2034  # names set here are used by the files that source this
# `vetro doctor`: look at the installation and say what is wrong, one line a
# check. It changes nothing. Exit status 1 if any check failed; warnings are
# things that cost the glass but not the desktop.
#
# Sourced by bin/vetro, after lib/common.sh and lib/lay.sh.

DOCTOR_FAILED=0

d_ok() { printf 'ok    %s\n' "$*"; }
d_warn() { printf 'warn  %s\n' "$*"; }
d_fail() {
  printf 'FAIL  %s\n' "$*"
  DOCTOR_FAILED=1
}

# d_link NAME WHAT: NAME is one of Vetro's links and leads somewhere.
d_link() {
  if [ ! -L "$1" ]; then
    d_fail "$2 is not there ($1): run $ROOT/install.sh"
  elif [ ! -e "$1" ]; then
    d_fail "$2 leads nowhere ($1 -> $(readlink "$1")): run $ROOT/install.sh"
  elif ! ours "$1"; then
    d_warn "$2 is a link, but not into this checkout ($1 -> $(readlink "$1"))"
  else
    d_ok "$2"
  fi
}

doctor_links() {
  local plugin
  d_link "$HOME/.local/bin/vetro" "the vetro command"
  d_link "$CONFIG/hypr/vetro.lua" "the Lua file Hyprland loads"
  d_link "$CONFIG/omarchy/hooks/theme-set.d/50-vetro.sh" "the hook after a change of theme"
  d_link "$CONFIG/omarchy/hooks/post-update.d/50-vetro-glass.sh" "the hook after an update"
  d_link "$PLUGINS_DIR/vetro.lib" "the library the shell plugins share"
  d_link "$HOME/.local/share/fonts/vetro" "the fonts"
  if [ -L "$CONFIG/vetro/shell.json" ]; then
    d_warn "the shell settings are a link ($CONFIG/vetro/shell.json): install.sh makes them a copy of your own"
  elif jq -e . "$CONFIG/vetro/shell.json" >/dev/null 2>&1; then
    d_ok "the shell settings"
  else
    d_warn "$CONFIG/vetro/shell.json is missing or not JSON: the shell uses its defaults"
  fi
  if [ -x "$HOME/.local/bin/vetro-notifyd" ]; then
    d_ok "the notification daemon"
  else
    d_warn "the notification daemon is not built (install.sh builds it with cargo): Omarchy's notifications are used"
  fi
  while IFS= read -r plugin; do
    d_link "$PLUGINS_DIR/$plugin" "the $plugin plugin"
  done < <(shell_plugins)
  if bar_built; then
    d_link "$BAR_PLUGIN" "Vetro's bar, built from this Omarchy's"
  else
    d_warn "Vetro's bar is not built from this Omarchy's bar (install.sh says why): Omarchy's own is used, over the glass strip"
  fi
  if block_has "$HYPR_CONFIG" "$HYPR_BEGIN"; then
    d_ok "hyprland.lua loads Vetro"
  else
    d_fail "hyprland.lua does not load Vetro: run $ROOT/install.sh"
  fi
}

doctor_engine() {
  local tool missing=0
  for tool in git make pkg-config g++; do
    command -v "$tool" >/dev/null 2>&1 || {
      d_warn "$tool is not installed: the engine cannot be built"
      missing=1
    }
  done
  if [ "$missing" = 0 ]; then
    d_ok "the tools the engine is built with are installed"
    if "$ROOT/engine/build.sh" --check >/dev/null 2>&1; then
      d_ok "the engine is built for the Hyprland that is installed"
    else
      d_warn "$("$ROOT/engine/build.sh" --check 2>&1 | head -n 1): vetro build"
    fi
  fi
  if ! in_hyprland; then
    d_warn "not in a Hyprland session: the engine's state in it was not looked at"
  elif glass_loaded && glass_fits; then
    d_ok "the engine is loaded in the running Hyprland"
  elif is_on; then
    d_warn "the engine is not loaded in the running Hyprland: panes are plain blur (vetro glass load)"
  fi
  if [ -e "$GUARD_MARK" ]; then
    d_warn "the config stood down after being read in a loop ($(sed -n 1p "$GUARD_MARK")): vetro on"
  else
    d_ok "the guard against a config read in a loop is ready"
  fi
}

doctor_state() {
  local plugin errors
  if is_on; then
    if block_has "$SHELL_TOML" "$LOOK_BEGIN"; then
      d_ok "the bar's block is in shell.toml"
    else
      d_fail "Vetro is on but the bar's block is not in shell.toml: vetro on"
    fi
    if shell_alive; then
      for plugin in "${CORE_PLUGINS[@]}"; do
        # left off on purpose without the daemon (said above)
        if needs_notifyd "$plugin" && [ ! -x "$HOME/.local/bin/vetro-notifyd" ]; then
          continue
        fi
        if omarchy-plugin-list --json 2>/dev/null | jq -e --arg id "$plugin" 'any(.[]; .id == $id and .enabled)' >/dev/null 2>&1; then
          d_ok "$plugin is enabled"
        else
          d_fail "Vetro is on but $plugin is not enabled: vetro on"
        fi
      done
      plugin=$(bar_in_use)
      if [ "$plugin" = vetro.bar ]; then
        if bar_built; then
          d_ok "Vetro's bar is the bar in use"
        else
          d_fail "the shell is set to Vetro's bar, which is not built: vetro on"
        fi
      elif [ "$plugin" != omarchy.bar ]; then
        d_ok "a bar of your own choosing is in use ($plugin), and Vetro leaves it"
      elif bar_built; then
        d_fail "Vetro is on but Omarchy's own bar is in use: vetro on"
      fi
      for plugin in "${BAR_CLONES[@]}"; do
        if on_bar "${plugin##*:}"; then
          d_fail "Vetro is on but ${plugin##*:} is still on the bar in place of ${plugin%%:*}: vetro on"
        elif on_bar "${plugin%%:*}"; then
          d_ok "${plugin%%:*} is on the bar"
        fi
      done
    else
      d_warn "the shell is not running: the plugins were not looked at"
    fi
  else
    if block_has "$SHELL_TOML" "$LOOK_BEGIN"; then
      d_fail "Vetro is off but its block is still in shell.toml: vetro off"
    else
      d_ok "Vetro is off and has left nothing in shell.toml"
    fi
  fi
  if in_hyprland; then
    errors=$(hyprctl configerrors 2>/dev/null | grep -v '^$' || true)
    if [ -n "$errors" ]; then
      d_warn "Hyprland reports config errors: $(printf '%s' "$errors" | head -n 1)"
    else
      d_ok "Hyprland reports no config errors"
    fi
  fi
}

# The packages the look draws on. Vetro installs none (that is root's work); a
# missing one costs the part of the look that uses it and nothing else.
LOOK_PACKAGES=(papirus-icon-theme adw-gtk-theme kvantum qt6ct-kde xcursor-pro-cursor-theme xcursor-pro-hyprcursor)
# The launch addon's daemon is built with cargo (Arch's rust, or rustup's) and
# fetches with curl: commands, whichever package brought them.
LAUNCH_TOOLS=(cargo curl)

doctor_packages() {
  local package missing=()
  command -v pacman >/dev/null 2>&1 || return 0
  for package in "${LOOK_PACKAGES[@]}"; do
    pacman -Qq "$package" >/dev/null 2>&1 || missing+=("$package")
  done
  if addon_installed launch; then
    for package in "${LAUNCH_TOOLS[@]}"; do
      command -v "$package" >/dev/null 2>&1 || missing+=("$package")
    done
  fi
  if [ ${#missing[@]} -eq 0 ]; then
    d_ok "the packages the look draws on are installed"
  else
    d_warn "not installed: ${missing[*]} (the part of the look that uses each is missing)"
  fi
}

doctor_leftovers() {
  local dangling line
  dangling=$(find "$CONFIG" "$HOME/.local/bin" "$HOME/.local/share/fonts" -maxdepth 5 -xtype l -lname "$ROOT/*" 2>/dev/null || true)
  if [ -n "$dangling" ]; then
    d_fail "links into this checkout lead nowhere:"
    while IFS= read -r line; do
      printf '        %s\n' "$line"
    done <<<"$dangling"
  else
    d_ok "no link into this checkout is dangling"
  fi
  if [ -x "$ROOT/migrate.sh" ]; then
    if "$ROOT/migrate.sh" --check >/dev/null 2>&1; then
      d_ok "nothing is left of the earlier projects"
    else
      d_fail "parts of the earlier projects are still in this home folder: $ROOT/migrate.sh --dry-run says what"
    fi
  fi
}

doctor() {
  DOCTOR_FAILED=0
  doctor_links
  doctor_engine
  doctor_state
  doctor_packages
  doctor_leftovers
  if [ "$DOCTOR_FAILED" = 0 ]; then
    echo "all well"
  else
    echo "something is wrong"
    return 1
  fi
}
