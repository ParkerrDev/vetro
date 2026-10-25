#!/bin/bash
# Omarchy runs this after every change of theme, with the new theme's name.
# The launch theme wants GTK the way the launch look had it: adw-gtk3-dark
# (libadwaita's look for GTK 3 apps, which Vetro's gtk.css turns black and
# white), D-DIN, and the XCursor-Pro cursor. Omarchy sets Adwaita-dark and its
# own font after every theme. Choosing another theme takes back what this set,
# and only that: a value you changed since is left alone.
THEME=launch
SCHEMA=org.gnome.desktop.interface

command -v gsettings >/dev/null 2>&1 || exit 0

if [[ ${1:-} == "$THEME" ]]; then
  gsettings set "$SCHEMA" gtk-theme adw-gtk3-dark
  gsettings set "$SCHEMA" color-scheme prefer-dark
  gsettings set "$SCHEMA" font-name 'D-DIN 11'
  gsettings set "$SCHEMA" accent-color slate
  gsettings set "$SCHEMA" cursor-theme XCursor-Pro-Dark
  gsettings set "$SCHEMA" cursor-size 24
  exit 0
fi

for pair in "font-name:'D-DIN 11'" "accent-color:'slate'" "cursor-theme:'XCursor-Pro-Dark'" "cursor-size:24"; do
  key=${pair%%:*}
  [[ $(gsettings get "$SCHEMA" "$key" 2>/dev/null) == "${pair#*:}" ]] && gsettings reset "$SCHEMA" "$key"
done
exit 0
