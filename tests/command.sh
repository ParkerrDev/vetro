#!/bin/bash
# The vetro command against stand-ins: every subcommand, on and off under a
# theme that is not the launch theme and under one that is, a change of theme
# in between, the glass engine's three states, and the guard against a config
# read in a loop. No compositor, shell or service is involved.
# shellcheck source-path=SCRIPTDIR
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"
make_bin

new_home stock
run "$REPO/install.sh" --no-build >"$WORK/install.out" 2>&1 || fail "install.sh did not run: $(tail -n 3 "$WORK/install.out")"
V=$H/.local/bin/vetro
is "a fresh install leaves the command in place" "$REPO/bin/vetro" "$(readlink "$V")"

# --- before anything is switched on
is "a fresh install is off" off "$(run vetro status | sed -n 1p)"
no "off before on has nothing laid in shell.toml" test -e "$H/.config/omarchy/shell.toml"
says "status names the theme and does not own it" "theme: wallhaven-k8j6w1 (yours" run vetro status
says "status says the guard is ready" "guard: ready" run vetro status
says "status says the engine is not built" "engine: not built" run vetro status

# --- on: under a theme that is not the launch theme, and no engine yet
forget_calls
mkdir -p "$H/.config/kitty" "$H/.config/foot"
printf '# my own kitty settings\nfont_size 12\n' >"$H/.config/kitty/kitty.conf"
printf '[main]\nfont=monospace:size=11\n' >"$H/.config/foot/foot.ini"
cp "$H/.config/kitty/kitty.conf" "$WORK/kitty.before"
cp "$H/.config/foot/foot.ini" "$WORK/foot.before"
mkdir -p "$H/.config/gtk-3.0"
echo "my own gtk.css" >"$H/.config/gtk-3.0/gtk.css"
run vetro on >"$WORK/on.out" 2>&1 || fail "vetro on failed: $(tail -n 3 "$WORK/on.out")"
is "on reads as on" on "$(run vetro status | sed -n 1p)"
is "on does not touch the theme" wallhaven-k8j6w1 "$(theme)"
if called "omarchy theme set"; then fail "on set a theme"; fi
has "on lays the bar's block" "$H/.config/omarchy/shell.toml" "# >>> vetro look"
has "the block makes the bar see-through" "$H/.config/omarchy/shell.toml" "background-alpha = 0"
lacks "no glass colours without the engine" "$H/.config/omarchy/shell.toml" "# >>> vetro glass"
has "kitty gets its block" "$H/.config/kitty/kitty.conf" "background_opacity 0.2"
has "foot gets its block" "$H/.config/foot/foot.ini" "blur=yes"
has "kitty keeps the user's lines" "$H/.config/kitty/kitty.conf" "font_size 12"
is "GTK colours are linked" "$REPO/override/gtk-3.0/vetro.css" "$(readlink "$H/.config/gtk-3.0/vetro.css")"
is "Qt colours are linked" "$REPO/override/qt6ct/vetro.colors" "$(readlink "$H/.config/qt6ct/colors/vetro.colors")"
is "the Kvantum theme is linked" "$REPO/override/kvantum/vetro" "$(readlink "$H/.config/Kvantum/vetro")"
has "qt6ct.conf is written for this home" "$H/.local/state/vetro/qt6ct.conf" "color_scheme_path=$H/.config/qt6ct/colors/vetro.colors"
is "the user's own gtk.css was kept" "my own gtk.css" "$(cat "$H/.config/gtk-3.0/gtk.css.pre-vetro")"
for plugin in vetro.notifications vetro.osd vetro.lock vetro.barglass vetro.center vetro.center-button vetro.menu; do
  is "$plugin is enabled" true "$(run omarchy-plugin-list --json | jq -r --arg id "$plugin" '.[] | select(.id == $id) | .enabled')"
