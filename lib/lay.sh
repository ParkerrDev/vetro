# shellcheck shell=bash
# shellcheck disable=SC2034  # names set here are used by the files that source this
# Laying Vetro over Omarchy, and taking it off again. Everything `vetro on`
# puts in your home folder is in here, and `vetro off` undoes each of them.
# Omarchy keeps its own bar (every widget, panel and gesture), its keybinds,
# its monitors and its apps; what changes is how they look. No part of it
# chooses a theme, a wallpaper or a colour scheme: it lies over whichever are
# there, and goes when it is switched off.
#
# Sourced by bin/vetro, after lib/common.sh.

# The shell plugins `vetro on` enables, in the order it enables them. Four of
# them (the notifications, the volume display, the lock screen and the menu) are
# clones of Omarchy plugins: enabling one switches the original off, and
# disabling it brings the original back.
CORE_PLUGINS=(vetro.notifications vetro.osd vetro.lock vetro.barglass vetro.center vetro.center-button vetro.menu)
OMARCHY_PLUGINS=(omarchy.menu omarchy.notifications omarchy.osd omarchy.lock)

# Widgets of Vetro's that stand where one of Omarchy's stands on the bar, each
# written with its original after a colon. `vetro on` puts Vetro's in the
# original's place, with its settings, and `vetro off` gives the place back.
# Where you took the original off your bar, Vetro's stays off it too.
BAR_CLONES=(vetro.workspaces:omarchy.workspaces)
SHELL_JSON=$CONFIG/omarchy/shell.json

# shell_plugins: every plugin folder of Vetro's that install.sh links.
shell_plugins() {
  local pair
  printf '%s\n' "${CORE_PLUGINS[@]}"
  for pair in "${BAR_CLONES[@]}"; do
    printf '%s\n' "${pair%%:*}"
  done
}

# Vetro's bar is Omarchy's own with three things changed (tools/bar-build.py
# says which), built from the Omarchy bar that is installed. Where it cannot
# be built, Omarchy's own bar stays in use, over the strip of glass
# shell/vetro.barglass draws under it.
BAR_SOURCE=${VETRO_BAR_SOURCE:-${OMARCHY_PATH:-/usr/share/omarchy}/shell/plugins/bar}
BAR_OUT=$STATE/bar
BAR_PLUGIN=$PLUGINS_DIR/vetro.bar

# on_bar ID: whether a widget is on the bar, in any of its three parts.
on_bar() {
  [ -f "$SHELL_JSON" ] || return 1
  jq -e --arg id "$1" \
    '[.bar.layout[]?[]? | if type == "object" then .id else . end] | index($id) != null' \
    "$SHELL_JSON" >/dev/null 2>&1
}

# The files of the user's, and the text of Vetro's that goes into a block in
# each: the terminals. A terminal that is not installed has no file, and gets
# no block.
TERMINALS=("foot/foot.ini:terminal/foot.ini" "kitty/kitty.conf:terminal/kitty.conf")

# ---------------------------------------------------------------- blocks ----

# lay_blocks: the bar without a background of its own, so the pane behind it
# shows, and the terminals almost clear, so that theirs does. foot asks Hyprland for the
# blur behind it itself (blur=yes); without that Hyprland leaves it sharp.
# kitty does not ask, and is blurred like any window.
lay_blocks() {
  local pair file
  overlay "$ROOT/override/bar.toml" | block_put "$SHELL_TOML" "$LOOK_BEGIN" "$LOOK_END"
  for pair in "${TERMINALS[@]}"; do
    file=$CONFIG/${pair%%:*}
    [ -f "$file" ] || continue
    block_put "$file" "$LOOK_BEGIN" "$LOOK_END" <"$ROOT/override/${pair##*:}"
  done
}

unlay_blocks() {
  local pair
  block_drop "$SHELL_TOML" "$LOOK_BEGIN" "$LOOK_END"
  for pair in "${TERMINALS[@]}"; do
    block_drop "$CONFIG/${pair%%:*}" "$LOOK_BEGIN" "$LOOK_END"
  done
}

# ------------------------------------------------------------------ apps ----

