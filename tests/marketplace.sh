#!/bin/bash
set -uo pipefail
# shellcheck source=lib.sh
# shellcheck source-path=SCRIPTDIR
. "$(dirname "$(readlink -f "$0")")/lib.sh"
make_bin
new_home marketplace
is "fresh launcher asks for setup" setup "$(run "$REPO/marketplace/control" state)"
no "noninteractive setup refuses changes" run "$REPO/marketplace/control" setup </dev/null
no "setup did not install Vetro" test -e "$H/.config/hypr/vetro.lua"
no "toggle before setup refuses changes" run "$REPO/marketplace/control" toggle
no "open before setup refuses changes" run "$REPO/marketplace/control" open
no "piped installer refuses to fetch and execute code" run bash -s <"$REPO/install.sh"
no "unknown operation fails" run "$REPO/marketplace/control" unexpected

# Install over another project's live and broken symlinks. Neither the target
# nor a previous backup may be lost, and uninstall puts the original link back.
run bash -c '
  ROOT=$1
  . "$ROOT/lib/common.sh"
  mkdir -p "$HOME/foreign"
  printf "mine\n" > "$HOME/foreign/file"
  ln -s "$HOME/foreign/file" "$HOME/setting"
  printf "earlier\n" > "$HOME/setting.pre-vetro"
  link "$ROOT/LICENSE" "$HOME/setting"
  test "$(readlink "$HOME/setting.pre-vetro.1")" = "$HOME/foreign/file" || exit 1
  test "$(cat "$HOME/setting.pre-vetro")" = earlier || exit 1
  test "$(cat "$HOME/foreign/file")" = mine || exit 1
  ln -s "$HOME/foreign/missing" "$HOME/broken"
  link "$ROOT/LICENSE" "$HOME/broken"
  unlink_ours "$HOME/broken"
  test "$(readlink "$HOME/broken")" = "$HOME/foreign/missing"
' bash "$REPO" >"$WORK/links.out" 2>&1
is "foreign links and earlier backups survive" 0 "$?"
finish
