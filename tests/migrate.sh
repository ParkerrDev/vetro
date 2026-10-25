#!/bin/bash
# The move from liftoff and meniscus to Vetro, on a home folder set up the way
# the owner's laptop was (tests/owner-home.sh): install.sh takes their links,
# blocks, service and state over to Vetro's names, builds nothing the old engine
# already is, leaves the look on, leaves no dangling link and no stale block,
# changes nothing the second time, and migrate.sh --undo puts it all back.
# shellcheck source-path=SCRIPTDIR
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"
# shellcheck source=owner-home.sh
. "$(dirname "$(readlink -f "$0")")/owner-home.sh"
make_bin

# old_names: every file in the home folder's own places that still has the
# name of an earlier project, in its name or its text. (The record of the move
# has them all: it is what was there before.)
old_names() {
  {
    (cd "$H" && find .config .local .cache \( -iname '*liftoff*' -o -iname '*meniscus*' \) 2>/dev/null)
    (cd "$H" && grep -rIl -e liftoff -e meniscus .config .local 2>/dev/null)
  } | grep -v '^\.local/state/vetro-migrated' | sort -u
}

# --- the home folder as the owner's was
owner_home owner
snapshot >"$WORK/owner.before"
run "$REPO/migrate.sh" --check >"$WORK/check.out" 2>&1
is "--check finds the earlier projects" 1 "$?"
has "--check names a link" "$WORK/check.out" "left: the link .config/omarchy/plugins/liftoff.menu"
has "--check names a block" "$WORK/check.out" "left: an old block in .config/omarchy/shell.toml"

# --- a dry run changes nothing and says what it would do
run "$REPO/migrate.sh" --dry-run >"$WORK/dry.out" 2>&1 || fail "migrate.sh --dry-run failed"
snapshot >"$WORK/owner.dry"
is "a dry run changes nothing" "$(cat "$WORK/owner.before")" "$(cat "$WORK/owner.dry")"
has "it says it would remove a link" "$WORK/dry.out" "remove the link .config/omarchy/plugins/liftoff.menu"
has "it says it would rename the plugin ids" "$WORK/dry.out" "omarchy/shell.json: the plugin ids liftoff.* become vetro.*"
has "it says it would move the engine" "$WORK/dry.out" "move .local/lib/meniscus/hyprglass.so to .local/lib/vetro/hyprglass.so"
no "and makes no record" test -e "$H/.local/state/vetro-migrated"
if called "systemctl --user stop"; then fail "a dry run stopped the service"; fi

# --- the move is refused until Vetro's own plugins are linked
run "$REPO/migrate.sh" >"$WORK/early.out" 2>&1
is "migrate.sh alone, before install.sh, is refused" 1 "$?"
has "and says what to run" "$WORK/early.out" "install.sh"
snapshot >"$WORK/owner.early"
is "and changes nothing" "$(cat "$WORK/owner.before")" "$(cat "$WORK/owner.early")"

# --- the move
forget_calls
run "$REPO/install.sh" >"$WORK/install.out" 2>&1 || fail "install.sh over the earlier projects failed: $(tail -n 5 "$WORK/install.out")"
run "$REPO/migrate.sh" --check >"$WORK/check2.out" 2>&1
is "--check finds nothing of them afterwards" 0 "$?"
leftover=$(old_names | grep -v -e '^\.local/state/liftoff\(/\|$\)' -e '^\.local/share/liftoff\(/\|$\)' -e '^\.cache/liftoff\(/\|$\)')
is "no file of the home folder has an old name in its name or text" "" "$leftover"
is "no link is dangling" "" "$(find "$H/.config" "$H/.local" -xtype l 2>/dev/null)"

# the theme and the wallpaper are the owner's
is "the theme is still the owner's" wallhaven-k8j6w1 "$(theme)"
if called "omarchy theme set"; then fail "the move set a theme"; fi
no "the earlier theme link is gone" test -e "$H/.config/omarchy/themes/liftoff"
no "and the two meniscus themes" test -e "$H/.config/omarchy/themes/meniscus"

