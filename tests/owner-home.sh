# shellcheck shell=bash
# A scratch home folder set up the way the owner's laptop was, with liftoff and
# meniscus installed and switched on, so that the move to Vetro is tried on the
# real shape of things: their links (into repositories that sit where the real
# ones do, under Dev/GitHub), the blocks they added to other programs' files,
# their plugin ids in the shell's own file, their service and its state, the
# built engine, the key bound to the old command.
#
# This file is the one place, with migrate.sh, that names the two earlier
# projects. Sourced after tests/lib.sh; owner_home NAME builds it and sets H.

# plugin DIR ID KINDS [CLONED-FROM]: a plugin folder with a manifest.
fake_plugin() {
  local dir=$1 id=$2 kinds=$3 cloned=${4:-}
  mkdir -p "$dir"
  jq -n --arg id "$id" --argjson kinds "$kinds" --arg cloned "$cloned" '
    {schemaVersion: 1, id: $id, name: $id, version: "1.0.0", kinds: $kinds, entryPoints: {panel: "X.qml"}}
    + (if $cloned == "" then {} else {omarchy: {clonedFrom: $cloned}} end)' >"$dir/manifest.json"
  echo "// $id" >"$dir/X.qml"
}

owner_home() {
  local lo mn plugin
  new_home "$1"
  lo=$H/Dev/GitHub/liftoff
  mn=$H/Dev/GitHub/meniscus
  # --- the two repositories, with what the links lead to
  mkdir -p "$lo/home/.config/gtk-3.0" "$lo/home/.config/gtk-4.0" "$lo/omarchy/gtk/gtk-3.0" "$lo/omarchy/gtk/gtk-4.0" \
    "$lo/omarchy/hypr" "$lo/home/.config/Kvantum/liftoff" "$lo/home/.config/liftoff" "$lo/omarchy/hooks/theme-set.d" \
    "$lo/omarchy/theme/liftoff" "$lo/omarchy/qt6ct" "$lo/home/.config/qt6ct" "$lo/omarchy/systemd/liftoff-launchd.service.d" \
    "$lo/home/.config/systemd/user" "$lo/bin" "$lo/omarchy/fonts" "$mn/glass" "$mn/theme/meniscus" "$mn/theme/meniscus-light"
  for v in 3.0 4.0; do
    echo "@import 'matugen.css';" >"$lo/home/.config/gtk-$v/gtk.css"
    echo "[Settings]" >"$lo/home/.config/gtk-$v/settings.ini"
    echo "/* black and white */" >"$lo/omarchy/gtk/gtk-$v/matugen.css"
  done
  echo "-- the look" >"$lo/omarchy/hypr/liftoff-look.lua"
  echo "[General]" >"$lo/home/.config/Kvantum/kvantum.kvconfig"
  echo "theme" >"$lo/home/.config/Kvantum/liftoff/liftoff.kvconfig"
  echo '{}' >"$lo/home/.config/liftoff/shell.json"
  echo "# wallpaper" >"$lo/home/.config/liftoff/wallpaper.toml"
  echo "#!/bin/bash" >"$lo/omarchy/hooks/theme-set.d/50-liftoff.sh"
  echo "colors" >"$lo/omarchy/theme/liftoff/colors.toml"
  echo "colors" >"$lo/omarchy/qt6ct/matugen-kde.colors"
  echo "[Appearance]" >"$lo/home/.config/qt6ct/qt6ct.conf"
  echo "[Service]" >"$lo/omarchy/systemd/liftoff-launchd.service.d/intel-decode.conf"
  echo "[Service]" >"$lo/home/.config/systemd/user/liftoff-launchd.service"
  for f in liftoff liftoff-launchd; do echo "#!/bin/bash" >"$lo/bin/$f"; done
  echo "#!/bin/bash" >"$lo/omarchy/liftoff-look"
  echo "font" >"$lo/omarchy/fonts/D-DIN.otf"
  echo "-- glass" >"$mn/glass/meniscus.lua"
  echo "#!/bin/bash" >"$mn/glass/post-update.sh"
  echo "#!/bin/bash" >"$mn/meniscus"
  echo "colors" >"$mn/theme/meniscus/colors.toml"
  echo "colors" >"$mn/theme/meniscus-light/colors.toml"
  fake_plugin "$lo/omarchy/plugin/liftoff" liftoff '["bar","panel"]'
  fake_plugin "$lo/omarchy/plugin/liftoff.barglass" liftoff.barglass '["panel"]'
  fake_plugin "$lo/omarchy/plugin/liftoff.cards" liftoff.cards '["panel"]'
  fake_plugin "$lo/omarchy/plugin/liftoff.center" liftoff.center '["panel"]'
  fake_plugin "$lo/omarchy/plugin/liftoff.center-button" liftoff.center-button '["bar-widget"]'
  fake_plugin "$lo/omarchy/plugin/liftoff.lock" liftoff.lock '["service"]' omarchy.lock
  fake_plugin "$lo/omarchy/plugin/liftoff.menu" liftoff.menu '["menu","bar-widget"]' omarchy.menu
  fake_plugin "$lo/omarchy/plugin/liftoff.notifications" liftoff.notifications '["service"]' omarchy.notifications
  fake_plugin "$lo/omarchy/plugin/liftoff.osd" liftoff.osd '["panel"]' omarchy.osd
  fake_plugin "$lo/omarchy/plugin/liftoff.windows-button" liftoff.windows-button '["bar-widget"]'
  fake_plugin "$H/Dev/GitHub/passthru/plugin/passthru.windows" passthru.windows '["bar-widget"]'

  # --- the links, as `find ~/.config ~/.local -lname '*liftoff*'` found them
  for v in 3.0 4.0; do
    mkdir -p "$H/.config/gtk-$v"
    ln -s "$lo/home/.config/gtk-$v/gtk.css" "$H/.config/gtk-$v/gtk.css"
    ln -s "$lo/omarchy/gtk/gtk-$v/matugen.css" "$H/.config/gtk-$v/matugen.css"
    ln -s "$lo/home/.config/gtk-$v/settings.ini" "$H/.config/gtk-$v/settings.ini"
  done
  ln -s "$lo/omarchy/hypr/liftoff-look.lua" "$H/.config/hypr/liftoff-look.lua"
  ln -s "$mn/glass/meniscus.lua" "$H/.config/hypr/meniscus.lua"
  mkdir -p "$H/.config/Kvantum" "$H/.config/qt6ct/colors" "$H/.config/systemd/user/liftoff-launchd.service.d" "$H/.local/share/fonts"
  ln -s "$lo/home/.config/Kvantum/kvantum.kvconfig" "$H/.config/Kvantum/kvantum.kvconfig"
  ln -s "$lo/home/.config/Kvantum/liftoff" "$H/.config/Kvantum/liftoff"
  ln -s "$lo/home/.config/liftoff" "$H/.config/liftoff"
  ln -s "$mn/glass/post-update.sh" "$H/.config/omarchy/hooks/post-update.d/50-meniscus-glass.sh"
  ln -s "$lo/omarchy/hooks/theme-set.d/50-liftoff.sh" "$H/.config/omarchy/hooks/theme-set.d/50-liftoff.sh"
  for plugin in liftoff liftoff.barglass liftoff.cards liftoff.center liftoff.center-button liftoff.lock liftoff.menu \
    liftoff.notifications liftoff.osd liftoff.windows-button; do
    ln -s "$lo/omarchy/plugin/$plugin" "$H/.config/omarchy/plugins/$plugin"
  done
  ln -s "$H/Dev/GitHub/passthru/plugin/passthru.windows" "$H/.config/omarchy/plugins/passthru.windows"
  mkdir -p "$H/.config/omarchy/themes"
  ln -s "$lo/omarchy/theme/liftoff" "$H/.config/omarchy/themes/liftoff"
  ln -s "$mn/theme/meniscus" "$H/.config/omarchy/themes/meniscus"
  ln -s "$mn/theme/meniscus-light" "$H/.config/omarchy/themes/meniscus-light"
  ln -s "$lo/omarchy/qt6ct/matugen-kde.colors" "$H/.config/qt6ct/colors/matugen-kde.colors"
  ln -s "$lo/home/.config/qt6ct/qt6ct.conf" "$H/.config/qt6ct/qt6ct.conf"
  ln -s "$lo/omarchy/systemd/liftoff-launchd.service.d/intel-decode.conf" "$H/.config/systemd/user/liftoff-launchd.service.d/intel-decode.conf"
  ln -s "$lo/home/.config/systemd/user/liftoff-launchd.service" "$H/.config/systemd/user/liftoff-launchd.service"
  ln -s "$lo/bin/liftoff" "$H/.local/bin/liftoff"
  ln -s "$lo/bin/liftoff-launchd" "$H/.local/bin/liftoff-launchd"
  ln -s "$lo/omarchy/liftoff-look" "$H/.local/bin/liftoff-look"
  ln -s "$mn/meniscus" "$H/.local/bin/meniscus"
  ln -s "$lo/omarchy/fonts" "$H/.local/share/fonts/liftoff"

  # --- the files they added blocks to
  cat >"$H/.config/hypr/hyprland.lua" <<'EOF'
-- Learn how to configure Hyprland: https://wiki.hypr.land/Configuring/Start/
require("default.hypr.omarchy")
require("hypr.monitors")
require("hypr.bindings")

-- Add any other personal Hyprland configuration below.

-- liftoff: its look (blur, cursor, Qt theme, launch wallpaper) on top of Omarchy, and no keybinds (~/.config/hypr/liftoff-look.lua)
require("hypr.liftoff-look")


-- The NVIDIA card to the Windows VM and back (~/.config/hypr/passthru.lua)
require("hypr.passthru")

-- Shake the mouse to find the pointer (dotfiles: config/hypr/pointer-shake.lua)
if package.searchpath("hypr.pointer-shake", package.path) then require("hypr.pointer-shake") end
EOF
  cat >"$H/.config/hypr/bindings.lua" <<'EOF'
-- Add a new binding.
-- o.bind("SUPER + SHIFT + R", "SSH", "alacritty -e ssh your-server")

-- liftoff: its look off (Omarchy exactly as it ships) and on again
o.bind("SUPER + CTRL + ALT + L", "Toggle liftoff look", "liftoff-look toggle")


-- twofa: the 2FA panel
o.bind("SUPER + SHIFT + T", "2FA codes", "omarchy-shell shell toggle twofa")
EOF
  mkdir -p "$H/.config/kitty" "$H/.config/foot"
  cat >"$H/.config/kitty/kitty.conf" <<'EOF'
# Remove all inherited shortcuts, including Kitty's built-in shortcuts
# clear_all_shortcuts yes
# >>> liftoff look (omarchy/install-look.sh; liftoff-look off removes this)
background_opacity 0.8
cursor_trail 0
# <<< liftoff look
EOF
  cat >"$H/.config/foot/foot.ini" <<'EOF'
[text-bindings]
\x1b[13;2u=Shift+Return
# >>> liftoff look (omarchy/install-look.sh; liftoff-look off removes this)
[colors-dark]
alpha=0.8
blur=yes
# <<< liftoff look
EOF
  cat >"$H/.config/omarchy/shell.toml" <<'EOF'
# >>> liftoff look (omarchy/install-look.sh; liftoff-look off removes this)
# Omarchy's bar sits in liftoff's rounded glass, which the liftoff.barglass
# plugin draws behind it, so the bar has no background of its own and leaves
# room for the gap around the glass.
[bar]
background-alpha = 0
size-horizontal  = 40
size-vertical    = 40
# <<< liftoff look
# >>> liftoff glass (liftoff-look glass load; glass unload removes this)
[bar]
text   = "#ffffff"
active = "#ffffff"
[popups]
background       = "#000000"
background-alpha = 0.2
# <<< liftoff glass
EOF
  # The shell's own file, with the plugins the old look enabled and the bell
  # where its owner put it, in the middle of the right side of the bar.
  cat >"$H/.config/omarchy/shell.json" <<'EOF'
{
  "bar": {
    "centerAnchor": "omarchy.clock",
    "layout": {
      "center": [{"id": "omarchy.indicators"}, {"format": "dddd HH:mm", "id": "omarchy.clock"}],
      "left": [{"id": "liftoff.menu"}, {"id": "passthru.windows"}, {"id": "omarchy.workspaces"}],
      "right": [
        {"id": "omarchy.tray"},
        {"id": "weather"},
        {"id": "pomodoro"},
        {"id": "liftoff.center-button"},
        {"id": "omarchy.tailscale"},
        {"id": "omarchy.power"}
      ]
    },
    "position": "top",
    "transparent": false
  },
  "cloneSourceRestores": ["liftoff.notifications", "liftoff.osd", "liftoff.lock", "liftoff.menu"],
  "disabledPlugins": ["omarchy.notifications", "omarchy.osd", "omarchy.lock", "omarchy.menu"],
  "idle": {"lock": 300, "screensaver": 150},
  "plugins": [
    {"id": "twofa"},
    {"id": "xeet"},
    {"id": "liftoff.notifications"},
    {"id": "liftoff.osd"},
    {"id": "liftoff.lock"},
    {"id": "liftoff.cards"},
    {"id": "liftoff.barglass"},
    {"id": "liftoff.center"}
  ],
  "version": 1
}
EOF
  chmod 600 "$H/.config/omarchy/shell.json"

  # --- their state, caches, built engine and runtime files
  mkdir -p "$H/.local/state/liftoff/windows" "$H/.local/state/meniscus" "$H/.cache/liftoff/looking-glass-build" \
    "$H/.cache/meniscus/hyprglass" "$H/.local/share/liftoff/clips" "$H/.local/share/liftoff/windows-tools" "$H/.local/lib/meniscus"
  echo '{"wallpaper_mode": "video", "static_wallpaper": ""}' >"$H/.local/state/liftoff/state.json"
  echo '{"autoUpdateCheck": true}' >"$H/.local/state/liftoff/misc.json"
  echo '{"username": ""}' >"$H/.local/state/liftoff/account.json"
  echo '{"doNotDisturb": false}' >"$H/.local/state/liftoff/notifications.json"
  echo tokyo-night >"$H/.local/state/liftoff/look-previous-theme"
  echo tokyo-night >"$H/.local/state/meniscus/previous-theme"
  echo "vm" >"$H/.local/state/liftoff/windows/mode"
  echo '{"upcoming": []}' >"$H/.cache/liftoff/launches.json"
  echo "objects" >"$H/.cache/meniscus/hyprglass/main.o"
  echo "log" >"$H/.cache/meniscus/build.log"
  # The notes of a clip hold its full path, under the old folder.
  printf '{"id": "launch", "file": "%s", "poster": "%s"}\n' "$H/.local/share/liftoff/clips/launch.mp4" "$H/.local/share/liftoff/clips/launch.jpg" \
    >"$H/.local/share/liftoff/clips/launch.json"
  cp "$H/.local/share/liftoff/clips/launch.json" "$H/.local/share/liftoff/clips/current.json"
  echo "clip" >"$H/.local/share/liftoff/clips/launch.mp4"
  echo "still" >"$H/.local/share/liftoff/clips/launch.jpg"
  echo "iso" >"$H/.local/share/liftoff/windows-tools/tools.zip"
  echo "fake engine, built long ago" >"$H/.local/lib/meniscus/hyprglass.so"
  printf '%s\nv0.56.2\n%s\n' "$PINNED_HYPRLAND" "$PINNED_ENGINE" >"$H/.local/lib/meniscus/hyprglass.built-for"
  echo "an older build" >"$H/.local/lib/meniscus/hyprglass.so.stock-radius"
  echo "read 13 times in ten seconds" >"$H/run/liftoff-glass-stood-down"
  printf '1\n13\n' >"$H/run/liftoff-glass-readings"
  mkdir -p "$H/run/liftoff"
  : >"$H/run/liftoff/launchd.sock"
  # Aether remembers the theme that was current before its own
  mkdir -p "$H/.config/aether"
  echo liftoff >"$H/.config/aether/previous-omarchy-theme"
  # the daemon is running, and the look is on
  : >"$H/.standin/active-liftoff-launchd.service"
}
