#!/bin/bash
# Every shell plugin's QML, compiled by Quickshell's own engine through the links
# install.sh makes: each import resolves (the plugins import their library as a
# sibling, ../vetro.lib, which works only through the link), and no file has a
# syntax error. A component that creates a window cannot
# be finished here, with no compositor to give it one; the only error allowed for
# it is "No PanelWindow backend loaded", which Quickshell gives after every
# import above that line has been resolved.
#
# It runs quickshell with no display, no D-Bus and a scratch home, so it cannot
# reach the session it is run from. It is skipped where quickshell or Omarchy's
# shell is not installed.
set -uo pipefail
# shellcheck source=lib.sh
# shellcheck source-path=SCRIPTDIR
. "$(dirname "$(readlink -f "$0")")/lib.sh"

if ! command -v quickshell >/dev/null || [ ! -d /usr/share/omarchy/shell/Commons ]; then
  echo "skip  tests/qml.sh needs quickshell and Omarchy's shell (qs.Commons)"
  exit 0
fi
make_bin
new_home qml
# Vetro's bar is built here from the bar of the Omarchy that is installed, as
# it is on a real machine, and compiled with the rest.
BAR_SOURCE=/usr/share/omarchy/shell/plugins/bar
run "$REPO/install.sh" --no-build >"$WORK/install.out" 2>&1 || fail "install.sh failed"
ok "Vetro's bar builds from this Omarchy's bar: $(grep -m1 'is not built' "$WORK/install.out")" test -f "$H/.config/omarchy/plugins/vetro.bar/Bar.qml"

# The config root has Omarchy's modules beside the harness: `import qs.Commons`
# is a folder in the config's own root.
mkdir -p "$WORK/cfg"
ln -s /usr/share/omarchy/shell/Commons "$WORK/cfg/Commons"
ln -s /usr/share/omarchy/shell/Ui "$WORK/cfg/Ui"
cp "$REPO/tests/qml/shell.qml" "$WORK/cfg/shell.qml"

env -i HOME="$H" PATH=/usr/bin XDG_RUNTIME_DIR="$H/run" QT_QPA_PLATFORM=offscreen OMARCHY_PATH=/usr/share/omarchy \
  PLUGINS="$H/.config/omarchy/plugins" nice -n 19 timeout 12 quickshell -n -p "$WORK/cfg" >"$WORK/qml.log" 2>&1
has "the harness ran to its end" "$WORK/qml.log" "COMPONENTS DONE"
while read -r line; do
  entry=${line#*COMPONENT }
  entry=${entry%% status=*}
  status=$(sed -n 's/.* status=\([0-9]\).*/\1/p' <<<"$line")
  case $status in
    1) checks=$((checks + 1)) ;;
    3)
      # Every error but the one allowed: Quickshell has no window to give.
      rest=${line#*status=3}
      other=$(printf '%s\n' "${rest// | /$'\n'}" | grep -v -e '^[[:space:]]*$' -e 'No PanelWindow backend loaded' -e 'unavailable$' || true)
      if [ -n "$other" ]; then fail "$entry: $other"; else checks=$((checks + 1)); fi
      ;;
    *) fail "$entry: no status in '$line'" ;;
  esac
done < <(grep 'COMPONENT vetro' "$WORK/qml.log" | sed 's/\x1b\[[0-9;]*m//g')
is "all thirteen entry points were looked at" 13 "$(grep -c 'COMPONENT vetro' "$WORK/qml.log")"

finish