# a theme name the old projects had is the new one's
is "Aether's remembered theme is called launch now" launch "$(cat "$H/.config/aether/previous-omarchy-theme")"

# the links
for old in liftoff liftoff.barglass liftoff.cards liftoff.center liftoff.center-button liftoff.lock liftoff.menu \
  liftoff.notifications liftoff.osd liftoff.windows-button; do
  no "$old is no longer a plugin" test -e "$H/.config/omarchy/plugins/$old"
done
ok "passthru's button is untouched" test -L "$H/.config/omarchy/plugins/passthru.windows"
for new in vetro.notifications vetro.osd vetro.lock vetro.barglass vetro.center vetro.center-button vetro.menu vetro.workspaces vetro.cards vetro.lib; do
  ok "$new is linked" test -e "$H/.config/omarchy/plugins/$new"
done
for link in .local/bin/liftoff .local/bin/liftoff-launchd .local/bin/liftoff-look .local/bin/meniscus \
  .config/hypr/liftoff-look.lua .config/hypr/meniscus.lua .config/liftoff .local/share/fonts/liftoff \
  .config/omarchy/hooks/theme-set.d/50-liftoff.sh .config/omarchy/hooks/post-update.d/50-meniscus-glass.sh \
  .config/systemd/user/liftoff-launchd.service .config/gtk-3.0/matugen.css .config/qt6ct/colors/matugen-kde.colors \
  .config/Kvantum/liftoff; do
  no "$link is gone" test -e "$H/$link" -o -L "$H/$link"
done
is "GTK's colours are Vetro's" "$REPO/override/gtk-3.0/vetro.css" "$(readlink "$H/.config/gtk-3.0/vetro.css")"
is "Qt's colours are Vetro's" "$REPO/override/qt6ct/vetro.colors" "$(readlink "$H/.config/qt6ct/colors/vetro.colors")"
is "Kvantum's theme is Vetro's" "$REPO/override/kvantum/vetro" "$(readlink "$H/.config/Kvantum/vetro")"
is "the command is Vetro's" "$REPO/bin/vetro" "$(readlink "$H/.local/bin/vetro")"
is "the Windows VM's files are left where they were" "vm" "$(cat "$H/.local/state/liftoff/windows/mode")"
ok "and so are the VM tools" test -e "$H/.local/share/liftoff/windows-tools/tools.zip"

# the blocks
for file in .config/omarchy/shell.toml .config/kitty/kitty.conf .config/foot/foot.ini; do
  has "$file has Vetro's block" "$H/$file" "# >>> vetro look"
  lacks "$file has no old block" "$H/$file" "liftoff"
done
has "shell.toml has the glass colours" "$H/.config/omarchy/shell.toml" "# >>> vetro glass"
is "the user's lines in kitty.conf are as they were" "# Remove all inherited shortcuts, including Kitty's built-in shortcuts" "$(sed -n 1p "$H/.config/kitty/kitty.conf")"
is "hyprland.lua has Vetro's block once" 1 "$(grep -c '^-- >>> vetro' "$H/.config/hypr/hyprland.lua")"
lacks "and not the old line" "$H/.config/hypr/hyprland.lua" "liftoff-look"
ok "the block stands where the old line stood, before passthru's" \
  test "$(grep -n '^-- >>> vetro' "$H/.config/hypr/hyprland.lua" | cut -d: -f1)" -lt "$(grep -n 'require("hypr.passthru")' "$H/.config/hypr/hyprland.lua" | cut -d: -f1)"
is "the lines after it are as they were" 'require("hypr.passthru")' "$(grep 'require("hypr.passthru")' "$H/.config/hypr/hyprland.lua")"
has "the key for the old command runs Vetro" "$H/.config/hypr/bindings.lua" 'o.bind("SUPER + CTRL + ALT + L", "Toggle Vetro", "vetro toggle")'
lacks "and the old command is gone" "$H/.config/hypr/bindings.lua" "liftoff"
has "the other keys are as they were" "$H/.config/hypr/bindings.lua" 'o.bind("SUPER + SHIFT + T", "2FA codes"'

