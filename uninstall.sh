#!/bin/bash
# Takes Vetro out again: everything install.sh put in place, and what the glass
# build and the switch left behind. This folder itself is not touched, and
# neither are your theme and wallpaper: Vetro never set them.
#
#   ./uninstall.sh           Vetro off and out; your data stays
#   ./uninstall.sh --purge   and the data goes too
#
# In order: Vetro is switched off (if it is on), the addons come out, the block
# comes out of ~/.config/hypr/hyprland.lua, then the links, the glass engine's
# build folder and Vetro's own state. Only links that point into this folder
# are removed, and a file that install.sh set aside as NAME.pre-vetro goes back
# to its name.
#
# The data is what is slow to get again: the built glass engine, the launch
# clips, the saved state of the shell, and the record of a migration
# (~/.local/state/vetro-migrated), which is the way back to the earlier
# projects (migrate.sh --undo). --purge is for when none of it is wanted.

# shellcheck source-path=SCRIPTDIR
set -euo pipefail

ROOT=$(dirname "$(readlink -f "$0")")
# shellcheck source=lib/common.sh
. "$ROOT/lib/common.sh"
# shellcheck source=lib/lay.sh
. "$ROOT/lib/lay.sh"

purge=no
case ${1:-} in
  --purge) purge=yes ;;
  "") ;;
  *)
    awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"
    exit 2
    ;;
esac

XDG_CACHE=${XDG_CACHE_HOME:-$HOME/.cache}
XDG_DATA=${XDG_DATA_HOME:-$HOME/.local/share}

# laid: whether anything of Vetro's is laid over the theme.
laid() {
  block_has "$SHELL_TOML" "$LOOK_BEGIN" || block_has "$SHELL_TOML" "$GLASS_BEGIN"
}

switch_off() {
  say "Switching off"
  if [ -x "$ROOT/bin/vetro" ] && laid; then
    "$ROOT/bin/vetro" off || echo "  could not switch it off cleanly: the steps below take out what is left"
  else
    echo "  nothing is laid"
  fi
  # The browser's grey too, where off did not see to it.
  unlay_browser
}

remove_addons() {
  local name
  say "Addons"
  while IFS= read -r name; do
    if [ -x "$ROOT/addons/$name/addon.sh" ]; then
      if [ "$purge" = yes ]; then
        "$ROOT/addons/$name/addon.sh" uninstall --purge
      else
        "$ROOT/addons/$name/addon.sh" uninstall
      fi
    fi
  done < <(installed_addons)
}

remove_links() {
  local plugin
  say "Links"
  unlink_ours "$HOME/.local/bin/vetro"
  unlink_ours "$HOME/.local/bin/vetro-focus-or-open"
  unlink_ours "$HOME/.local/bin/vetro-notifyd"
  unlink_ours "$CONFIG/hypr/vetro.lua"
  unlink_ours "$HOME/.local/share/applications/dev.vetro.Glass.desktop"
  unlink_ours "$HOME/.local/share/icons/hicolor/scalable/apps/dev.vetro.Glass.svg"
  unlink_ours "$HOME/.local/share/icons/hicolor/symbolic/apps/dev.vetro.Glass-symbolic.svg"
  prune "$HOME/.local/share/icons/hicolor/scalable/apps" "$HOME/.local/share/icons/hicolor/scalable" \
    "$HOME/.local/share/icons/hicolor/symbolic/apps" "$HOME/.local/share/icons/hicolor/symbolic" \
    "$HOME/.local/share/icons/hicolor" "$HOME/.local/share/icons"
  unlink_ours "$CONFIG/omarchy/hooks/theme-set.d/50-vetro.sh"
  unlink_ours "$CONFIG/omarchy/hooks/post-update.d/50-vetro-glass.sh"
  unlink_gone "$PLUGINS_DIR"
  unlink_ours "$PLUGINS_DIR/vetro.lib"
  while IFS= read -r plugin; do
    unlink_ours "$PLUGINS_DIR/$plugin"
  done < <(shell_plugins)
  bar_drop
  unlink_ours "$HOME/.local/share/fonts/vetro"
  # the settings go if they are as Vetro made them (and a link an earlier
  # Vetro made there goes)
  unlink_ours "$CONFIG/vetro/shell.json"
  unseed "$ROOT/shell/shell.json" "$CONFIG/vetro/shell.json"
  prune "$CONFIG/vetro" "$HOME/.local/share/fonts" "$HOME/.local/share/applications" "$HOME/.local/share"
  if shell_alive; then
    omarchy-shell shell rescanPlugins >/dev/null 2>&1 || true
  fi
}

remove_built() {
  say "What was built and what was kept"
  rm -rf "$XDG_CACHE/vetro/engine" "$XDG_CACHE/vetro/build.log"
  # the notification daemon and its build, made again by an install
  rm -rf "$XDG_CACHE/vetro/notifyd-target" "$XDG_CACHE/vetro/notifyd-build.log" "$STATE/bin/vetro-notifyd"
  rmdir "$STATE/bin" 2>/dev/null || true
  rmdir "$XDG_CACHE/vetro" 2>/dev/null || true
  rm -f "$STATE/off" "$STATE/resume-on" "$STATE/build.log" "$STATE/qt6ct.conf"
  rmdir "$STATE/addons" 2>/dev/null || true
  if [ "$purge" = yes ]; then
    rm -f "$ENGINE_DIR"/hyprglass.so "$ENGINE_DIR"/hyprglass.so.new "$ENGINE_DIR"/hyprglass.built-for "$ENGINE_DIR"/hyprglass.built-for.new
    rm -f "$STATE"/misc.json "$STATE"/account.json "$STATE"/notifications.json "$STATE"/state.json
    rm -rf "${XDG_STATE_HOME:-$HOME/.local/state}/vetro-migrated"
    rmdir "$XDG_DATA/vetro" 2>/dev/null || true
    echo "  removed the engine, the saved state and the record of the migration"
  else
    if [ -s "$ENGINE" ]; then echo "  kept the built engine ($ENGINE)"; fi
    if [ -d "$XDG_DATA/vetro/clips" ]; then echo "  kept the launch clips ($XDG_DATA/vetro/clips)"; fi
    if [ -d "${XDG_STATE_HOME:-$HOME/.local/state}/vetro-migrated" ]; then
      echo "  kept the record of the migration (migrate.sh --undo uses it)"
    fi
  fi
  rmdir "$ENGINE_DIR" "$STATE" 2>/dev/null || true
  if [ -n "${XDG_RUNTIME_DIR:-}" ]; then
    rm -f "$XDG_RUNTIME_DIR/vetro-glass-stood-down" "$XDG_RUNTIME_DIR/vetro-glass-readings"
  fi
  return 0
}

switch_off
remove_addons

say "Hyprland"
if block_has "$HYPR_CONFIG" "$HYPR_BEGIN"; then
  hypr_block_drop
  echo "  $HYPR_CONFIG no longer loads it"
fi

remove_links
remove_built

say "Done. Vetro is out; this folder is yours to keep or delete."
