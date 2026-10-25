#!/bin/bash
# The launch theme: pure black and white, in the SpaceX launch card's style,
# with a transparent background for the launch addon's video wallpaper.
# It is an ordinary Omarchy theme, chosen like any other (the theme menu, or
# `omarchy theme set launch`). Vetro never chooses it, and Vetro's glass does
# not need it: the glass lies over whichever theme you pick.
#
#   addon.sh install       link the theme and the hook that sets GTK for it
#   addon.sh uninstall     take them out
#   addon.sh on | off      nothing: a theme is not switched by Vetro
#   addon.sh status

# shellcheck source-path=SCRIPTDIR
set -euo pipefail

HERE=$(dirname "$(readlink -f "$0")")
ROOT=$(cd "$HERE/../.." && pwd -P)
# shellcheck source=../../lib/common.sh
. "$ROOT/lib/common.sh"

NAME=theme
HOOK=$CONFIG/omarchy/hooks/theme-set.d/60-vetro-launch-theme.sh

case ${1:-} in
  install)
    link "$HERE/launch" "$CONFIG/omarchy/themes/launch"
    link "$HERE/hook/60-vetro-launch-theme.sh" "$HOOK"
    mkdir -p "$ADDON_MARKS"
    : >"$ADDON_MARKS/$NAME"
    ;;
  uninstall)
    unlink_ours "$CONFIG/omarchy/themes/launch"
    unlink_ours "$HOOK"
    rm -f "$ADDON_MARKS/$NAME"
    prune "$CONFIG/omarchy/themes"
    ;;
  on | off) ;;
  status) echo "theme: the launch theme is installed (choose it in Omarchy's theme menu)" ;;
  *)
    awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"
    exit 2
    ;;
esac