# the shell's plugins: renamed where they stood
is "the menu is still first on the left" vetro.menu "$(jq -r '.bar.layout.left[0].id' "$H/.config/omarchy/shell.json")"
is "passthru's button is still beside it" passthru.windows "$(jq -r '.bar.layout.left[1].id' "$H/.config/omarchy/shell.json")"
is "the bell is still in the middle of the right side" vetro.center-button "$(jq -r '.bar.layout.right[3].id' "$H/.config/omarchy/shell.json")"
is "and nothing else on the bar moved" "omarchy.tray weather pomodoro vetro.center-button omarchy.tailscale omarchy.power" \
  "$(jq -r '.bar.layout.right | map(.id) | join(" ")' "$H/.config/omarchy/shell.json")"
is "the plugins are Vetro's" "twofa xeet vetro.notifications vetro.osd vetro.lock vetro.cards vetro.barglass vetro.center" \
  "$(jq -r '.plugins | map(.id) | join(" ")' "$H/.config/omarchy/shell.json")"
is "Omarchy's originals are still switched off" "omarchy.notifications omarchy.osd omarchy.lock omarchy.menu" \
  "$(jq -r '.disabledPlugins | join(" ")' "$H/.config/omarchy/shell.json")"
is "and will come back when their clones go" "vetro.notifications vetro.osd vetro.lock vetro.menu" \
  "$(jq -r '.cloneSourceRestores | join(" ")' "$H/.config/omarchy/shell.json")"
is "the rest of the shell's file is as it was" '{"lock":300,"screensaver":150}' "$(jq -c .idle "$H/.config/omarchy/shell.json")"
is "the file is still private" 600 "$(stat -c %a "$H/.config/omarchy/shell.json")"

# state, data and the engine
for f in state.json misc.json account.json notifications.json; do
  ok "state: $f moved" test -s "$H/.local/state/vetro/$f"
  no "state: $f is not left behind" test -e "$H/.local/state/liftoff/$f"
done
no "the remembered theme is dropped" test -e "$H/.local/state/liftoff/look-previous-theme"
no "and meniscus's" test -e "$H/.local/state/meniscus"
ok "the launch cache moved" test -s "$H/.cache/vetro/launches.json"
ok "the clips moved" test -s "$H/.local/share/vetro/clips/launch.mp4"
is "and current.json names the clip where it is now" "$H/.local/share/vetro/clips/launch.mp4" "$(jq -r .file "$H/.local/share/vetro/clips/current.json")"
is "and so does the clip's own note" "$H/.local/share/vetro/clips/launch.jpg" "$(jq -r .poster "$H/.local/share/vetro/clips/launch.json")"
ok "so that the file it names is there" test -s "$(jq -r .file "$H/.local/share/vetro/clips/current.json")"
no "the old build folder is gone" test -e "$H/.cache/meniscus"
is "the engine moved" "fake engine, built long ago" "$(cat "$H/.local/lib/vetro/hyprglass.so")"
ok "with its stamp" test -s "$H/.local/lib/vetro/hyprglass.built-for"
is "and the backup beside it" "an older build" "$(cat "$H/.local/lib/vetro/hyprglass.so.stock-radius")"
no "the engine was not built again" grep -q 'make in' "$H/.standin/log"
no "the old guard marks are gone" test -e "$H/run/liftoff-glass-stood-down"
no "and the old daemon's sockets" test -e "$H/run/liftoff"

# the service
if called "systemctl --user stop liftoff-launchd.service"; then :; else fail "the old service was not stopped"; fi
ok "the new service is running" test -e "$H/.standin/active-vetro-launchd.service"
no "the old one is not" test -e "$H/.standin/active-liftoff-launchd.service"
is "the new unit is linked" "$REPO/addons/launch/systemd/vetro-launchd.service" "$(readlink "$H/.config/systemd/user/vetro-launchd.service")"

