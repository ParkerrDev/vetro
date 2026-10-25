#!/bin/bash
# Builds the glass engine for the Hyprland that is installed: this project's
# fork of hyprglass (https://github.com/hyprnux/hyprglass, BSD-3-Clause), which
# takes every card on a surface for a pane of its own and draws it for about a
# millisecond. The build is put where hypr/vetro.lua and `vetro glass load`
# look for it:
#
#   ~/.local/lib/vetro/hyprglass.so
#   ~/.local/lib/vetro/hyprglass.built-for   the Hyprland it was built for
#
#   engine/build.sh           build if there is nothing built for this Hyprland
#   engine/build.sh --force   build again regardless
#   engine/build.sh --check   build nothing; say whether the engine is built for
#                             this Hyprland (exit 0), is not (1), or has no pin (3)
#
# A compositor plugin has to match its compositor exactly, so what to build
# comes from engine/pins and nowhere else. A Hyprland that is not listed there
# is refused (exit 3) and whatever was built before is left in place: nothing
# loads an engine built for another Hyprland, and the panes are plain blur
# until a pin exists.
#
# The engine is part of this repository, so the commit a pin names is fetched
# from this repository and not from a remote: it is one of the commits that
# came in with the engine's history, or one made since. VETRO_ENGINE_SOURCE
# names another place to fetch it from, a mirror for one.
#
# It needs no root. It does need git, make, a C++ compiler and Hyprland's
# headers, which Omarchy's hyprland package installs.
set -euo pipefail

HERE=$(dirname "$(readlink -f "$0")")
PINS=$HERE/pins
UPSTREAM=${VETRO_ENGINE_SOURCE:-$(cd "$HERE/.." && pwd -P)}
LIB=$HOME/.local/lib/vetro
SOURCE=${XDG_CACHE_HOME:-$HOME/.cache}/vetro/engine
STAMP=$LIB/hyprglass.built-for

die() {
  echo "engine/build.sh: $*" >&2
  exit 1
}

usage() {
  awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"
}

# The headers say which Hyprland a plugin built now will belong to. After an
# update that is the Hyprland of the next login, which is the one to build for.
# The header sits under one of the include directories pkg-config names; which
# one differs between packagings, so each is tried. Sets hyprland_commit and
# hyprland_tag.
read_installed_hyprland() {
  local header='' flag candidate flags
  # cargo too: the engine's logic is Rust, built into the plugin by its Makefile.
  for tool in git make pkg-config g++ cargo; do
    command -v "$tool" >/dev/null || die "$tool is not installed"
  done
  pkg-config --exists hyprland || die "Hyprland's headers are not installed (pkg-config finds no hyprland)"
  read -ra flags <<<"$(pkg-config --cflags-only-I hyprland)"
  for flag in "${flags[@]}"; do
    for candidate in "${flag#-I}/src/version.h" "${flag#-I}/hyprland/src/version.h"; do
      if [ -r "$candidate" ] && grep -q GIT_COMMIT_HASH "$candidate"; then
        header=$candidate
        break 2
      fi
    done
  done
  [ -n "$header" ] || die "cannot find Hyprland's version.h under its include directories"
  hyprland_commit=$(sed -n 's/^#define GIT_COMMIT_HASH *"\([0-9a-f]\{40\}\)".*/\1/p' "$header")
  hyprland_tag=$(sed -n 's/^#define GIT_TAG *"\([^"]*\)".*/\1/p' "$header")
  [ -n "$hyprland_commit" ] || die "no commit in $header"
}

# The pin for the installed Hyprland: sets pin (an engine commit) and
# pin_folder (where the engine's Makefile is in that commit, "." unless the
# line says otherwise). Fails if the installed Hyprland has no line.
find_pin() {
  read -r pin pin_folder <<<"$(awk -v commit="$hyprland_commit" \
    '$1 == commit { print $2, ($3 == "" || $3 ~ /^#/) ? "." : $3; exit }' "$PINS")"
  [ -n "$pin" ]
}

up_to_date() {
  [ -s "$LIB/hyprglass.so" ] && [ "$(sed -n 1p "$STAMP" 2>/dev/null)" = "$hyprland_commit" ] &&
    [ "$(sed -n 3p "$STAMP" 2>/dev/null)" = "$pin" ]
}

shallow() { [ "$(git -C "$1" rev-parse --is-shallow-repository 2>/dev/null)" = true ]; }

