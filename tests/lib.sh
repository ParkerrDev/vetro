# shellcheck shell=bash
# shellcheck disable=SC2034  # names set here are used by the scripts that source this
# What the tests share. They cannot reach a real desktop: every script under
# test runs with an empty environment, a HOME of its own in a scratch folder,
# and a PATH that holds only the basic tools it uses plus stand-ins for
# Omarchy's, Hyprland's and the system's commands (tests/standins), which write
# down what they were asked and do nothing else. Nothing here starts, asks or
# configures a compositor, a shell or a service, and nothing is written outside
# the scratch folder.
#
# Sourced by the test scripts, which need `set -uo pipefail` (not -e: a failed
# check is reported and the run goes on).

REPO=$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.." && pwd -P)
WORK=$(mktemp -d "${TMPDIR:-/tmp}/vetro-test.XXXXXX") || exit 2
# Everything below is built on this path: without it, stop before anything
# could land in a real one.
[ -n "$WORK" ] && [ -d "$WORK" ] || exit 2
trap 'rm -rf "$WORK"' EXIT
bad=0
checks=0

# Real Hyprland's commit for the pin that engine/pins carries, so that the real
# pins file and the real build script are what the tests run.
PINNED_HYPRLAND=$(awk '$1 !~ /^#/ && NF >= 2 { print $1; exit }' "$REPO/engine/pins")
PINNED_ENGINE=$(awk '$1 !~ /^#/ && NF >= 2 { print $2; exit }' "$REPO/engine/pins")

fail() {
  echo "FAIL  $*"
  bad=1
}

# is DESCRIPTION EXPECTED ACTUAL
is() {
  checks=$((checks + 1))
  [ "$2" = "$3" ] || fail "$1: wanted '$2', got '$3'"
}

# ok DESCRIPTION COMMAND...: the command succeeds.
ok() {
  local what=$1
  shift
  checks=$((checks + 1))
  "$@" >/dev/null 2>&1 || fail "$what"
}

# no DESCRIPTION COMMAND...: the command fails.
no() {
  local what=$1
  shift
  checks=$((checks + 1))
  if "$@" >/dev/null 2>&1; then fail "$what"; fi
}

# says DESCRIPTION TEXT COMMAND...: the command's output, both streams, has the
# text. (Not a pipe into grep -q: that ends the command early and, with
# pipefail, counts as its failure.)
says() {
  local what=$1 text=$2 out
  shift 2
  checks=$((checks + 1))
  out=$("$@" 2>&1)
  grep -qF -- "$text" <<<"$out" || fail "$what (got: $(head -c 300 <<<"$out"))"
}

# has DESCRIPTION FILE TEXT / lacks: a fixed string is in the file or is not.
has() {
  checks=$((checks + 1))
  grep -qF -- "$3" "$2" 2>/dev/null || fail "$1 ($2 lacks '$3')"
}
lacks() {
  checks=$((checks + 1))
  if grep -qF -- "$3" "$2" 2>/dev/null; then fail "$1 ($2 has '$3')"; fi
}

# The tools the scripts may use, and no others.
TOOLS=(bash sh env cat cp mv rm ln mkdir rmdir touch dirname basename readlink date seq sleep head tail sed awk grep
  tr sort uniq cut wc find ls mktemp cmp diff chmod install tac id nice nproc stat tee xargs jq git python3 printf
  md5sum sha256sum true false test expr)