# Vetro is on, as the look was
is "Vetro reads as on" on "$(run vetro status | sed -n 1p)"
says "and the engine is loaded" "engine: loaded" run vetro status
says "and the guard is ready" "guard: ready" run vetro status
called "hyprctl plugin load $H/.local/lib/vetro/hyprglass.so" || fail "the engine was not loaded from its new place"
run vetro doctor >"$WORK/doctor.out" 2>&1 || fail "doctor finds fault after the move: $(grep -E '^(FAIL|warn)' "$WORK/doctor.out" | head -n 4 | tr '\n' ' ')"

# --- a second run changes nothing
snapshot >"$WORK/owner.after"
forget_calls
run "$REPO/install.sh" >"$WORK/install2.out" 2>&1 || fail "install.sh a second time failed"
snapshot >"$WORK/owner.after2"
is "a second run changes nothing" "$(cat "$WORK/owner.after")" "$(cat "$WORK/owner.after2")"
has "and migrate.sh says there is nothing to take over" "$WORK/install2.out" "nothing to take over"
if called "hyprctl reload"; then fail "a second run reloaded Hyprland"; fi

# --- taking Vetro out afterwards leaves the owner's own things alone
run "$REPO/uninstall.sh" >/dev/null 2>&1 || fail "uninstall.sh after the move failed"
no "no block of Vetro's is left in shell.toml" test -e "$H/.config/omarchy/shell.toml"
lacks "or in kitty.conf" "$H/.config/kitty/kitty.conf" "vetro"
lacks "or in hyprland.lua" "$H/.config/hypr/hyprland.lua" "vetro"
has "the owner's key still runs the command Vetro had" "$H/.config/hypr/bindings.lua" "vetro toggle"
is "the theme is the owner's" wallhaven-k8j6w1 "$(theme)"

# --- the owner on the launch theme itself: the same theme, under its new name
owner_home onlaunch
echo liftoff >"$H/.local/state/omarchy/current/theme.name"
forget_calls
run "$REPO/install.sh" >/dev/null 2>&1 || fail "install.sh with the old theme current failed"
is "the theme that was current is current under its new name" launch "$(theme)"
if called "omarchy theme set"; then fail "the move set a theme"; fi
no "and the theme link is the new one's" test -e "$H/.config/omarchy/themes/liftoff"
ok "the launch theme is there to choose" test -d "$H/.config/omarchy/themes/launch"

# --- both earlier projects' lines in hyprland.lua become one block of Vetro's
owner_home both
printf '\n-- >>> meniscus (install.sh adds this block, uninstall.sh removes it)\nrequire("default.hypr.require_optional").module("hypr.meniscus")\n-- <<< meniscus\n' >>"$H/.config/hypr/hyprland.lua"
run "$REPO/install.sh" >/dev/null 2>&1 || fail "install.sh with both projects' lines in hyprland.lua failed"
is "one block of Vetro's" 1 "$(grep -c '^-- >>> vetro' "$H/.config/hypr/hyprland.lua")"
no "none of meniscus's" grep -q 'meniscus' "$H/.config/hypr/hyprland.lua"
no "and no line of the other's" grep -q 'liftoff' "$H/.config/hypr/hyprland.lua"
is "the file still loads Vetro's Lua once" 1 "$(grep -c 'hypr.vetro' "$H/.config/hypr/hyprland.lua")"
run "$REPO/install.sh" >/dev/null 2>&1
is "and a second run adds no other" 1 "$(grep -c '^-- >>> vetro' "$H/.config/hypr/hyprland.lua")"

# --- an old look that was switched off stays off
owner_home wasoff
touch "$H/.local/state/liftoff/look-off"
sed -i '/>>> liftoff/,/<<< liftoff/d' "$H/.config/omarchy/shell.toml" "$H/.config/kitty/kitty.conf" "$H/.config/foot/foot.ini"
rm -f "$H/.config/omarchy/shell.toml" "$H/.config/gtk-3.0/"{gtk.css,matugen.css,settings.ini} "$H/.config/gtk-4.0/"{gtk.css,matugen.css,settings.ini}
jq '.plugins |= map(select(.id | startswith("liftoff") | not)) | del(.disabledPlugins, .cloneSourceRestores)
    | .bar.layout.left |= map(if .id == "liftoff.menu" then {id: "omarchy.menu"} else . end)
    | .bar.layout.right |= map(select(.id != "liftoff.center-button"))' "$H/.config/omarchy/shell.json" >"$WORK/off.json" && cp "$WORK/off.json" "$H/.config/omarchy/shell.json"