# fetch_pinned: the pinned commit of the engine, checked out and clean in
# $SOURCE. Objects left from another commit would be linked into this one.
fetch_pinned() {
  if [ ! -d "$SOURCE/.git" ]; then
    mkdir -p "$(dirname "$SOURCE")"
    git clone --quiet --no-checkout "$UPSTREAM" "$SOURCE"
  fi
  # A clone kept from before may have been made from somewhere else.
  git -C "$SOURCE" remote set-url origin "$UPSTREAM"
  # A shallow clone of Vetro (git clone --depth 1) lacks the pinned commit
  # when it is not the newest, and so does a clone made from it: the history
  # is fetched whole first.
  if ! git -C "$UPSTREAM" cat-file -e "$pin^{commit}" 2>/dev/null && shallow "$UPSTREAM"; then
    git -C "$UPSTREAM" fetch --quiet --unshallow || die "cannot fetch the history that holds commit $pin"
  fi
  if ! git -C "$SOURCE" cat-file -e "$pin^{commit}" 2>/dev/null; then
    if shallow "$SOURCE"; then
      git -C "$SOURCE" fetch --quiet --tags --unshallow origin
    else
      git -C "$SOURCE" fetch --quiet --tags origin
    fi
  fi
  git -C "$SOURCE" cat-file -e "$pin^{commit}" 2>/dev/null || die "this repository has no commit $pin (engine/pins names it)"
  git -C "$SOURCE" -c advice.detachedHead=false checkout --quiet --force "$pin"
  git -C "$SOURCE" clean -fdxq
  [ -f "$SOURCE/$pin_folder/Makefile" ] || die "commit ${pin:0:12} has no engine Makefile in $pin_folder"
}

# compile: eight jobs at most, enough to be quick and kind to a laptop, and at
# a lower priority so the desktop stays smooth while it runs.
compile() {
  local jobs log
  jobs=$(nproc 2>/dev/null || echo 2)
  if [ "$jobs" -gt 8 ]; then jobs=8; fi
  log=$(dirname "$SOURCE")/build.log
  echo "Building the engine ${pin:0:12} for Hyprland ${hyprland_tag:-$hyprland_commit}..."
  if ! nice -n 10 make -C "$SOURCE/$pin_folder" -j"$jobs" >"$log" 2>&1; then
    tail -n 20 "$log" >&2
    die "the build failed (all of it: $log)"
  fi
  [ -s "$SOURCE/$pin_folder/hyprglass.so" ] || die "the build produced no hyprglass.so"
}

# install_built: moved into place, never written over: a running Hyprland may
# have the old file loaded, and keeps it until the next login.
install_built() {
  mkdir -p "$LIB"
  install -m 0755 "$SOURCE/$pin_folder/hyprglass.so" "$LIB/hyprglass.so.new"
  # The debug information is nine tenths of the file (40 MB) and of no use
  # to the compositor: the installed copy goes without it, and loads that
  # much less (a file strip cannot read is installed as it is). The build
  # folder keeps the full one.
  if command -v strip >/dev/null; then
    strip --strip-debug "$LIB/hyprglass.so.new" 2>/dev/null || true
  fi
  mv -f "$LIB/hyprglass.so.new" "$LIB/hyprglass.so"
  printf '%s\n%s\n%s\n' "$hyprland_commit" "$hyprland_tag" "$pin" >"$STAMP.new"
  mv -f "$STAMP.new" "$STAMP"
  echo "Built $LIB/hyprglass.so. Hyprland picks it up at the next login or reload."
}

mode=build
case ${1:-} in
  --force) mode=force ;;
  --check) mode=check ;;
  "") ;;
  *)
    usage
    exit 2
    ;;
esac

hyprland_commit='' hyprland_tag='' pin='' pin_folder=.
read_installed_hyprland

if ! find_pin; then
  echo "engine/build.sh: no engine is pinned for Hyprland ${hyprland_tag:-?} ($hyprland_commit)." >&2
  echo "Nothing was built. The panes use plain blur until engine/pins has a line for it." >&2
  exit 3
fi

if [ "$mode" != force ] && up_to_date; then
  echo "The glass engine is already built for Hyprland ${hyprland_tag:-$hyprland_commit}."
  exit 0
fi
if [ "$mode" = check ]; then
  echo "the engine is not built for Hyprland ${hyprland_tag:-$hyprland_commit}" >&2
  exit 1
fi

fetch_pinned
compile
install_built
