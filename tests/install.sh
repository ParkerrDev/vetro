#!/bin/bash
# install.sh and uninstall.sh on a home folder that has nothing of Vetro's and
# nothing of the earlier projects: what is linked and where, that a fresh
# install changes nothing on screen, that a second run changes nothing at all,
# that a file of the user's in the way is kept and put back, that an addon can
# be left out, and that uninstalling leaves the home folder as it was found.
# shellcheck source-path=SCRIPTDIR
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"
make_bin

CORE=(vetro.notifications vetro.osd vetro.lock vetro.barglass vetro.center vetro.center-button vetro.menu vetro.workspaces)

# links_ok: every link the core installs leads where it should.
links_ok() {
  local when=$1 plugin
  is "$when: the command" "$REPO/bin/vetro" "$(readlink "$H/.local/bin/vetro")"
  is "$when: what the menu starts apps with" "$REPO/bin/vetro-focus-or-open" "$(readlink "$H/.local/bin/vetro-focus-or-open")"
  is "$when: the app icon" "$REPO/assets/branding/vetro.svg" "$(readlink "$H/.local/share/icons/hicolor/scalable/apps/dev.vetro.Glass.svg")"
  is "$when: the symbolic icon" "$REPO/assets/branding/vetro-symbolic.svg" "$(readlink "$H/.local/share/icons/hicolor/symbolic/apps/dev.vetro.Glass-symbolic.svg")"
  is "$when: the Lua file" "$REPO/hypr/vetro.lua" "$(readlink "$H/.config/hypr/vetro.lua")"
  is "$when: the hook after a theme" "$REPO/hooks/theme-set.d/50-vetro.sh" "$(readlink "$H/.config/omarchy/hooks/theme-set.d/50-vetro.sh")"
  is "$when: the hook after an update" "$REPO/engine/post-update.sh" "$(readlink "$H/.config/omarchy/hooks/post-update.d/50-vetro-glass.sh")"
  is "$when: the library" "$REPO/shell/vetro.lib" "$(readlink "$H/.config/omarchy/plugins/vetro.lib")"
  is "$when: the fonts" "$REPO/shell/fonts" "$(readlink "$H/.local/share/fonts/vetro")"
  no "$when: the shell settings are not a link" test -L "$H/.config/vetro/shell.json"
  is "$when: they are a copy of their own" 0.2 "$(jq -r .panels.opacity "$H/.config/vetro/shell.json")"
  for plugin in "${CORE[@]}"; do
    is "$when: $plugin" "$REPO/shell/$plugin" "$(readlink "$H/.config/omarchy/plugins/$plugin")"
    ok "$when: $plugin has a manifest the link leads to" test -f "$H/.config/omarchy/plugins/$plugin/manifest.json"
  done
  # Vetro's bar is built in this home from Omarchy's own, and linked in.
  is "$when: Vetro's bar" "$H/.local/state/vetro/bar" "$(readlink "$H/.config/omarchy/plugins/vetro.bar")"
  is "$when: it says it is a bar, in the place of Omarchy's" "bar omarchy.bar" \
    "$(jq -r '"\(.kinds[0]) \(.omarchy.clonedFrom)"' "$H/.config/omarchy/plugins/vetro.bar/manifest.json")"
  has "$when: its glass is in its own surface" "$H/.config/omarchy/plugins/vetro.bar/Bar.qml" 'WlrLayershell.namespace: "vetro:bar"'
  has "$when: with the pane that draws it" "$H/.config/omarchy/plugins/vetro.bar/Bar.qml" "VetroPane {"
  is "$when: the pane is Vetro's own file" "$REPO/shell/bar/VetroPane.qml" "$(readlink "$H/.config/omarchy/plugins/vetro.bar/VetroPane.qml")"
  has "$when: nothing on it is dragged" "$H/.config/omarchy/plugins/vetro.bar/Bar.qml" "readonly property bool vetroGestures: false"
  is "$when: the model beside it is Omarchy's, untouched" "$(md5sum <"$REPO/tests/fixtures/bar/BarModel.js")" \
    "$(md5sum <"$H/.config/omarchy/plugins/vetro.bar/BarModel.js")"
}