done
is "Omarchy's menu gives way to Vetro's" "vetro.menu" "$(jq -r '.bar.layout.left[0].id' "$H/.config/omarchy/shell.json")"
is "and its workspace numbers to Vetro's, where they stood" "vetro.menu vetro.workspaces" "$(jq -r '.bar.layout.left | map(.id) | join(" ")' "$H/.config/omarchy/shell.json")"
is "and its bar to Vetro's, built from it" vetro.bar "$(jq -r '.bar.id' "$H/.config/omarchy/shell.json")"
is "the clones switch Omarchy's originals off" 4 "$(jq -r '.disabledPlugins | length' "$H/.config/omarchy/shell.json")"
called "hyprctl reload" || fail "on did not reload Hyprland"
called "omarchy-restart-shell" || fail "on did not restart the shell"
if called "plugin load"; then fail "on loaded an engine that is not built"; fi

# --- the engine built for this Hyprland: glass load puts it in
mkdir -p "$H/.local/lib/vetro"
echo "fake engine" >"$H/.local/lib/vetro/hyprglass.so"
printf '%s\nv0.56.2\n%s\n' "$PINNED_HYPRLAND" "$PINNED_ENGINE" >"$H/.local/lib/vetro/hyprglass.built-for"
run vetro glass load || fail "glass load failed"
called "hyprctl plugin load $H/.local/lib/vetro/hyprglass.so" || fail "glass load did not load the engine from Vetro's folder"
has "the glass colours go in with the engine" "$H/.config/omarchy/shell.toml" "# >>> vetro glass"
has "the bar's own colour is black, so no theme tints the pane" "$H/.config/omarchy/shell.toml" 'background = "#000000"'
says "status says the engine is loaded" "engine: loaded" run vetro status
forget_calls
run vetro glass load
if called "plugin load"; then fail "glass load loaded the engine twice"; fi

# --- a change of theme leaves everything in place, and the hook sees to the rest
cp "$H/.config/omarchy/shell.toml" "$WORK/shell.toml.on"
run omarchy theme set tokyo-night
is "the theme is the one chosen" tokyo-night "$(theme)"
is "a change of theme leaves shell.toml as it was" "$(cat "$WORK/shell.toml.on")" "$(cat "$H/.config/omarchy/shell.toml")"
is "and Vetro on" on "$(run vetro status | sed -n 1p)"
is "and the engine in" 1 "$(run hyprctl plugin list | grep -c '^Plugin hyprglass')"
run omarchy theme set launch
is "the launch theme is no different" "$(cat "$WORK/shell.toml.on")" "$(cat "$H/.config/omarchy/shell.toml")"
# A hook that finds the glass colours gone (a theme tool rewrote shell.toml) puts them back.
{
  grep -v '^' /dev/null
  sed '/# >>> vetro glass/,/# <<< vetro glass/d' "$H/.config/omarchy/shell.toml"
} >"$WORK/shell.toml.cut"
cat "$WORK/shell.toml.cut" >"$H/.config/omarchy/shell.toml"
lacks "the glass colours are gone" "$H/.config/omarchy/shell.toml" "# >>> vetro glass"
run omarchy theme set wallhaven-k8j6w1
is "the theme hook brings the glass colours back" "$(cat "$WORK/shell.toml.on")" "$(cat "$H/.config/omarchy/shell.toml")"

# --- the guard: stood down, the engine may be in but the glass colours are not laid
touch "$H/run/vetro-glass-stood-down"
echo "read 13 times in ten seconds" >"$H/run/vetro-glass-stood-down"
run vetro glass load
lacks "stood down: no glass colours" "$H/.config/omarchy/shell.toml" "# >>> vetro glass"
says "status says the guard stood down" "guard: stood down" run vetro status
says "doctor says the guard stood down" "stood down after being read in a loop" run vetro doctor
run vetro on >/dev/null 2>&1 || fail "vetro on failed after the guard stood down"
no "on clears the guard" test -e "$H/run/vetro-glass-stood-down"
has "and the glass colours are back" "$H/.config/omarchy/shell.toml" "# >>> vetro glass"

# --- an engine built for another Hyprland is not loaded
run vetro off >/dev/null 2>&1
printf 'ffffffffffffffffffffffffffffffffffffffff\nv9.9.9\n%s\n' "$PINNED_ENGINE" >"$H/.local/lib/vetro/hyprglass.built-for"
run vetro on >/dev/null 2>&1
if called "hyprctl plugin load"; then
  forget_calls
