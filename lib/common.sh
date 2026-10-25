# shellcheck shell=bash
# shellcheck disable=SC2034  # names set here are used by the files that source this
# What Vetro's command and installers share: where things live, links, marked
# blocks in files that belong to someone else, and the few questions to ask
# Omarchy's shell. Sourced by them, never run. The caller sets ROOT (this
# checkout) before sourcing it.

: "${ROOT:?common.sh needs ROOT, the folder this repository is checked out in}"

# Where Vetro keeps its own things. Omarchy's own commands use $HOME/.config
# whatever XDG_CONFIG_HOME says, so the config folders follow them.
CONFIG=$HOME/.config
STATE=${XDG_STATE_HOME:-$HOME/.local/state}/vetro
ENGINE_DIR=$HOME/.local/lib/vetro
ENGINE=$ENGINE_DIR/hyprglass.so
PLUGINS_DIR=$CONFIG/omarchy/plugins
SHELL_TOML=$CONFIG/omarchy/shell.toml
HYPR_CONFIG=$CONFIG/hypr/hyprland.lua
# Present while Vetro is switched off.
FLAG=$STATE/off
# Where the browsers' policy folders are (the tests give a folder of their own).
POLICY_ROOT=${VETRO_POLICY_ROOT:-/etc}

# The marks around what Vetro adds to files that are not its own. A block is
# found by its marks alone, so the text in between can change from one
# version to the next.
LOOK_BEGIN="# >>> vetro look (vetro on adds this; vetro off removes it)"
LOOK_END="# <<< vetro look"
GLASS_BEGIN="# >>> vetro glass (vetro glass load adds this; vetro glass unload removes it)"
GLASS_END="# <<< vetro glass"
HYPR_BEGIN="-- >>> vetro (install.sh adds this block, uninstall.sh removes it)"
HYPR_END="-- <<< vetro"
# Through Omarchy's own helper, so a missing file is skipped instead of
# stopping Hyprland's config: deleting this folder must never cost a session.
HYPR_LINE='require("default.hypr.require_optional").module("hypr.vetro")'

say() {
  printf '\n%s\n' "$*"
}

warn() {
  printf 'vetro: %s\n' "$*" >&2
}

die() {
  warn "$*"
  exit 1
}

# ---------------------------------------------------------------- links -----

# link TARGET NAME: make NAME a symlink to TARGET. A file of the user's own in
# the way is kept beside it as NAME.pre-vetro, and uninstall.sh puts it back.
link() {
  local target=$1 name=$2 keep
  mkdir -p "$(dirname "$name")"
  if [ -L "$name" ]; then
    [ "$(readlink "$name")" = "$target" ] && return 0
    rm -f "$name"
  elif [ -e "$name" ]; then
    keep=$name.pre-vetro
    if [ -e "$keep" ] || [ -L "$keep" ]; then
      keep=$keep.$(date +%s)
    fi
    mv "$name" "$keep"
    echo "  kept the old $name as $keep"
  fi
  ln -s "$target" "$name"
  echo "  $name"
}

