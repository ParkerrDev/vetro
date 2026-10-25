#!/bin/bash
# Omarchy runs this after every change of theme, with the new theme's name.
#
# Vetro is not part of any theme: the glass and its colours stay through every
# change of theme and wallpaper, and nothing here looks at which theme it is.
# Omarchy's shell keeps ~/.config/omarchy/shell.toml over the theme's own, key
# by key, so the colours are already in force. This asks `vetro glass load` to
# see that the engine is in Hyprland and the colours are in the file, which is
# also what login does, and to give the browser the glass's grey again: Omarchy
# has just given it the theme's colour. While Vetro is off it takes nothing in
# and puts nothing in.
command -v vetro >/dev/null 2>&1 && vetro glass load
exit 0