fi
no "an engine for another Hyprland is not loaded" test -e "$H/.standin/plugin"
lacks "and so no glass colours" "$H/.config/omarchy/shell.toml" "# >>> vetro glass"
says "status says the engine is for another Hyprland" "engine: built for Hyprland v9.9.9, not for the one running" run vetro status
printf '%s\nv0.56.2\n%s\n' "$PINNED_HYPRLAND" "$PINNED_ENGINE" >"$H/.local/lib/vetro/hyprglass.built-for"
run vetro glass load

# --- off
forget_calls
run vetro off >"$WORK/off.out" 2>&1 || fail "vetro off failed: $(tail -n 3 "$WORK/off.out")"
is "off reads as off" off "$(run vetro status | sed -n 1p)"
is "off does not touch the theme" wallhaven-k8j6w1 "$(theme)"
if called "omarchy theme set"; then fail "off set a theme"; fi
if called "gsettings"; then fail "off touched GTK's settings"; fi
no "off leaves no block in shell.toml" test -e "$H/.config/omarchy/shell.toml"
is "off puts kitty.conf back as it was" "$(cat "$WORK/kitty.before")" "$(cat "$H/.config/kitty/kitty.conf")"
is "off puts foot.ini back as it was" "$(cat "$WORK/foot.before")" "$(cat "$H/.config/foot/foot.ini")"
no "off unlinks GTK's colours" test -L "$H/.config/gtk-3.0/vetro.css"
no "off unlinks Qt's colours" test -L "$H/.config/qt6ct/colors/vetro.colors"
is "off puts the user's own gtk.css back" "my own gtk.css" "$(cat "$H/.config/gtk-3.0/gtk.css")"
no "off leaves no engine in Hyprland" test -e "$H/.standin/plugin"
called "hyprctl plugin unload $H/.local/lib/vetro/hyprglass.so" || fail "off did not unload the engine"
is "off gives Omarchy's menu back" omarchy.menu "$(jq -r '.bar.layout.left[0].id' "$H/.config/omarchy/shell.json")"
is "and its workspace numbers, where they stood" "omarchy.menu omarchy.workspaces" "$(jq -r '.bar.layout.left | map(.id) | join(" ")' "$H/.config/omarchy/shell.json")"
is "and its bar" null "$(jq -r '.bar.id' "$H/.config/omarchy/shell.json")"
is "off brings Omarchy's originals back" null "$(jq -r '.disabledPlugins' "$H/.config/omarchy/shell.json")"
is "off leaves no Vetro plugin enabled" 0 "$(run omarchy-plugin-list --json | jq '[.[] | select(.id | startswith("vetro.")) | select(.enabled)] | length')"
called "hyprctl setcursor default 24" || fail "off did not put the cursor back"
run vetro off >/dev/null 2>&1 || fail "off when already off failed"
run vetro glass load
no "glass load while off loads nothing" test -e "$H/.standin/plugin"
no "and lays nothing" test -e "$H/.config/omarchy/shell.toml"

# --- toggle, and the launch theme as the current one makes no difference
echo launch >"$H/.local/state/omarchy/current/theme.name"
run vetro toggle >/dev/null 2>&1
is "toggle from off is on" on "$(run vetro status | sed -n 1p)"
has "under the launch theme the same block is laid" "$H/.config/omarchy/shell.toml" "# >>> vetro look"
is "and the launch theme is still the theme" launch "$(theme)"
run vetro toggle >/dev/null 2>&1
is "toggle from on is off" off "$(run vetro status | sed -n 1p)"
is "and the launch theme is still the theme" launch "$(theme)"

# --- an engine that cannot be unloaded from this path (it was loaded from another)
run vetro on >/dev/null 2>&1
echo "/somewhere/else/hyprglass.so" >"$H/.standin/plugin"
run vetro off >"$WORK/off2.out" 2>&1 || fail "off failed when the engine could not be unloaded"
has "it says where the engine stays" "$WORK/off2.out" "goes at the next login"
is "off still finished" off "$(run vetro status | sed -n 1p)"
rm -f "$H/.standin/plugin"

