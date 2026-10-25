#!/bin/bash
# Vetro: build the glass engine again after an update, in case the update
# brought a new Hyprland. Omarchy runs this from `omarchy update`, once the
# system packages and its migrations are done (install.sh links it into
# ~/.config/omarchy/hooks/post-update.d).
#
# It never fails the update. If the new Hyprland has no pin yet, or the build
# breaks, the old engine stays where it is, nothing loads it on a Hyprland it
# was not built for, and the panes are plain blur until this succeeds.
here=$(dirname "$(readlink -f "$0")")
state=${XDG_STATE_HOME:-$HOME/.local/state}/vetro
mkdir -p "$state"

if "$here/build.sh" >"$state/build.log" 2>&1; then
  echo "vetro: the glass engine matches this Hyprland."
else
  echo "vetro: the glass engine was not rebuilt; the panes use plain blur until it is."
  echo "       $(tail -n 2 "$state/build.log" | head -n 1)"
fi

# Vetro's bar is built from Omarchy's own, which the update may have changed.
# It is built again from the new one. If that cannot be done, the shell is
# taken off Vetro's bar, so that it shows Omarchy's own and not none.
# shellcheck disable=SC2034  # lib/common.sh reads it
ROOT=$(dirname "$here")
# shellcheck source=lib/common.sh
. "$ROOT/lib/common.sh"
# shellcheck source=lib/lay.sh
. "$ROOT/lib/lay.sh"
if bar_build; then
  echo "vetro: its bar matches this Omarchy's."
elif [ "$(bar_in_use)" = vetro.bar ]; then
  omarchy plugin disable vetro.bar >/dev/null 2>&1 || true
fi
exit 0