# What GTK and Qt apps draw behind their content: next to nothing, so that the
# glass behind a window shows, with white text, whatever the theme.
# The files stay in this checkout and are only linked, so editing them edits
# the repository; qt6ct.conf alone is written for the machine, because qt6ct
# wants an absolute path to the colours.
APP_LINKS=(
  "gtk-3.0/gtk.css:gtk-3.0/gtk.css"
  "gtk-3.0/vetro.css:gtk-3.0/vetro.css"
  "gtk-3.0/settings.ini:gtk-3.0/settings.ini"
  "gtk-4.0/gtk.css:gtk-4.0/gtk.css"
  "gtk-4.0/vetro.css:gtk-4.0/vetro.css"
  "gtk-4.0/settings.ini:gtk-4.0/settings.ini"
  "qt6ct/vetro.colors:qt6ct/colors/vetro.colors"
  "kvantum/kvantum.kvconfig:Kvantum/kvantum.kvconfig"
  "kvantum/vetro:Kvantum/vetro"
)

# render_qt6ct: qt6ct.conf.in with this home folder in it, kept in the state
# folder.
render_qt6ct() {
  local line
  mkdir -p "$STATE"
  while IFS= read -r line || [ -n "$line" ]; do
    printf '%s\n' "${line//@HOME@/$HOME}"
  done <"$ROOT/override/qt6ct/qt6ct.conf.in" >"$STATE/qt6ct.conf"
}

lay_apps() {
  local pair
  for pair in "${APP_LINKS[@]}"; do
    link "$ROOT/override/${pair%%:*}" "$CONFIG/${pair##*:}"
  done
  render_qt6ct
  link "$STATE/qt6ct.conf" "$CONFIG/qt6ct/qt6ct.conf"
}

unlay_apps() {
  local pair
  for pair in "${APP_LINKS[@]}"; do
    unlink_ours "$CONFIG/${pair##*:}"
  done
  unlink_ours "$CONFIG/qt6ct/qt6ct.conf"
  rm -f "$STATE/qt6ct.conf"
  prune "$CONFIG/gtk-3.0" "$CONFIG/gtk-4.0" "$CONFIG/Kvantum" "$CONFIG/qt6ct/colors" "$CONFIG/qt6ct"
  unlay_browser
}

# --------------------------------------------------------------- browser ----

# The browser's own frame (Brave, Chromium, Chrome, Edge). Omarchy hands it
# the theme's background through the browsers' policy, and Chromium makes a
# whole palette of that one colour, so a theme's blue-grey comes out purple.
# While Vetro is on the browser is handed the glass's own dark grey instead,
# which Chromium makes greys of. It goes through Omarchy's own helper, which
# Omarchy's sudoers rule lets run without a password; where that rule is
# missing nothing is asked and the theme's colour stays. Every change of theme
# hands the theme's colour back first, so the hook lays the grey again (in
# `vetro glass load`). `vetro off` has Omarchy put the theme's colour back,
# and `touch ~/.config/vetro/browser-themed` keeps it while Vetro is on.
BROWSER_GREY=161616
BROWSER_POLICIES=(chromium opt/chrome opt/edge brave)
BROWSER_HELPER=/usr/bin/omarchy-theme-set-browser-policy

# browser_colours: the colour each browser's policy names, one a line.
browser_colours() {
  local dir file
  for dir in "${BROWSER_POLICIES[@]}"; do
    file=$POLICY_ROOT/$dir/policies/managed/color.json
    if [ -r "$file" ]; then jq -r '.BrowserThemeColor // empty' "$file" 2>/dev/null || true; fi
  done
}

# browser_is RRGGBB: every browser that has a policy names this colour.
browser_is() {
  [ "$(browser_colours | tr 'A-F' 'a-f' | sort -u)" = "#$1" ]
}

# Whether sudo runs Omarchy's helper without asking for a password: the long
# listing names the matching rule's tags, and runs and asks nothing.
browser_helper_free() {
  command -v sudo >/dev/null 2>&1 || return 1
  sudo -n -l -l "$BROWSER_HELPER" "$BROWSER_GREY" 2>/dev/null | grep -q '!authenticate'
}

