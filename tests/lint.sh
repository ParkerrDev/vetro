#!/bin/bash
# Checks that need nothing but the files: no desktop, no network, no root, and
# no compositor, shell or service (every script runs against stand-ins, in a
# home folder made for the purpose).
#
#   shell    bash -n, the executable bit and shellcheck on every script
#   Lua      luac -p on every file, then tests/lua.lua: hypr/vetro.lua against
#            a stand-in for Hyprland's API
#   Python   every script parses; the qmldir files are up to date
#   data     every TOML and JSON file parses; the launch theme has what Omarchy's
#            theme setter looks for; the shell plugins pass Omarchy's own
#            validator where that is installed, and qmllint finds no syntax
#            error or missing import folder in their QML where it is
#   tests    command, engine, install, migrate (the move from the earlier
#            projects, undone too), addons, system, qml (every plugin compiled
#            by Quickshell's engine), and names (no old name left where it does
#            not belong)
#   engine   with VETRO_TEST_ENGINE=1, the engine's own tests and the launch
#            daemon's (clippy with warnings as errors, then its tests; they compile)
#
# Hyprland's own `--verify-config` is deliberately not used. It is not a pure
# parser: it runs the compositor's start-up code, and aborts with a core dump
# when the environment is not a session's. A check that can crash the program
# it is checking, on the desktop of whoever runs it, is not one to run on every
# commit. `hyprctl configerrors` in a live session says the same and more.
#
# A tool that is not installed skips its check and says so.
set -uo pipefail

cd "$(dirname "$(readlink -f "$0")")/.." || exit 2
bad=0

fail() {
  echo "FAIL  $*"
  bad=1
}
skip() {
  echo "skip  $*"
}
first_of() {
  local tool
  for tool in "$@"; do
    command -v "$tool" 2>/dev/null && return 0
  done
  return 1
}

mapfile -t tracked < <(git ls-files --cached --others --exclude-standard)