# --- a fresh install, twice
new_home fresh
cp "$H/.config/hypr/hyprland.lua" "$WORK/hyprland.lua.before"
cp "$H/.config/omarchy/shell.json" "$WORK/shell.json.before"
echo "mine" >"$H/.local/bin/vetro"
mkdir -p "$H/.local/share/icons/hicolor/scalable/apps"
echo "my icon" >"$H/.local/share/icons/hicolor/scalable/apps/dev.vetro.Glass.svg"
snapshot >"$WORK/snapshot.before"
run "$REPO/install.sh" --no-build >"$WORK/install.1" 2>&1 || fail "install.sh (first run) failed: $(tail -n 3 "$WORK/install.1")"
links_ok "first run"
is "a file of the user's in the way is kept" mine "$(cat "$H/.local/bin/vetro.pre-vetro")"
is "an existing icon is kept" "my icon" "$(cat "$H/.local/share/icons/hicolor/scalable/apps/dev.vetro.Glass.svg.pre-vetro")"
is "hyprland.lua loads Vetro once" 1 "$(grep -c 'hypr.vetro' "$H/.config/hypr/hyprland.lua")"
is "with its marks, once" 2 "$(grep -c '^-- [<>]\{3\} vetro' "$H/.config/hypr/hyprland.lua")"
is "the user's lines in hyprland.lua are as they were" "$(cat "$WORK/hyprland.lua.before")" "$(head -n "$(wc -l <"$WORK/hyprland.lua.before")" "$H/.config/hypr/hyprland.lua")"
ok "a fresh install is off" test -e "$H/.local/state/vetro/off"
is "so status says off" off "$(run vetro status | sed -n 1p)"
no "nothing is laid in shell.toml" test -e "$H/.config/omarchy/shell.toml"
is "the shell's plugins are as they were" "$(jq -S . "$WORK/shell.json.before")" "$(jq -S . "$H/.config/omarchy/shell.json")"
is "the theme is not touched" wallhaven-k8j6w1 "$(theme)"
if called "omarchy theme set"; then fail "install set a theme"; fi
if called "hyprctl reload"; then fail "install reloaded Hyprland"; fi
if called "omarchy-restart-shell"; then fail "install restarted the shell"; fi
ok "the launch addon is installed" test -e "$H/.local/state/vetro/addons/launch"
ok "the theme addon is installed" test -e "$H/.local/state/vetro/addons/theme"
is "the launch command" "$REPO/addons/launch/bin/vetro-launch" "$(readlink "$H/.local/bin/vetro-launch")"
is "the launch daemon, built" "$H/.local/state/vetro/bin/vetro-launchd" "$(readlink "$H/.local/bin/vetro-launchd")"
ok "and it runs" test -x "$H/.local/bin/vetro-launchd"
called "cargo build --release --locked" || fail "the launch daemon was not built with its locked versions"
is "the daemon's service" "$REPO/addons/launch/systemd/vetro-launchd.service" "$(readlink "$H/.config/systemd/user/vetro-launchd.service")"
no "the launch settings are not a link" test -L "$H/.config/vetro/launches.toml"
ok "they are a copy of their own" grep -q '^\[weather\]' "$H/.config/vetro/launches.toml"
is "the cards" "$REPO/addons/launch/plugin/vetro.cards" "$(readlink "$H/.config/omarchy/plugins/vetro.cards")"
is "the launch theme" "$REPO/addons/theme/launch" "$(readlink "$H/.config/omarchy/themes/launch")"
is "and the hook that sets GTK for it" "$REPO/addons/theme/hook/60-vetro-launch-theme.sh" "$(readlink "$H/.config/omarchy/hooks/theme-set.d/60-vetro-launch-theme.sh")"
no "no decode drop-in: there is no video to decode" test -e "$H/.config/systemd/user/vetro-launchd.service.d/intel-decode.conf"
no "nothing started the daemon" test -e "$H/.standin/active-vetro-launchd.service"
snapshot >"$WORK/snapshot.1"
run "$REPO/install.sh" --no-build >"$WORK/install.2" 2>&1 || fail "install.sh (second run) failed"
snapshot >"$WORK/snapshot.2"
is "a second run changes nothing" "$(cat "$WORK/snapshot.1")" "$(cat "$WORK/snapshot.2")"

# --- install changes nothing a running Vetro has laid
run vetro on >/dev/null 2>&1
snapshot >"$WORK/snapshot.on"
forget_calls
run "$REPO/install.sh" --no-build >"$WORK/install.3" 2>&1 || fail "install.sh over a Vetro that is on failed"
snapshot >"$WORK/snapshot.on2"
is "installing over a Vetro that is on changes nothing" "$(cat "$WORK/snapshot.on")" "$(cat "$WORK/snapshot.on2")"
is "and it stays on" on "$(run vetro status | sed -n 1p)"
if called "hyprctl reload"; then fail "installing over a Vetro that is on reloaded Hyprland"; fi