# The browsers that are running read their policy again, as Omarchy has them
# do after a change of theme.
browser_refresh() {
  local pair
  for pair in chromium:chromium chrome:google-chrome-stable msedge:microsoft-edge-stable brave:brave; do
    if command -v "${pair#*:}" >/dev/null 2>&1 && pgrep -x "${pair%%:*}" >/dev/null 2>&1; then
      "${pair#*:}" --refresh-platform-policy --no-startup-window >/dev/null 2>&1 &
    fi
  done
}

lay_browser() {
  if [ -e "$CONFIG/vetro/browser-themed" ]; then
    unlay_browser
    return 0
  fi
  if [ -z "$(browser_colours)" ] || browser_is "$BROWSER_GREY"; then return 0; fi
  if ! command -v omarchy-theme-set-browser-policy >/dev/null 2>&1 || ! browser_helper_free; then return 0; fi
  omarchy-theme-set-browser-policy "$BROWSER_GREY" </dev/null >/dev/null 2>&1 || return 0
  browser_refresh
}

# The theme's colour back, through Omarchy's own command, and only where the
# grey is Vetro's: a colour set some other way is left alone.
unlay_browser() {
  browser_is "$BROWSER_GREY" || return 0
  if ! command -v omarchy-theme-set-browser >/dev/null 2>&1 || ! browser_helper_free; then return 0; fi
  omarchy-theme-set-browser </dev/null >/dev/null 2>&1 || true
}

# --------------------------------------------------------------- plugins ----

# needs_notifyd PLUGIN: whether the plugin shows what vetro-notifyd keeps.
needs_notifyd() {
  case $1 in
    vetro.notifications | vetro.center | vetro.center-button) return 0 ;;
  esac
  return 1
}

# lay_plugins: Vetro's notifications, volume display, lock screen, bar glass,
# notification centre and bell, and its menu in the Omarchy menu's place. With
# no shell running there is nothing to ask; the next `vetro on` from a session
# does it.
lay_plugins() {
  local plugin
  if ! shell_alive; then
    echo "  the shell isn't running: the plugins are enabled by 'vetro on' from a session"
    return 0
  fi
  omarchy-shell shell rescanPlugins >/dev/null 2>&1 || true
  for plugin in "${CORE_PLUGINS[@]}"; do
    if needs_notifyd "$plugin" && [ ! -x "$HOME/.local/bin/vetro-notifyd" ]; then
      echo "  $plugin is left off: the notification daemon is not built (install.sh builds it), so Omarchy's notifications stay"
      continue
    fi
    wait_for_plugin "$plugin" || warn "the shell does not know $plugin (install.sh links it)"
    omarchy plugin enable "$plugin" >/dev/null 2>&1 || warn "could not enable $plugin"
  done
  lay_bar_clones
  lay_bar
  # A button that is enabled lands at the end of its side of the bar. The menu
  # belongs at the start, and passthru's Windows button, where passthru has put
  # one in, beside it, ahead of the workspaces: put back every time, since off
  # and on enable the menu anew.
  omarchy bar move vetro.menu --section left --index 0 >/dev/null 2>&1 || true
  if [ -e "$PLUGINS_DIR/passthru.windows" ]; then
    omarchy bar move passthru.windows --after vetro.menu >/dev/null 2>&1 || true
  fi
}

# bar_built: Vetro's bar is built, from the Omarchy bar that is installed now
# and by the builder as it is now.
bar_built() {
  local stamp
  [ -f "$BAR_OUT/built-from" ] && [ -f "$BAR_OUT/Bar.qml" ] || return 1
  stamp=$(python3 "$ROOT/tools/bar-build.py" --stamp "$BAR_SOURCE" 2>/dev/null) || return 1
  [ "$(cat "$BAR_OUT/built-from")" = "$stamp" ]
}

