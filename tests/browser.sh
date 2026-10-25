#!/bin/bash
# The browser's colour: the glass's grey while Vetro is on, given again after
# each change of theme, the theme's own when Vetro is off or asked to keep it,
# never through a password, and nothing asked where no browser has a policy.
# The browsers' policy folders are in the scratch home, not under /etc.
# shellcheck source-path=SCRIPTDIR
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"
make_bin

new_home browser
P=$H/etc
policy() { jq -r '.BrowserThemeColor' "$P/$1/policies/managed/color.json" 2>/dev/null; }
# Whether the helper itself ran (and not only the question to sudo about it).
ran_helper() { calls | grep -q '^omarchy-theme-set-browser-policy '; }
theme_colour() { echo "$1" >"$H/.local/state/omarchy/current/theme/chromium.theme"; }
mkdir -p "$P/brave/policies/managed" "$P/chromium/policies/managed" "$H/.local/state/omarchy/current/theme"
echo '{"BraveRewardsDisabled": true}' >"$P/brave/policies/managed/policy.json"
theme_colour "30,30,46"
run omarchy-theme-set-browser
is "the theme's colour is in Brave's policy to begin with" "#1e1e2e" "$(policy brave)"

run "$REPO/install.sh" --no-build >"$WORK/install.out" 2>&1 || fail "install.sh did not run: $(tail -n 3 "$WORK/install.out")"
says "off, status names the browser's colour" "browser: #1e1e2e, not the glass's grey" run vetro status

# --- on: the grey, through Omarchy's helper and its rule, and nothing else
forget_calls
run vetro on >/dev/null 2>&1 || fail "vetro on failed"
is "on makes Brave's frame grey" "#161616" "$(policy brave)"
is "and Chromium's" "#161616" "$(policy chromium)"
is "Brave's other policy is left as it was" '{"BraveRewardsDisabled": true}' "$(cat "$P/brave/policies/managed/policy.json")"
no "no policy folder is made for a browser that has none" test -e "$P/opt"
says "status says so" "browser: grey, the glass's" run vetro status
called "sudo -n -l -l /usr/bin/omarchy-theme-set-browser-policy" || fail "on did not ask whether the helper needs a password"
if calls | grep '^sudo' | grep -qv -- '^sudo -n -l -l '; then fail "Vetro ran something through sudo itself"; fi
is "the helper was asked once" 1 "$(calls | grep -c '^omarchy-theme-set-browser-policy 161616')"
forget_calls
run vetro glass load
if ran_helper; then fail "glass load set the grey again when it was there"; fi

# --- a change of theme gives the theme's colour, and the hook the grey again
theme_colour "26,27,38"
forget_calls
run omarchy theme set tokyo-night
called "omarchy-theme-set-browser-policy 1a1b26" || fail "the stand-in theme did not give its colour first"
is "after a change of theme the frame is grey again" "#161616" "$(policy brave)"

# --- kept as the theme has it
mkdir -p "$H/.config/vetro"
touch "$H/.config/vetro/browser-themed"
run vetro glass load
is "browser-themed gives the theme's colour back" "#1a1b26" "$(policy brave)"
run omarchy theme set tokyo-night
is "and a change of theme leaves it so" "#1a1b26" "$(policy brave)"
rm -f "$H/.config/vetro/browser-themed"
run vetro glass load
is "without it the grey comes back" "#161616" "$(policy chromium)"

# --- off
run vetro off >/dev/null 2>&1
is "off gives the theme's colour back" "#1a1b26" "$(policy brave)"
says "and status says it" "browser: #1a1b26, not the glass's grey" run vetro status
run omarchy theme set tokyo-night
is "a change of theme while off leaves the theme's colour" "#1a1b26" "$(policy brave)"
run vetro on >/dev/null 2>&1
run omarchy-theme-set-browser-policy 336699
run vetro off >/dev/null 2>&1
is "off leaves a colour that was set some other way" "#336699" "$(policy brave)"

# --- no rule for the helper: nothing is asked, and nothing changes
touch "$H/.standin/no-sudo-rule"
forget_calls
run vetro on >/dev/null 2>&1
is "without Omarchy's rule the colour stays" "#336699" "$(policy brave)"
if ran_helper; then fail "the helper was run without its rule"; fi
rm -f "$H/.standin/no-sudo-rule"

# --- no browser with a policy: nothing is asked at all
rm -rf "$P"
forget_calls
run vetro glass load
if called "sudo"; then fail "sudo was asked with no browser policy to change"; fi
says "status says there is none" "browser: no colour set" run vetro status

# --- uninstall gives the theme's colour back
mkdir -p "$P/brave/policies/managed"
run vetro glass load
run omarchy-theme-set-browser-policy 161616
run "$REPO/uninstall.sh" >"$WORK/uninstall.out" 2>&1 || fail "uninstall.sh did not run: $(tail -n 3 "$WORK/uninstall.out")"
is "uninstall gives the theme's colour back" "#1a1b26" "$(policy brave)"

finish