# --- an earlier Vetro that is on, from before it had workspace numbers of its own
cp "$H/.config/omarchy/shell.json" "$WORK/shell.json.on"
jq '(.bar.layout.left[] | select(.id == "vetro.workspaces") | .id) = "omarchy.workspaces"' "$WORK/shell.json.on" >"$H/.config/omarchy/shell.json"
rm "$H/.config/omarchy/plugins/vetro.workspaces"
forget_calls
run "$REPO/install.sh" --no-build >"$WORK/install.4" 2>&1 || fail "install.sh over an earlier Vetro that is on failed"
is "installing over it brings in the plugin this version added" "vetro.menu vetro.workspaces" "$(jq -r '.bar.layout.left | map(.id) | join(" ")' "$H/.config/omarchy/shell.json")"
is "and leaves the rest of the shell's file as it was" "$(jq -S . "$WORK/shell.json.on")" "$(jq -S . "$H/.config/omarchy/shell.json")"
if called "hyprctl reload"; then fail "bringing a plugin in reloaded Hyprland"; fi
if called "omarchy-restart-shell"; then fail "bringing a plugin in restarted the shell"; fi

# --- an earlier Vetro that is on, from before it had a bar of its own
jq 'del(.bar.id)' "$WORK/shell.json.on" >"$H/.config/omarchy/shell.json"
rm "$H/.config/omarchy/plugins/vetro.bar"
rm -r "$H/.local/state/vetro/bar"
: >"$H/.standin/locked"
forget_calls
run "$REPO/install.sh" --no-build >"$WORK/install.5" 2>&1 || fail "install.sh over a Vetro without a bar failed"
is "installing over it switches the shell to Vetro's bar" vetro.bar "$(jq -r '.bar.id' "$H/.config/omarchy/shell.json")"
if called "omarchy-restart-shell"; then fail "the shell was restarted while the screen was locked"; fi
has "and says the bar changes later, the screen being locked" "$WORK/install.5" "the screen is locked now"
rm "$H/.standin/locked"
jq 'del(.bar.id)' "$WORK/shell.json.on" >"$H/.config/omarchy/shell.json"
forget_calls
run "$REPO/install.sh" --no-build >"$WORK/install.6" 2>&1 || fail "install.sh over a Vetro on Omarchy's bar failed"
is "with the screen unlocked it switches" vetro.bar "$(jq -r '.bar.id' "$H/.config/omarchy/shell.json")"
called "omarchy-restart-shell" || fail "the shell was not started afresh after its bar was switched"
if called "hyprctl reload"; then fail "switching the bar reloaded Hyprland"; fi
is "and the rest of the shell's file is as it was" "$(jq -S . "$WORK/shell.json.on")" "$(jq -S . "$H/.config/omarchy/shell.json")"

# --- uninstall from the on state: back to what was found
run "$REPO/uninstall.sh" >"$WORK/uninstall.1" 2>&1 || fail "uninstall.sh failed: $(tail -n 3 "$WORK/uninstall.1")"
for link in .local/bin/vetro .config/hypr/vetro.lua .config/omarchy/hooks/theme-set.d/50-vetro.sh \
  .config/omarchy/hooks/post-update.d/50-vetro-glass.sh .config/omarchy/plugins/vetro.lib \
  .local/share/fonts/vetro .local/bin/vetro-launch .local/bin/vetro-launchd \
  .config/systemd/user/vetro-launchd.service .config/omarchy/plugins/vetro.cards .config/omarchy/themes/launch \
  .config/omarchy/hooks/theme-set.d/60-vetro-launch-theme.sh; do
  no "uninstall removes $link" test -L "$H/$link"
done
for plugin in "${CORE[@]}"; do
  no "uninstall removes $plugin" test -L "$H/.config/omarchy/plugins/$plugin"
done
no "uninstall removes the link to Vetro's bar" test -L "$H/.config/omarchy/plugins/vetro.bar"
is "and the shell is back on Omarchy's own bar" null "$(jq -r '.bar.id' "$H/.config/omarchy/shell.json")"
is "uninstall puts hyprland.lua back as it was" "$(cat "$WORK/hyprland.lua.before")" "$(cat "$H/.config/hypr/hyprland.lua")"
is "uninstall puts the user's file back" mine "$(cat "$H/.local/bin/vetro")"
is "uninstall restores the earlier icon" "my icon" "$(cat "$H/.local/share/icons/hicolor/scalable/apps/dev.vetro.Glass.svg")"
no "uninstall removes the symbolic icon" test -L "$H/.local/share/icons/hicolor/symbolic/apps/dev.vetro.Glass-symbolic.svg"
is "uninstall leaves the plugins as they were" "$(jq -S 'del(.disabledPlugins, .cloneSourceRestores)' "$WORK/shell.json.before")" "$(jq -S 'del(.disabledPlugins, .cloneSourceRestores)' "$H/.config/omarchy/shell.json")"
is "uninstall leaves the theme alone" wallhaven-k8j6w1 "$(theme)"
if called "omarchy theme set"; then fail "uninstall set a theme"; fi
no "uninstall leaves no state" test -e "$H/.local/state/vetro"
snapshot >"$WORK/snapshot.after"
# (The shell rewrites its own shell.json whenever a plugin is switched, so that
# one is compared by what is in it, above.)
diff <(grep -v 'omarchy/shell.json' "$WORK/snapshot.before") <(grep -v 'omarchy/shell.json' "$WORK/snapshot.after") >"$WORK/snapshot.diff" 2>&1 || fail "uninstall left the home folder different from how it was found: $(head -n 6 "$WORK/snapshot.diff" | tr '\n' ' ')"
run "$REPO/uninstall.sh" >/dev/null 2>&1 || fail "uninstall.sh run twice failed"