# bar_build: build Vetro's bar unless it is built already, and link it in as a
# shell plugin. When it cannot be built, what is left of an earlier build goes
# (it was made for another Omarchy) and the reason is said.
bar_build() {
  local why
  if ! bar_built; then
    if ! why=$(python3 "$ROOT/tools/bar-build.py" "$BAR_SOURCE" "$BAR_OUT" "$ROOT/shell/bar" 2>&1); then
      unlink_ours "$BAR_PLUGIN" >/dev/null
      echo "  Vetro's bar is not built, and Omarchy's own is used: ${why:-python3 is needed}"
      return 1
    fi
  fi
  link "$BAR_OUT" "$BAR_PLUGIN" >/dev/null
}

# bar_drop: the built bar and its link go.
bar_drop() {
  unlink_ours "$BAR_PLUGIN"
  if [ -f "$BAR_OUT/built-from" ]; then
    rm -rf -- "$BAR_OUT"
  fi
}

# bar_in_use: the id of the bar the shell is set to show.
bar_in_use() {
  jq -r '.bar.id // "omarchy.bar"' "$SHELL_JSON" 2>/dev/null || echo omarchy.bar
}

# lay_bar: Vetro's bar in the place of Omarchy's own. A bar of somebody
# else's that you chose is left where it is. The shell shows a bar it is
# switched to properly only once it has started afresh, which `vetro on` sees
# to; BAR_SWITCHED tells a caller that does not.
BAR_SWITCHED=no
lay_bar() {
  local now
  now=$(bar_in_use)
  if ! bar_build; then
    if [ "$now" = vetro.bar ]; then
      omarchy plugin disable vetro.bar >/dev/null 2>&1 || true
      BAR_SWITCHED=yes
    fi
    return 0
  fi
  [ "$now" = omarchy.bar ] || return 0
  omarchy-shell shell rescanPlugins >/dev/null 2>&1 || true
  wait_for_plugin vetro.bar || warn "the shell does not know vetro.bar"
  if omarchy bar use vetro.bar >/dev/null 2>&1; then
    BAR_SWITCHED=yes
  else
    warn "could not switch to Vetro's bar: Omarchy's own stays"
  fi
}

# unlay_bar: Omarchy's own bar comes back.
unlay_bar() {
  [ "$(bar_in_use)" = vetro.bar ] || return 0
  omarchy plugin disable vetro.bar >/dev/null 2>&1 || omarchy bar use omarchy.bar >/dev/null 2>&1 || true
}

# lay_bar_clones: each of Vetro's bar widgets takes its original's place, where
# the original is on the bar. Enabling a clone is what swaps them, there and
# then, so nothing is moved.
lay_bar_clones() {
  local pair clone
  for pair in "${BAR_CLONES[@]}"; do
    clone=${pair%%:*}
    on_bar "$clone" && continue
    on_bar "${pair##*:}" || continue
    wait_for_plugin "$clone" || warn "the shell does not know $clone (install.sh links it)"
    omarchy plugin enable "$clone" >/dev/null 2>&1 || warn "could not enable $clone"
  done
}

# unlay_plugins: Omarchy's own notifications, volume display, lock screen and
# menu come back. Disabling a clone does that by itself; asking for the
# originals as well settles a shell that lost track.
unlay_plugins() {
  local plugin
  if ! shell_alive; then
    echo "  the shell isn't running: its plugins are switched back by 'vetro off' from a session"
    return 0
  fi
  for plugin in "${CORE_PLUGINS[@]}"; do
    omarchy plugin disable "$plugin" >/dev/null 2>&1 || true
  done
  unlay_bar
  # A bar widget that is disabled gives its place back to its original.
  for plugin in "${BAR_CLONES[@]}"; do
    on_bar "${plugin%%:*}" || continue
    omarchy plugin disable "${plugin%%:*}" >/dev/null 2>&1 || true
  done
  for plugin in "${OMARCHY_PLUGINS[@]}"; do
    omarchy plugin enable "$plugin" >/dev/null 2>&1 || true
  done
}

# ---------------------------------------------------------------- addons ----

lay_addons() {
  local name
  while IFS= read -r name; do
    addon_run "$name" on
  done < <(installed_addons)
}

unlay_addons() {
  local name
  while IFS= read -r name; do
    addon_run "$name" off
  done < <(installed_addons)
}