rm -f "$H/.standin/active-liftoff-launchd.service"
forget_calls
run "$REPO/install.sh" >"$WORK/wasoff.out" 2>&1 || fail "install.sh over an old look that was off failed: $(tail -n 3 "$WORK/wasoff.out")"
is "it is off" off "$(run vetro status | sed -n 1p)"
no "no resume was left" test -e "$H/.local/state/vetro/resume-on"
ok "the flag was carried across" test -e "$H/.local/state/vetro/off"
no "nothing is laid in shell.toml" test -e "$H/.config/omarchy/shell.toml"
lacks "nor in kitty.conf" "$H/.config/kitty/kitty.conf" "vetro"
is "no Vetro plugin is enabled" 0 "$(run omarchy-plugin-list --json | jq '[.[] | select(.id | startswith("vetro.")) | select(.enabled)] | length')"
no "the engine was not loaded" test -e "$H/.standin/plugin"
if called "hyprctl reload"; then fail "installing over an old look that was off reloaded Hyprland"; fi
no "the service was not started" test -e "$H/.standin/active-vetro-launchd.service"
no "and no old link is left" test -L "$H/.config/omarchy/plugins/liftoff.menu"
run "$REPO/migrate.sh" --check >/dev/null 2>&1
is "nothing of the earlier projects is left" 0 "$?"
has "the key for the old command runs Vetro all the same" "$H/.config/hypr/bindings.lua" "vetro toggle"
run vetro on >/dev/null 2>&1 || fail "vetro on after the move failed"
is "and it can be switched on" on "$(run vetro status | sed -n 1p)"

# --- the way back: put the record back, and it is as it was
owner_home undo
snapshot >"$WORK/undo.before"
cp "$H/.config/omarchy/shell.json" "$WORK/undo.shell.json"
run "$REPO/install.sh" >/dev/null 2>&1 || fail "install.sh (for the undo) failed"
ok "a record of the move is kept" test -d "$H/.local/state/vetro-migrated"
run "$REPO/migrate.sh" --undo >"$WORK/undo.out" 2>&1 || fail "migrate.sh --undo failed: $(tail -n 4 "$WORK/undo.out")"
snapshot >"$WORK/undo.after"
# What an undo cannot put back: the record itself, and the engine's old build
# folder (it is rebuilt) and the shell's own file's formatting (the shell
# rewrites it; its contents are compared).
filter() { grep -v -e 'vetro-migrated' -e '\.cache/meniscus' -e 'omarchy/shell.json' "$1"; }
diff <(filter "$WORK/undo.before") <(filter "$WORK/undo.after") >"$WORK/undo.diff" 2>&1 || fail "the undo left the home folder different: $(head -n 8 "$WORK/undo.diff" | tr '\n' ' ')"
is "the shell's file is as it was" "$(jq -S . "$WORK/undo.shell.json")" "$(jq -S . "$H/.config/omarchy/shell.json")"
# (but for the stand-in daemon every test home starts with)
is "Vetro is out" "" "$(find "$H/.config" "$H/.local" -name '*vetro*' -not -path '*/vetro-migrated*' -not -path "$H/.local/bin/vetro-notifyd" 2>/dev/null)"
ok "the old look's command is back" test -L "$H/.local/bin/liftoff-look"
ok "the record is marked as used" test -n "$(find "$H/.local/state/vetro-migrated" -maxdepth 1 -name '*.undone')"
run "$REPO/migrate.sh" --undo >/dev/null 2>&1
is "a second undo has nothing to undo" 1 "$?"

finish