# --- an addon can be left out, and the core runs without it
new_home lean
run "$REPO/install.sh" --no-build --without launch --without theme >"$WORK/lean.out" 2>&1 || fail "install.sh --without failed"
links_ok "without addons"
no "no launch daemon" test -e "$H/.local/bin/vetro-launchd"
no "no cards" test -e "$H/.config/omarchy/plugins/vetro.cards"
no "no launch theme" test -e "$H/.config/omarchy/themes/launch"
no "no addon mark" test -e "$H/.local/state/vetro/addons/launch"
run vetro on >/dev/null 2>&1 || fail "vetro on failed without the addons"
is "it is on" on "$(run vetro status | sed -n 1p)"
is "the cards are not enabled" 0 "$(run omarchy-plugin-list --json | jq '[.[] | select(.id == "vetro.cards")] | length')"
if called "vetro-launchd"; then fail "the launch daemon was asked for without the addon"; fi
run vetro off >/dev/null 2>&1
run "$REPO/uninstall.sh" >/dev/null 2>&1 || fail "uninstall.sh failed without the addons"

# --- the launch addon needs cargo to build its daemon; without it, it is left
# out and says so
new_home nocargo
rm -f "$BIN/cargo"
run "$REPO/install.sh" --no-build >"$WORK/nocargo.out" 2>&1 || fail "install.sh without cargo failed"
has "it says the launch addon is left out" "$WORK/nocargo.out" "cargo is not installed: the launch addon is left out"
no "no daemon is linked" test -e "$H/.local/bin/vetro-launchd"
no "and no mark is left" test -e "$H/.local/state/vetro/addons/launch"
ok "the theme addon does not mind" test -L "$H/.config/omarchy/themes/launch"
run vetro on >/dev/null 2>&1 || fail "vetro on without the launch addon failed"
no "the daemon is not asked for" test -e "$H/.standin/active-vetro-launchd.service"
run "$REPO/uninstall.sh" >/dev/null 2>&1 || fail "uninstall.sh without the launch addon failed"
ln -s "$REPO/tests/standins/cargo" "$BIN/cargo"

# --- what the video wallpaper left (its settings and its decoder drop-in) is
# taken away by an install
new_home leftovers
mkdir -p "$H/.config/vetro" "$H/.config/systemd/user/vetro-launchd.service.d"
ln -s "$REPO/addons/launch/wallpaper.toml" "$H/.config/vetro/wallpaper.toml"
ln -s "$REPO/addons/launch/systemd/vetro-launchd.service.d/intel-decode.conf" \
  "$H/.config/systemd/user/vetro-launchd.service.d/intel-decode.conf"
run "$REPO/install.sh" --no-build >/dev/null 2>&1
no "the wallpaper's settings are gone" test -L "$H/.config/vetro/wallpaper.toml"
no "and the decoder drop-in, with its folder" test -e "$H/.config/systemd/user/vetro-launchd.service.d"
run "$REPO/uninstall.sh" >/dev/null 2>&1

# --- --purge
new_home purge
run "$REPO/install.sh" --no-build >/dev/null 2>&1
mkdir -p "$H/.local/lib/vetro" "$H/.local/share/vetro/clips"
echo engine >"$H/.local/lib/vetro/hyprglass.so"
echo clip >"$H/.local/share/vetro/clips/a.mp4"
echo '{}' >"$H/.local/state/vetro/misc.json"
run "$REPO/uninstall.sh" >"$WORK/keep.out" 2>&1
ok "without --purge the engine stays" test -s "$H/.local/lib/vetro/hyprglass.so"
ok "and the clips" test -s "$H/.local/share/vetro/clips/a.mp4"
ok "and the saved state" test -s "$H/.local/state/vetro/misc.json"
has "and it says so" "$WORK/keep.out" "kept the built engine"
run "$REPO/install.sh" --no-build >/dev/null 2>&1
run "$REPO/uninstall.sh" --purge >/dev/null 2>&1
no "--purge takes the engine" test -e "$H/.local/lib/vetro"
no "--purge takes the clips" test -e "$H/.local/share/vetro"
no "--purge takes the state" test -e "$H/.local/state/vetro"

# --- the words and the refusals
no "an unknown word is refused" run "$REPO/install.sh" --nonsense
new_home nolua
rm -f "$H/.config/hypr/hyprland.lua"
no "no Hyprland config, no install" run "$REPO/install.sh" --no-build

finish