# --- a bar its owner took the workspace numbers off gets none of Vetro's
numbers() { jq -r '[.bar.layout[][] | .id | select(test("workspaces"))] | join(" ")' "$H/.config/omarchy/shell.json"; }
cp "$H/.config/omarchy/shell.json" "$WORK/shell.json.numbers"
jq '.bar.layout.left |= map(select(.id != "omarchy.workspaces"))' "$WORK/shell.json.numbers" >"$H/.config/omarchy/shell.json"
run vetro on >/dev/null 2>&1
is "on puts no workspace numbers on a bar that had none" "" "$(numbers)"
run vetro doctor >"$WORK/doctor0.out" 2>&1 || fail "doctor failed on a bar without workspace numbers: $(grep -E '^FAIL' "$WORK/doctor0.out" | head -n 3)"
run vetro off >/dev/null 2>&1
is "and off puts none there either" "" "$(numbers)"
cp "$WORK/shell.json.numbers" "$H/.config/omarchy/shell.json"

# --- a bar of somebody else's that its owner chose is left where it is
mkdir -p "$H/.config/omarchy/plugins/local.neon-bar"
echo '{"schemaVersion":1,"id":"local.neon-bar","name":"Neon","version":"1","kinds":["bar"],"entryPoints":{"bar":"Bar.qml"}}' >"$H/.config/omarchy/plugins/local.neon-bar/manifest.json"
run omarchy bar use local.neon-bar || fail "the stand-in would not use another bar"
run vetro on >/dev/null 2>&1
is "on leaves a bar of your own choosing in use" local.neon-bar "$(jq -r '.bar.id' "$H/.config/omarchy/shell.json")"
run vetro doctor >"$WORK/doctor-neon.out" 2>&1 || fail "doctor failed with a bar of the owner's own: $(grep -E '^FAIL' "$WORK/doctor-neon.out" | head -n 3)"
has "and doctor says whose it is" "$WORK/doctor-neon.out" "a bar of your own choosing is in use (local.neon-bar)"
run vetro off >/dev/null 2>&1
is "and so does off" local.neon-bar "$(jq -r '.bar.id' "$H/.config/omarchy/shell.json")"
run omarchy bar use omarchy.bar
rm -r "$H/.config/omarchy/plugins/local.neon-bar"

# --- an Omarchy whose bar Vetro's builder does not know: Omarchy's own bar stays
mkdir -p "$WORK/newer-bar"
sed 's/if (dragging) return/if (dragging) { return }/' "$REPO/tests/fixtures/bar/Bar.qml" >"$WORK/newer-bar/Bar.qml"
cp "$REPO/tests/fixtures/bar/BarModel.js" "$WORK/newer-bar/"
BAR_SOURCE=$WORK/newer-bar
run vetro on >"$WORK/on-newer.out" 2>&1 || fail "vetro on failed over a bar it does not know"
is "over an Omarchy bar it does not know, Omarchy's own stays in use" null "$(jq -r '.bar.id' "$H/.config/omarchy/shell.json")"
has "on says why" "$WORK/on-newer.out" "Vetro's bar is not built"
no "the bar built for the earlier Omarchy is gone" test -e "$H/.local/state/vetro/bar"
no "and its link" test -L "$H/.config/omarchy/plugins/vetro.bar"
run vetro doctor >"$WORK/doctor-newer.out" 2>&1 || fail "doctor failed where Vetro's bar could not be built: $(grep -E '^FAIL' "$WORK/doctor-newer.out" | head -n 3)"
has "doctor warns, and does not fail" "$WORK/doctor-newer.out" "warn  Vetro's bar is not built"
unset BAR_SOURCE
run vetro on >/dev/null 2>&1
is "with a bar it knows again, Vetro's is back in use" vetro.bar "$(jq -r '.bar.id' "$H/.config/omarchy/shell.json")"
run vetro off >/dev/null 2>&1