# seed SOURCE NAME: give NAME a copy of SOURCE, a settings file that is the
# user's to change. A link there into this folder (where an earlier Vetro
# linked the settings) becomes a copy of the file it shows; a file of the
# user's own, or a link of theirs, stays as it is.
seed() {
  local source=$1 name=$2 shown
  mkdir -p "$(dirname "$name")"
  if [ -L "$name" ]; then
    case $(readlink "$name") in
      "$ROOT"/*) ;;
      *) return 0 ;;
    esac
    shown=$(readlink -f "$name")
    rm -f "$name"
    cp "${shown:-$source}" "$name" 2>/dev/null || cp "$source" "$name"
    echo "  $name (now a copy of its own)"
  elif [ ! -e "$name" ]; then
    cp "$source" "$name"
    echo "  $name"
  fi
}

# unseed SOURCE NAME: take back a copy seed made, if it is still as it was
# made; one the user has changed is theirs, and stays.
unseed() {
  local source=$1 name=$2
  [ -f "$name" ] && [ ! -L "$name" ] || return 0
  if cmp -s "$source" "$name"; then
    rm -f "$name"
    echo "  $name"
  else
    echo "  kept $name, which has your settings"
  fi
}

# prune DIR...: remove these folders if nothing is in them, in the order given
# (the deepest first). Linking made them where they were not; taking the link
# away should not leave them behind.
prune() {
  local dir
  for dir in "$@"; do
    rmdir "$dir" 2>/dev/null || true
  done
}

# ours NAME: whether NAME is a link into this checkout, or into Vetro's state
# folder (where a file that has to be written for the machine, like the Qt
# settings with their absolute path, is kept).
ours() {
  local target
  [ -L "$1" ] || return 1
  target=$(readlink "$1")
  [[ $target == "$ROOT"/* || $target == "$STATE"/* || $target == "$ROOT" ]]
}

# unlink_gone DIR: remove the links in DIR into this checkout whose target is
# no longer there: a part an earlier Vetro linked that this one has dropped.
unlink_gone() {
  local name
  for name in "$1"/*; do
    if ours "$name" && [ ! -e "$name" ]; then
      rm -f "$name"
      echo "  $name, which this Vetro no longer has"
    fi
  done
}

# unlink_ours NAME: remove NAME if it is one of Vetro's links, and put back the
# file that link() set aside, if there was one.
unlink_ours() {
  local name=$1
  ours "$name" || return 0
  rm -f "$name"
  echo "  $name"
  if [ -e "$name.pre-vetro" ] || [ -L "$name.pre-vetro" ]; then
    mv "$name.pre-vetro" "$name"
    echo "  put the old $name back"
  fi
}

# ---------------------------------------------------------------- blocks ----

# block_put FILE BEGIN END: put the text on standard input between the two
# marks in FILE. A block that is there already is replaced where it stands; if
# there is none, one is added at the end, after a blank line. The file is
# written once and through its own name, so a link stays a link and a program
# that watches it (Omarchy's shell watches shell.toml) sees a single change.
block_put() {
  local file=$1 begin=$2 end=$3 text out
  text=$(mktemp)
  out=$(mktemp)
  cat >"$text"
  if [ -f "$file" ]; then
    cat "$file"
  fi | awk -v b="$begin" -v e="$end" -v t="$text" '
    function emit(   line) {
      print b
      while ((getline line < t) > 0) print line
      close(t)
      print e
    }
    { lines[NR] = $0 }
    END {
      start = 0; stop = 0
      for (i = 1; i <= NR; i++) {
        if (!start && lines[i] == b) start = i
        else if (start && !stop && lines[i] == e) stop = i
      }
      if (start) {
        if (!stop) stop = NR
        for (i = 1; i < start; i++) print lines[i]
        emit()
        for (i = stop + 1; i <= NR; i++) print lines[i]
      } else {
        last = NR
        while (last > 0 && lines[last] ~ /^[ \t]*$/) last--
        for (i = 1; i <= last; i++) print lines[i]
        if (last > 0) print ""
        emit()
      }
    }' >"$out"
  mkdir -p "$(dirname "$file")"
  # Nothing is written when nothing changes: the shell watching the file
  # would read it again for no reason.
  if ! cmp -s "$out" "$file" 2>/dev/null; then
    cat "$out" >"$file"
  fi
  rm -f "$text" "$out"
}

# block_has FILE BEGIN: whether the block is in FILE.
block_has() {
  [ -f "$1" ] && grep -qxF -- "$2" "$1"
}

# block_drop FILE BEGIN END: take the block out of FILE, and the blank line it
# was added after. A file with nothing left in it is removed: it was Vetro's
# to create.
block_drop() {
  local file=$1 begin=$2 end=$3 out
  block_has "$file" "$begin" || return 0
  out=$(mktemp)
  awk -v b="$begin" -v e="$end" '
    { lines[NR] = $0 }
    END {
      start = 0; stop = 0
      for (i = 1; i <= NR; i++) {
        if (!start && lines[i] == b) start = i
        else if (start && !stop && lines[i] == e) stop = i
      }
      if (!stop) stop = NR
      n = 0
      for (i = 1; i <= NR; i++) {
        if (i >= start && i <= stop) continue
        kept[++n] = lines[i]
      }
      while (n > 0 && kept[n] ~ /^[ \t]*$/) n--
      for (i = 1; i <= n; i++) print kept[i]
    }' "$file" >"$out"
  if grep -q '[^[:space:]]' "$out"; then
    cat "$out" >"$file"
  else
    rm -f "$file"
  fi
  rm -f "$out"
}

# overlay FILE: the text of an override file as it is laid into a block, with
# its comments and blank lines left out.
overlay() {
  grep -v '^#' "$1" | grep -v '^$' || true
}

# --------------------------------------------------------- Hyprland's file ---

# hypr_block_put: the one line that makes Hyprland read hypr/vetro.lua.
hypr_block_put() {
  [ -f "$HYPR_CONFIG" ] || return 1
  printf '%s\n' "$HYPR_LINE" | block_put "$HYPR_CONFIG" "$HYPR_BEGIN" "$HYPR_END"
}

hypr_block_drop() {
  block_drop "$HYPR_CONFIG" "$HYPR_BEGIN" "$HYPR_END"
}

# --------------------------------------------------------- Omarchy's shell ---

# shell_alive: whether Omarchy's shell is running and will answer.
shell_alive() {
  command -v omarchy-shell >/dev/null 2>&1 && omarchy-shell shell ping >/dev/null 2>&1
}

plugin_known() {
  omarchy-plugin-list --json 2>/dev/null | jq -e --arg id "$1" 'any(.[]; .id == $id)' >/dev/null 2>&1
}

# wait_for_plugin ID: a freshly linked plugin is found by the shell a moment
# after it is asked to look.
wait_for_plugin() {
  local _
  for _ in $(seq 40); do
    plugin_known "$1" && return 0
    sleep 0.1
  done
  return 1
}

# ----------------------------------------------------------------- addons ---

# An addon is a folder in addons/ with an addon.sh that answers install,
# uninstall, on, off and status. It counts as installed once its install has
# left a mark in the state folder.
ADDON_MARKS=$STATE/addons

addon_installed() {
  [ -e "$ADDON_MARKS/$1" ]
}

# installed_addons: the names, one a line.
installed_addons() {
  local mark
  for mark in "$ADDON_MARKS"/*; do
    [ -e "$mark" ] && basename "$mark"
  done
  return 0
}

# addon_run NAME ACTION: ask an addon to do something, if it is installed and
# has the script; what it prints is passed on.
addon_run() {
  local script=$ROOT/addons/$1/addon.sh
  addon_installed "$1" && [ -x "$script" ] || return 0
  "$script" "$2" || warn "the $1 addon could not $2"
}