# --- shell
scripts=()
for file in "${tracked[@]}"; do
  [ -f "$file" ] || continue
  case $file in third_party/* | engine/src/* | engine/docs/*) continue ;; esac
  if head -n 1 "$file" 2>/dev/null | grep -q '^#!.*\(bash\|/sh\)\b'; then
    scripts+=("$file")
    [ -x "$file" ] || fail "$file is a script but is not executable"
  elif [[ $file == lib/*.sh || $file == tests/lib.sh || $file == tests/owner-home.sh ]]; then
    scripts+=("$file")
  fi
done
for file in "${scripts[@]}"; do
  bash -n "$file" || fail "$file does not parse"
done
if command -v shellcheck >/dev/null; then
  shellcheck -x "${scripts[@]}" || fail "shellcheck"
else
  skip "shellcheck is not installed"
fi

# --- Lua
mapfile -t lua_files < <(printf '%s\n' "${tracked[@]}" | grep '\.lua$' | sort)
if luac=$(first_of luac luac5.4 luac5.3); then
  for file in "${lua_files[@]}"; do
    "$luac" -p "$file" || fail "$file does not parse"
  done
else
  skip "luac is not installed"
fi
if lua=$(first_of lua lua5.4 lua5.3); then
  # tests/lua.lua removes folders in its home, and runs only in one that holds
  # this mark.
  scratch=$(mktemp -d)
  mkdir -p "$scratch/home" "$scratch/run"
  touch "$scratch/home/.vetro-test-scratch" "$scratch/run/.vetro-test-scratch"
  env -i PATH="$PATH" HOME="$scratch/home" XDG_RUNTIME_DIR="$scratch/run" XDG_STATE_HOME="$scratch/home/.local/state" \
    VETRO_GUARD_TEST=1 "$lua" tests/lua.lua . || fail "tests/lua.lua"
  rm -rf "$scratch"
  # And once more as what it is, a reader that is not Hyprland: its readings
  # must not count toward the guard.
  scratch=$(mktemp -d)
  mkdir -p "$scratch/home" "$scratch/run"
  touch "$scratch/home/.vetro-test-scratch" "$scratch/run/.vetro-test-scratch"
  env -i PATH="$PATH" HOME="$scratch/home" XDG_RUNTIME_DIR="$scratch/run" XDG_STATE_HOME="$scratch/home/.local/state" \
    "$lua" tests/lua.lua . || fail "tests/lua.lua, read from outside Hyprland"
  rm -rf "$scratch"
else
  skip "lua is not installed: the Lua file was not run against the stand-in"
fi

# --- Python
if command -v python3 >/dev/null; then
  for file in "${tracked[@]}"; do
    [ -f "$file" ] || continue
    if [[ $file == *.py ]] || head -n 1 "$file" 2>/dev/null | grep -q '^#!.*python'; then
      python3 -I -c 'import ast, sys; ast.parse(open(sys.argv[1], encoding="utf-8").read(), sys.argv[1])' "$file" ||
        fail "$file does not parse"
    fi
  done
  python3 -I tools/gen-qmldir.py --check || fail "a qmldir is out of date (tools/gen-qmldir.py writes them)"
else
  skip "python3 is not installed"
fi

# --- data
if command -v python3 >/dev/null && python3 -c 'import tomllib' 2>/dev/null; then
  for file in "${tracked[@]}"; do
    [ -f "$file" ] || continue
    case $file in
      *.toml) python3 -I -c 'import sys, tomllib; tomllib.load(open(sys.argv[1], "rb"))' "$file" || fail "$file is not valid TOML" ;;
      *.json) python3 -I -c 'import json, sys; json.load(open(sys.argv[1]))' "$file" || fail "$file is not valid JSON" ;;
    esac
  done
else
  skip "python3 with tomllib is not installed: TOML and JSON were not parsed"
fi

# The launch theme has what Omarchy's theme picker and theme setter look for.
for file in colors.toml shell.toml hyprland.lua icons.theme preview.png; do
  [ -s "addons/theme/launch/$file" ] || fail "addons/theme/launch/$file is missing"
done
[ -n "$(find addons/theme/launch/backgrounds -type f \( -name '*.jpg' -o -name '*.png' \) 2>/dev/null)" ] ||
  fail "the launch theme has no background"
# The override files that are laid into blocks are what the blocks are made of.
for file in override/bar.toml override/glass.toml override/terminal/foot.ini override/terminal/kitty.conf \
  override/qt6ct/qt6ct.conf.in override/qt6ct/vetro.colors override/gtk-3.0/vetro.css override/gtk-4.0/vetro.css; do
  [ -s "$file" ] || fail "$file is missing"
done

# --- the shell plugins
plugins=(shell/vetro.* addons/launch/plugin/vetro.cards)
if command -v omarchy-plugin-validate >/dev/null; then
  for plugin in "${plugins[@]}"; do
    # the library the plugins share is not a plugin
    [ -f "$plugin/manifest.json" ] || continue
    omarchy-plugin-validate "$plugin" >/dev/null 2>&1 || fail "$plugin is not a valid Omarchy plugin"
  done
else
  skip "omarchy-plugin-validate is not installed"
fi

# qmllint on a copy laid out as install.sh lays the plugins out (a plugin
# imports the library, vetro.lib, by a path beside its own). Style is not checked: only
# QML that does not parse, and an import of a folder that is not there.
qmllint=$(first_of qmllint /usr/lib/qt6/bin/qmllint) || qmllint=
if [ -n "$qmllint" ] && command -v python3 >/dev/null; then
  scratch=$(mktemp -d)
  mkdir -p "$scratch/plugins"
  for plugin in "${plugins[@]}"; do cp -r "$plugin" "$scratch/plugins/$(basename "$plugin")"; done
  # Vetro's own parts of its bar, where the built bar has them (tests/qml.sh
  # builds the whole bar and compiles it).
  cp -r shell/bar "$scratch/plugins/vetro.bar"
  mapfile -t qml < <(cd "$scratch/plugins" && find . -name '*.qml' | sort)
  problems=$(cd "$scratch/plugins" && "$qmllint" --ignore-settings -I "$scratch" -I /usr/lib/qt6/qml --json - "${qml[@]}" 2>/dev/null |
    python3 -I -c '
import json, sys
try:
    data = json.load(sys.stdin)
except ValueError:
    print("qmllint printed nothing readable")
    sys.exit(0)
for f in data["files"]:
    for w in f["warnings"]:
        message = w["message"]
        if w.get("id") == "syntax" or w.get("type") == "error":
            print(f["filename"], w.get("line"), message[:120])
        elif w.get("id") == "import" and "does not exist" in message:
            print(f["filename"], w.get("line"), message[:120])
')
  if [ -n "$problems" ]; then
    fail "qmllint: $problems"
  fi
  rm -rf "$scratch"
else
  skip "qmllint is not installed: the plugins' QML was not read"
fi

# --- the tests that run the scripts against stand-ins
for test in command browser engine install migrate addons system qml names; do
  if ! bash "tests/$test.sh"; then
    fail "tests/$test.sh"
  fi
done

if [ "${VETRO_TEST_ENGINE:-}" = 1 ]; then
  make -C engine test || fail "the engine's own tests (make -C engine test)"
  launchd=addons/launch/launchd/Cargo.toml
  cargo clippy --locked --quiet --manifest-path "$launchd" --all-targets -- -D warnings || fail "clippy on the launch daemon"
  cargo test --locked --quiet --manifest-path "$launchd" >/dev/null || fail "the launch daemon's own tests"
  notifyd=shell/notifyd/Cargo.toml
  cargo clippy --locked --quiet --manifest-path "$notifyd" --all-targets -- -D warnings || fail "clippy on the notification daemon"
  cargo test --locked --quiet --manifest-path "$notifyd" >/dev/null || fail "the notification daemon's own tests"
else
  skip "the engine's and the two daemons' own tests compile; VETRO_TEST_ENGINE=1 runs them"
fi

if [ "$bad" = 1 ]; then
  exit 1
fi
echo "lint: all checks passed"
