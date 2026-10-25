#!/bin/bash
# The addons: the launch addon's on and off and its command, the hook of the
# launch theme (it sets GTK for that theme and takes back only what it set when
# another is chosen), and the core's hook, which does nothing without the
# command and asks only for the glass with it.
# shellcheck source-path=SCRIPTDIR
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"
make_bin

new_home addons
run "$REPO/install.sh" --no-build >/dev/null 2>&1 || fail "install.sh failed"

# --- the launch addon follows on and off
forget_calls
run "$REPO/addons/launch/addon.sh" status >"$WORK/status.out" 2>&1
has "status says it is installed" "$WORK/status.out" "launch: installed"
run "$REPO/addons/launch/addon.sh" on >/dev/null 2>&1 || fail "addon.sh on failed"
ok "on starts the daemon" test -e "$H/.standin/active-vetro-launchd.service"
is "and enables the cards" true "$(run omarchy-plugin-list --json | jq -r '.[] | select(.id == "vetro.cards") | .enabled')"
run "$REPO/addons/launch/addon.sh" off >/dev/null 2>&1 || fail "addon.sh off failed"
no "off stops the daemon" test -e "$H/.standin/active-vetro-launchd.service"
is "and disables the cards" false "$(run omarchy-plugin-list --json | jq -r '.[] | select(.id == "vetro.cards") | .enabled')"
run "$REPO/addons/launch/addon.sh" nonsense >/dev/null 2>&1
is "an unknown word is refused" 2 "$?"
run "$REPO/addons/theme/addon.sh" on >/dev/null 2>&1
is "the theme addon has nothing to do on or off" 0 "$?"
run vetro on >/dev/null 2>&1
ok "vetro on starts the daemon through the addon" test -e "$H/.standin/active-vetro-launchd.service"
run vetro off >/dev/null 2>&1
no "vetro off stops it" test -e "$H/.standin/active-vetro-launchd.service"

# --- the launch command and the daemon
mkdir -p "$H/.cache/vetro"
now=$(date +%s)
printf '{"upcoming": [{"title": "Falcon 9 Block 5", "net_epoch": %d, "net_precision": "MIN"}]}\n' "$((now + 5 * 3600 + 120))" >"$H/.cache/vetro/launches.json"
says "next names the launch" "FALCON 9 BLOCK 5" run vetro-launch next --short
says "and counts to it" "05:0" run vetro-launch next --short
run vetro-launch status >"$WORK/lstatus.out" 2>&1
is "status without the daemon fails" 1 "$?"
has "and says what to check" "$WORK/lstatus.out" "systemctl --user status vetro-launchd"
says "the daemon is the one built for this machine" "vetro-launchd (stand-in)" run vetro-launchd fetch
run vetro-launch static >"$WORK/lstatic.out" 2>&1
is "the video wallpaper's requests are gone from the command" 2 "$?"

# --- the launch theme's hook
HOOK=$REPO/addons/theme/hook/60-vetro-launch-theme.sh
run bash "$HOOK" launch
store=$H/.standin/gsettings
has "the launch theme sets GTK's theme" "$store" "gtk-theme 'adw-gtk3-dark'"
has "and the font" "$store" "font-name 'D-DIN 11'"
has "and the cursor" "$store" "cursor-theme 'XCursor-Pro-Dark'"
run bash "$HOOK" tokyo-night
lacks "another theme takes back the font" "$store" "font-name"
lacks "and the cursor" "$store" "cursor-theme"
lacks "and the accent" "$store" "accent-color"
has "but not what Omarchy sets for every theme" "$store" "gtk-theme 'adw-gtk3-dark'"
run bash "$HOOK" launch
run gsettings set org.gnome.desktop.interface font-name "'Inter 11'"
run bash "$HOOK" tokyo-night
has "a font changed since is left alone" "$store" "font-name 'Inter 11'"
lacks "while the rest is taken back" "$store" "cursor-theme"
rm -f "$BIN/gsettings"
run bash "$HOOK" launch
is "without gsettings the hook does nothing and succeeds" 0 "$?"
ln -s "$REPO/tests/standins/gsettings" "$BIN/gsettings"

# --- the core's hook after a theme
CORE_HOOK=$REPO/hooks/theme-set.d/50-vetro.sh
forget_calls
run bash "$CORE_HOOK" anything
is "the hook succeeds" 0 "$?"
run vetro on >/dev/null 2>&1
rm -f "$H/.local/bin/vetro"
forget_calls
run bash "$CORE_HOOK" anything
is "without the command it succeeds and asks for nothing" 0 "$?"
if called "hyprctl"; then fail "the hook asked for something without the command"; fi

finish