make_bin() {
  local tool path
  BIN=$WORK/bin
  mkdir -p "$BIN"
  for tool in "${TOOLS[@]}"; do
    path=$(command -v "$tool") || {
      echo "skip  $0 needs $tool"
      exit 0
    }
    ln -sf "$path" "$BIN/$tool"
  done
  for tool in "$REPO"/tests/standins/*; do
    ln -sf "$tool" "$BIN/$(basename "$tool")"
  done
}

# new_home NAME: an Omarchy home folder with nothing of Vetro's in it. Sets H
# and the environment run() gives its commands.
new_home() {
  H=$WORK/$1
  mkdir -p "$H/.config/hypr" "$H/.config/omarchy/plugins" "$H/.config/omarchy/hooks/theme-set.d" \
    "$H/.config/omarchy/hooks/post-update.d" "$H/.local/state/omarchy/current" "$H/.local/bin" "$H/run" "$H/.standin"
  chmod 700 "$H/run"
  # The notification daemon, as if a build had made it (the tests build
  # nothing): Vetro's notifications are left off without one.
  printf '#!/bin/sh\nexit 0\n' >"$H/.local/bin/vetro-notifyd"
  chmod +x "$H/.local/bin/vetro-notifyd"
  cat >"$H/.config/hypr/hyprland.lua" <<'EOF'
-- Learn how to configure Hyprland: https://wiki.hypr.land/Configuring/Start/
require("default.hypr.omarchy")
require("hypr.monitors")
require("hypr.bindings")

-- Add any other personal Hyprland configuration below.
EOF
  cat >"$H/.config/omarchy/shell.json" <<'EOF'
{
  "bar": {
    "layout": {
      "center": [{"id": "omarchy.clock"}],
      "left": [{"id": "omarchy.menu"}, {"id": "omarchy.workspaces"}],
      "right": [{"id": "omarchy.tray"}, {"id": "omarchy.network"}]
    },
    "position": "top"
  },
  "plugins": [{"id": "twofa"}],
  "version": 1
}
EOF
  chmod 600 "$H/.config/omarchy/shell.json"
  echo wallhaven-k8j6w1 >"$H/.local/state/omarchy/current/theme.name"
  mkdir -p "$H/.config/omarchy/plugins/twofa"
  echo '{"schemaVersion":1,"id":"twofa","name":"twofa","version":"1","kinds":["panel"],"entryPoints":{"panel":"P.qml"}}' >"$H/.config/omarchy/plugins/twofa/manifest.json"
  # A Hyprland whose headers are here, and whose commit is the pinned one.
  mkdir -p "$H/include/hyprland/src"
  printf '#define GIT_COMMIT_HASH "%s"\n#define GIT_TAG "v0.56.2"\n' "$PINNED_HYPRLAND" >"$H/include/hyprland/src/version.h"
}

# run [VAR=value]... COMMAND...: a command as the user of the scratch home, in
# the session's environment, with only the stand-ins and basic tools on its
# PATH. Extra variables go before the command, as with env.
run() {
  env -i HOME="$H" PATH="$BIN:$H/.local/bin" XDG_RUNTIME_DIR="$H/run" XDG_STATE_HOME="$H/.local/state" \
    XDG_CACHE_HOME="$H/.cache" XDG_DATA_HOME="$H/.local/share" HYPRLAND_INSTANCE_SIGNATURE=test \
    STANDIN_STATE="$H/.standin" FAKE_HYPRLAND_COMMIT="$PINNED_HYPRLAND" FAKE_HYPRLAND_INCLUDE="$H/include" \
    VETRO_DRM_ROOT="$H/no-drm" VETRO_DRI_ROOT="$H/no-dri" VETRO_POLICY_ROOT="$H/etc" VETRO_BAR_SOURCE="${BAR_SOURCE:-$REPO/tests/fixtures/bar}" "$@"
}

# The stand-ins' record, and the lines of it that match.
calls() { cat "$H/.standin/log" 2>/dev/null || true; }
called() { calls | grep -qF -- "$1"; }
forget_calls() { : >"$H/.standin/log"; }

theme() { cat "$H/.local/state/omarchy/current/theme.name"; }

# snapshot: everything in the home folder that a script could have changed, as
# text: each name, what kind it is, where a link leads, and a sum of each file.
# The stand-ins' own record and the session's runtime folder are left out.
snapshot() {
  (
    cd "$H" || exit 2
    find . \( -path ./.standin -o -path ./run \) -prune -o -printf '%y %p %l\n' | sort
    find . \( -path ./.standin -o -path ./run \) -prune -o -type f -exec md5sum {} + | sort -k2
  )
}

finish() {
  local name=${1:-${0##*/}}
  if [ "$bad" = 1 ]; then
    echo "$name: FAILED"
    exit 1
  fi
  echo "$name: $checks checks passed"
}