# --- doctor, build, and the words
run vetro on >/dev/null 2>&1
run vetro doctor >"$WORK/doctor.out" 2>&1 || fail "doctor failed on a good install: $(grep -E '^FAIL' "$WORK/doctor.out" | head -n 3)"
has "doctor ends well" "$WORK/doctor.out" "all well"
has "doctor looks at the workspace numbers" "$WORK/doctor.out" "vetro.workspaces is on the bar"
has "and at the bar" "$WORK/doctor.out" "Vetro's bar is the bar in use"
cp "$H/.config/omarchy/shell.json" "$WORK/shell.json.bar"
jq 'del(.bar.id)' "$WORK/shell.json.bar" >"$H/.config/omarchy/shell.json"
run vetro doctor >"$WORK/doctor-bar.out" 2>&1 && fail "doctor passed with Omarchy's own bar in use though Vetro's is built"
has "doctor says which bar is in use" "$WORK/doctor-bar.out" "Omarchy's own bar is in use"
cp "$WORK/shell.json.bar" "$H/.config/omarchy/shell.json"
cp "$H/.config/omarchy/shell.json" "$WORK/shell.json.on"
jq '(.bar.layout.left[] | select(.id == "vetro.workspaces") | .id) = "omarchy.workspaces"' "$WORK/shell.json.on" >"$H/.config/omarchy/shell.json"
run vetro doctor >"$WORK/doctor1.out" 2>&1 && fail "doctor passed with Omarchy's workspace numbers in the place of Vetro's"
has "doctor says which widget stands in the wrong place" "$WORK/doctor1.out" "omarchy.workspaces is still on the bar"
run vetro on >/dev/null 2>&1
is "on puts Vetro's numbers back" "vetro.workspaces" "$(numbers)"
rm -f "$H/.config/omarchy/plugins/vetro.osd"
run vetro doctor >"$WORK/doctor2.out" 2>&1 && fail "doctor passed with a plugin link gone"
has "doctor names the link that is gone" "$WORK/doctor2.out" "vetro.osd"
ln -s "$REPO/shell/vetro.osd" "$H/.config/omarchy/plugins/vetro.osd"
run vetro build --check >/dev/null 2>&1
is "build --check with the engine built for this Hyprland" 0 "$?"
rm -f "$H/.local/lib/vetro/hyprglass.built-for"
run vetro build --check >/dev/null 2>&1
is "build --check with no engine built" 1 "$?"
# --- which glass the panes and windows are
materials() { find "$H/.config/vetro" -maxdepth 1 -name '*-material' -printf '%f\n' | sort | tr '\n' ' '; }
is "the glass is Apple's own as it comes" apple "$(run vetro glass look)"
is "and that is no file" "" "$(materials)"
forget_calls
is "another glass is chosen by its name" cloud "$(run vetro glass look cloud)"
is "which leaves its file" "cloud-material " "$(materials)"
called "hyprctl reload" || fail "choosing a glass did not have Hyprland read the look again"
is "and is the one named afterwards" cloud "$(run vetro glass look)"
run vetro glass look sharp >/dev/null
is "choosing another takes the earlier one's file away" "sharp-material " "$(materials)"
run vetro glass look clear >/dev/null
is "the clear one is one of them" clear "$(run vetro glass look)"
: >"$H/.config/vetro/sharp-material"
is "with two files there, it is the one the look reads first" clear "$(run vetro glass look)"
run vetro glass look milky >/dev/null 2>&1
is "a glass that is not one is refused" 1 "$?"
is "and changes nothing" "clear-material sharp-material " "$(materials)"
: >"$H/.config/vetro/first-material"
is "the regular one is chosen like the others" regular "$(run vetro glass look regular)"
is "which leaves its file, and the one an earlier version left goes" "regular-material " "$(materials)"
run vetro glass look cloud >/dev/null
: >"$H/.config/vetro/regular-material"
is "the cloud one comes before the regular one" cloud "$(run vetro glass look)"
is "macOS 27's recipe is chosen like the others" macos "$(run vetro glass look macos)"
is "and leaves its file" "macos-material " "$(materials)"
is "the liquid one is chosen like the others" liquid "$(run vetro glass look liquid)"
is "and leaves its file" "liquid-material " "$(materials)"
is "Apple's own is chosen by its name" apple "$(run vetro glass look apple)"
is "and leaves no file" "" "$(materials)"
says "--help says how the glass is chosen" "vetro glass look" run vetro --help

run vetro nonsense >/dev/null 2>&1
is "an unknown word is refused" 2 "$?"
says "--help says how to switch it on" "vetro on" run vetro --help
rm -f "$V"
is "status works without the link" on "$(run "$REPO/bin/vetro" status | sed -n 1p)"
rm -f "$H/.config/hypr/vetro.lua"
run "$REPO/bin/vetro" on >/dev/null 2>&1
is "on without an installation is refused" 1 "$?"

finish
