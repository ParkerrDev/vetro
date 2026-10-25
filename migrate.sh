#!/bin/bash
# Takes a home folder that has liftoff and meniscus installed over to Vetro:
# their links, the marked blocks they added to other programs' files, their
# service and their state, to Vetro's names. It leaves no dangling link and no
# stale block behind, and a second run finds nothing to do. install.sh runs it
# once Vetro's own links are in place; it can be run alone too.
#
#   migrate.sh             do it
#   migrate.sh --dry-run   say what it would do, and change nothing
#   migrate.sh --check     change nothing; exit 1 if anything of the two earlier
#                          projects is still in this home folder
#   migrate.sh --undo      uninstall Vetro and put this home folder back as it
#                          was before the last migration (the record is in
#                          ~/.local/state/vetro-migrated)
#
# What is taken over: the links in ~/.config and ~/.local/bin; the blocks in
# shell.toml, kitty.conf, foot.ini and hyprland.lua, and the plugin ids in the
# shell's own shell.json (each id renamed where it stands, so the bar keeps its
# order); the key you bound to the old command in ~/.config/hypr/bindings.lua;
# the launch daemon's service; the state, the cache, the clips and the built
# glass engine. Your theme and wallpaper are not touched, and neither are
# files that merely sit beside the old ones (the Windows VM's, which are
# passthru's now). The theme and the login screen that liftoff had are the
# addons' to install: addons/theme and addons/login.
#
# Everything it replaces is first saved in the record, so --undo can put it
# back. Caches are the one exception: they are rebuilt.
set -euo pipefail

ROOT=$(dirname "$(readlink -f "$0")")
# shellcheck source=lib/common.sh
. "$ROOT/lib/common.sh"

# shellcheck disable=SC2034
MODE=run
case ${1:-} in
  --dry-run) MODE=dry ;;
  --check) MODE=check ;;
  --undo) MODE=undo ;;
  "") ;;
  *)
    awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"
    exit 2
    ;;
esac

XDG_STATE=${XDG_STATE_HOME:-$HOME/.local/state}
XDG_CACHE=${XDG_CACHE_HOME:-$HOME/.cache}
XDG_DATA=${XDG_DATA_HOME:-$HOME/.local/share}
OLD_STATE=$XDG_STATE/liftoff
OLD_MENISCUS_STATE=$XDG_STATE/meniscus
OLD_CACHE=$XDG_CACHE/liftoff
OLD_MENISCUS_CACHE=$XDG_CACHE/meniscus
OLD_SHARE=$XDG_DATA/liftoff
OLD_LIB=$HOME/.local/lib/meniscus
OLD_SERVICE=liftoff-launchd.service
RECORDS=$XDG_STATE/vetro-migrated

# The links the two projects made, relative to the home folder. One is taken
# only if it leads into something named for them, so a file of yours with the
# same name is left alone.
OLD_LINKS=(
  .config/gtk-3.0/gtk.css .config/gtk-3.0/matugen.css .config/gtk-3.0/settings.ini
  .config/gtk-4.0/gtk.css .config/gtk-4.0/matugen.css .config/gtk-4.0/settings.ini
  .config/hypr/liftoff-look.lua .config/hypr/meniscus.lua
  .config/Kvantum/kvantum.kvconfig .config/Kvantum/liftoff
  .config/liftoff
  .config/omarchy/hooks/post-update.d/50-meniscus-glass.sh
  .config/omarchy/hooks/theme-set.d/50-liftoff.sh
  .config/omarchy/hooks/theme-set.d/50-meniscus.sh
  .config/omarchy/plugins/liftoff
  .config/omarchy/plugins/liftoff.barglass .config/omarchy/plugins/liftoff.cards
  .config/omarchy/plugins/liftoff.center .config/omarchy/plugins/liftoff.center-button
  .config/omarchy/plugins/liftoff.lock .config/omarchy/plugins/liftoff.menu
  .config/omarchy/plugins/liftoff.notifications .config/omarchy/plugins/liftoff.osd
  .config/omarchy/plugins/liftoff.windows-button .config/omarchy/plugins/meniscus.bar
  .config/omarchy/themes/liftoff .config/omarchy/themes/meniscus .config/omarchy/themes/meniscus-light
  .config/qt6ct/colors/matugen-kde.colors .config/qt6ct/qt6ct.conf
  .config/systemd/user/liftoff-launchd.service
  .config/systemd/user/liftoff-launchd.service.d/intel-decode.conf
  .local/bin/liftoff .local/bin/liftoff-launchd .local/bin/liftoff-look .local/bin/meniscus
  .local/share/fonts/liftoff
)

# The files that carry blocks of theirs, and the old marks.
OLD_LOOK_BEGIN='# >>> liftoff look'
OLD_LOOK_END='# <<< liftoff look'
OLD_GLASS_BEGIN='# >>> liftoff glass'
OLD_GLASS_END='# <<< liftoff glass'
OLD_BLOCK_FILES=(.config/omarchy/shell.toml .config/foot/foot.ini .config/kitty/kitty.conf)

# ------------------------------------------------------------- the record ----

RUN=''
J=''
B=''

# plan TEXT: report one step; true if it is to be done (not a dry run).
plan() {
  echo "  $*"
  [ "$MODE" != dry ]
}

# open_record: the record of this run, made when the first change needs it.
open_record() {
  [ -n "$RUN" ] && return 0
  RUN=$RECORDS/$(date +%Y%m%d-%H%M%S)-$$
  J=$RUN/journal
  B=$RUN/files
  mkdir -p "$B"
  : >"$J"
}

journal() {
  open_record
  printf '%s\t%s\t%s\n' "$1" "$2" "${3:-}" >>"$J"
}

# keep FILE: a copy of a file before it is changed or removed, once a run.
keep() {
  local file=$1 copy
  [ -f "$file" ] || return 0
  open_record
  copy=$B/$(printf '%s' "$file" | tr '/ ' '__')
  [ -e "$copy" ] && return 0
  cp -p "$file" "$copy"
  journal F "$file" "$copy"
}

# drop_link NAME: remove one of the old links.
drop_link() {
  local name=$1 target
  [ -L "$name" ] || return 0
  target=$(readlink "$name")
  case $target in
    *liftoff* | *meniscus*) ;;
    *) return 0 ;;
  esac
  if plan "remove the link ${name#"$HOME"/} (to $target)"; then
    journal L "$name" "$target"
    rm -f "$name"
  fi
}

# take SOURCE DEST: move something across, unless it is not there or the
# destination is.
take() {
  local src=$1 dst=$2
  [ -e "$src" ] || [ -L "$src" ] || return 0
  if [ -e "$dst" ] || [ -L "$dst" ]; then
    echo "  ${dst#"$HOME"/} is there already: left ${src#"$HOME"/} where it is"
    return 0
  fi
  if plan "move ${src#"$HOME"/} to ${dst#"$HOME"/}"; then
    mkdir -p "$(dirname "$dst")"
    journal M "$src" "$dst"
    mv "$src" "$dst"
  fi
}

# discard FILE: a small file of the old projects' that nothing needs now.
discard() {
  [ -e "$1" ] || return 0
  if plan "remove ${1#"$HOME"/}"; then
    if [ -f "$1" ]; then
      keep "$1"
    fi
    rm -f "$1"
  fi
}

# tidy DIR: a folder the old projects had, once nothing is in it.
tidy() {
  [ -d "$1" ] && [ -z "$(ls -A "$1" 2>/dev/null)" ] || return 0
  if plan "remove the empty folder ${1#"$HOME"/}"; then
    journal D "$1"
    rmdir "$1"
  fi
}

# --------------------------------------------------------------- the steps ----

stop_old_service() {
  systemctl --user is-active --quiet "$OLD_SERVICE" 2>/dev/null || return 0
  if plan "stop $OLD_SERVICE (Vetro's launch addon starts its own)"; then
    journal S "$OLD_SERVICE"
    systemctl --user stop "$OLD_SERVICE" || warn "could not stop $OLD_SERVICE"
  fi
}

# Hyprland's file: the one line that loaded liftoff's Lua, and meniscus's block,
# become Vetro's block, in one write. With both in the file every key would be
# set twice, and with the old one alone, to a file that is gone.
migrate_hyprland() {
  local file=$HYPR_CONFIG new
  [ -f "$file" ] || return 0
  grep -q -e 'hypr\.liftoff-look' -e '^-- >>> meniscus' "$file" || return 0
  if ! plan "hyprland.lua: the old lines become Vetro's block"; then return 0; fi
  keep "$file"
  new=$(mktemp)
  awk -v b="$HYPR_BEGIN" -v l="$HYPR_LINE" -v e="$HYPR_END" '
    function block() { if (!placed) { print b; print l; print e; placed = 1 } }
    { lines[NR] = $0; if ($0 == b) placed = 1 }
    END {
      for (i = 1; i <= NR; i++) {
        s = lines[i]
        if (s ~ /^-- liftoff: its look/ && lines[i + 1] ~ /require\("hypr\.liftoff-look"\)/) { block(); i++ }
        else if (s ~ /require\("hypr\.liftoff-look"\)/) block()
        else if (s ~ /^-- >>> meniscus/) { block(); while (i < NR && lines[i] !~ /^-- <<< meniscus/) i++ }
        else print s
      }
    }' "$file" >"$new"
  cat "$new" >"$file"
  rm -f "$new"
}

# The key bound to the old command in the user's own bindings.
migrate_bindings() {
  local file=$CONFIG/hypr/bindings.lua
  [ -f "$file" ] && ! ours "$file" || return 0
  grep -q -e 'liftoff-look' -e 'meniscus glass' "$file" || return 0
  if plan "bindings.lua: the key for the old command runs vetro"; then
    keep "$file"
    sed -i --follow-symlinks \
      -e 's/^-- liftoff: its look off/-- Vetro: off/' \
      -e 's/Toggle liftoff look/Toggle Vetro/' \
      -e 's/liftoff-look/vetro/g' \
      -e 's/meniscus glass/vetro glass/g' "$file"
  fi
}

# rename_marks FILE: the old marks of the two blocks become the new ones, where
# they stand. The text between them is laid again by `vetro on`.
rename_marks() {
  local file=$1 new
  new=$(mktemp)
  awk -v lb="$LOOK_BEGIN" -v le="$LOOK_END" -v gb="$GLASS_BEGIN" -v ge="$GLASS_END" \
    -v olb="$OLD_LOOK_BEGIN" -v ole="$OLD_LOOK_END" -v ogb="$OLD_GLASS_BEGIN" -v oge="$OLD_GLASS_END" '
    index($0, olb) == 1 { print lb; next }
    index($0, ole) == 1 { print le; next }
    index($0, ogb) == 1 { print gb; next }
    index($0, oge) == 1 { print ge; next }
    { print }' "$file" >"$new"
  cat "$new" >"$file"
  rm -f "$new"
}

migrate_blocks() {
  local rel file
  for rel in "${OLD_BLOCK_FILES[@]}"; do
    file=$HOME/$rel
    [ -f "$file" ] || continue
    grep -q -e "^$OLD_LOOK_BEGIN" -e "^$OLD_GLASS_BEGIN" "$file" || continue
    if plan "$rel: the marks of the old blocks become Vetro's"; then
      keep "$file"
      rename_marks "$file"
    fi
  done
}

# was_laid: whether the old look was switched on. Its bar block is in
# shell.toml only then, and the flag file it leaves when switched off is not.
was_laid() {
  [ -f "$SHELL_TOML" ] && grep -q "^$OLD_LOOK_BEGIN" "$SHELL_TOML" && [ ! -e "$OLD_STATE/look-off" ]
}

# The plugin ids in the shell's own file, renamed where they stand. The windows
# button and the old bar plugin go; passthru has its own button.
# shellcheck disable=SC2016  # a jq program: its $ names are jq's, not the shell's
RENAME_IDS='
  def newid: if . == "liftoff" or . == "liftoff.windows-button" then null
             elif startswith("liftoff.") then "vetro." + .[8:] else . end;
  def fix: if type == "object" and (.id | type) == "string"
             then (.id | newid) as $n | if $n == null then empty else .id = $n end
           elif type == "string" then (newid // empty)
           else . end;
  (if (.plugins | type) == "array" then .plugins |= map(fix) else . end)
  | (if (.cloneSourceRestores | type) == "array" then .cloneSourceRestores |= map(fix) else . end)
  | (if (.bar.layout | type) == "object"
       then reduce ("left", "center", "right") as $s (.;
              if (.bar.layout[$s] | type) == "array" then .bar.layout[$s] |= map(fix) else . end)
       else . end)'

migrate_shell_json() {
  local file=$CONFIG/omarchy/shell.json new
  [ -f "$file" ] && grep -q '"liftoff' "$file" || return 0
  if ! plan "omarchy/shell.json: the plugin ids liftoff.* become vetro.*, in place"; then return 0; fi
  keep "$file"
  new=$(mktemp "$(dirname "$file")/.shell.json.XXXXXX")
  if jq -S "$RENAME_IDS" "$file" >"$new"; then
    chmod --reference="$file" "$new"
    mv -f "$new" "$file"
    omarchy-shell shell reloadConfig >/dev/null 2>&1 || omarchy-shell -q shell rescanPlugins >/dev/null 2>&1 || true
  else
    rm -f "$new"
    warn "shell.json was not changed: jq could not read it"
  fi
}

migrate_links() {
  local rel
  for rel in "${OLD_LINKS[@]}"; do
    drop_link "$HOME/$rel"
  done
  tidy "$CONFIG/systemd/user/liftoff-launchd.service.d"
}

# fix_clip_paths: each clip's notes (and current.json, which names the clip in
# use) hold the full path of the clip and its still, under the old folder. The
# daemon and the lock screen go by them, so they follow the clips across.
fix_clip_paths() {
  local clips=$XDG_DATA/vetro/clips file
  [ "$MODE" != dry ] && [ -d "$clips" ] || return 0
  for file in "$clips"/*.json; do
    if [ ! -f "$file" ] || ! grep -qF "$OLD_SHARE/clips" "$file"; then continue; fi
    echo "  ${file#"$HOME"/}: the clip's path is the new one"
    keep "$file"
    sed -i "s|$OLD_SHARE/clips|$clips|g" "$file"
  done
}

# The state, the cache, the clips and the built engine go to Vetro's folders;
# what was only there to remember a theme goes.
migrate_data() {
  local name file
  for name in state.json misc.json account.json notifications.json; do
    take "$OLD_STATE/$name" "$STATE/$name"
  done
  take "$OLD_STATE/look-off" "$FLAG"
  discard "$OLD_STATE/look-previous-theme"
  discard "$OLD_MENISCUS_STATE/previous-theme"
  take "$OLD_MENISCUS_STATE/build.log" "$STATE/build.log"
  take "$OLD_CACHE/launches.json" "$XDG_CACHE/vetro/launches.json"
  take "$OLD_SHARE/clips" "$XDG_DATA/vetro/clips"
  fix_clip_paths
  for file in "$OLD_LIB"/*; do
    if [ -e "$file" ]; then take "$file" "$ENGINE_DIR/$(basename "$file")"; fi
  done
  # The source and objects of the engine are rebuilt from this repository.
  if [ -d "$OLD_MENISCUS_CACHE" ] && plan "remove ${OLD_MENISCUS_CACHE#"$HOME"/} (the engine's old build folder; it is rebuilt)"; then
    rm -rf "$OLD_MENISCUS_CACHE"
  fi
  if [ -n "${XDG_RUNTIME_DIR:-}" ]; then
    for file in liftoff-glass-stood-down liftoff-glass-readings meniscus-stood-down meniscus-readings meniscus-material; do
      if [ -e "$XDG_RUNTIME_DIR/$file" ] && plan "remove $XDG_RUNTIME_DIR/$file"; then
        rm -f "$XDG_RUNTIME_DIR/$file"
      fi
    done
    # The old daemon's sockets, once it is stopped.
    if [ -d "$XDG_RUNTIME_DIR/liftoff" ] && ! systemctl --user is-active --quiet "$OLD_SERVICE" 2>/dev/null &&
      plan "remove $XDG_RUNTIME_DIR/liftoff"; then
      rm -rf "$XDG_RUNTIME_DIR/liftoff"
    fi
  fi
  for file in "$OLD_STATE" "$OLD_MENISCUS_STATE" "$OLD_CACHE" "$OLD_SHARE" "$OLD_LIB"; do
    tidy "$file"
  done
}

# The theme liftoff had is called launch now (addons/theme). Where a file holds
# the old name, the same theme is called by its new one: the theme Omarchy has
# as current, and the one Aether remembers as the previous. Nothing is set;
# a theme the user did not choose is not touched.
THEME_NAME_FILES=("$HOME/.local/state/omarchy/current/theme.name" "$CONFIG/aether/previous-omarchy-theme")

migrate_theme_names() {
  local file
  for file in "${THEME_NAME_FILES[@]}"; do
    if [ ! -f "$file" ] || [ "$(tr -d '[:space:]' <"$file")" != liftoff ]; then continue; fi
    if plan "${file#"$HOME"/}: the theme liftoff is called launch now"; then
      keep "$file"
      printf 'launch\n' >"$file"
    fi
  done
}

# The root half of the earlier install is not ours to touch.
mention_system() {
  if [ -e "${SYSROOT:-}/etc/sddm.conf.d/99-zz-liftoff.conf" ]; then
    echo "  the login screen still has its old names: run addons/login/install-system.sh as root"
  fi
}

# ------------------------------------------------------------- check, undo ----

# leftovers: one line for each thing of the earlier projects still here.
leftovers() {
  local rel file n=0 target
  for rel in "${OLD_LINKS[@]}"; do
    if [ -L "$HOME/$rel" ]; then
      target=$(readlink "$HOME/$rel")
      case $target in *liftoff* | *meniscus*)
        echo "left: the link $rel"
        n=$((n + 1))
        ;;
      esac
    fi
  done
  for rel in "${OLD_BLOCK_FILES[@]}"; do
    file=$HOME/$rel
    if [ -f "$file" ] && grep -q -e "^$OLD_LOOK_BEGIN" -e "^$OLD_GLASS_BEGIN" "$file"; then
      echo "left: an old block in $rel"
      n=$((n + 1))
    fi
  done
  for file in "$HYPR_CONFIG" "$CONFIG/hypr/bindings.lua"; do
    if [ -f "$file" ] && grep -q -e 'liftoff-look' -e '^-- >>> meniscus' -e 'meniscus glass' "$file"; then
      echo "left: an old command or block in ${file#"$HOME"/}"
      n=$((n + 1))
    fi
  done
  if [ -f "$CONFIG/omarchy/shell.json" ] && grep -q '"liftoff' "$CONFIG/omarchy/shell.json"; then
    echo "left: plugin ids liftoff.* in omarchy/shell.json"
    n=$((n + 1))
  fi
  for file in "${THEME_NAME_FILES[@]}"; do
    if [ -f "$file" ] && [ "$(tr -d '[:space:]' <"$file")" = liftoff ]; then
      echo "left: the theme name liftoff in ${file#"$HOME"/}"
      n=$((n + 1))
    fi
  done
  for file in "$OLD_STATE/state.json" "$OLD_STATE/look-off" "$OLD_LIB/hyprglass.so" "$OLD_CACHE/launches.json" "$OLD_SHARE/clips"; do
    if [ -e "$file" ]; then
      echo "left: ${file#"$HOME"/}"
      n=$((n + 1))
    fi
  done
  [ "$n" = 0 ]
}

# undo: Vetro out, and the last record put back.
undo() {
  local last='' op a b dir
  for dir in "$RECORDS"/*; do
    case $dir in *.undone) continue ;; esac
    if [ -d "$dir" ]; then last=$(basename "$dir"); fi
  done
  [ -n "$last" ] || die "there is no migration to undo (nothing in $RECORDS)"
  if [ -e "$HOME/.local/bin/vetro" ] || [ -L "$HOME/.local/bin/vetro" ]; then
    echo "Taking Vetro out first"
    "$ROOT/uninstall.sh"
  fi
  echo "Putting back what the migration of ${last%-*} changed"
  while IFS=$'\t' read -r op a b; do
    case $op in
      L) rm -f "$a"; mkdir -p "$(dirname "$a")"; ln -s "$b" "$a"; echo "  link ${a#"$HOME"/}" ;;
      F) mkdir -p "$(dirname "$a")"; cat "$b" >"$a"; echo "  restored ${a#"$HOME"/}" ;;
      M) if [ -e "$b" ] && [ ! -e "$a" ]; then mkdir -p "$(dirname "$a")"; mv "$b" "$a"; echo "  moved back ${a#"$HOME"/}"; fi ;;
      D) mkdir -p "$a" ;;
      S) systemctl --user daemon-reload 2>/dev/null || true; systemctl --user start "$a" 2>/dev/null || true; echo "  started $a" ;;
    esac
  done < <(tac "$RECORDS/$last/journal")
  systemctl --user daemon-reload 2>/dev/null || true
  # Folders that Vetro made, and that nothing is in now.
  prune "$STATE" "$ENGINE_DIR" "$XDG_CACHE/vetro" "$XDG_DATA/vetro"
  mv "$RECORDS/$last" "$RECORDS/$last.undone"
  omarchy-shell shell reloadConfig >/dev/null 2>&1 || true
  echo "Done. Restart the shell (omarchy-restart-shell) and reload Hyprland (hyprctl reload) to see it."
}

# ------------------------------------------------------------------- main ----

run() {
  if leftovers >/dev/null; then
    echo "Nothing of liftoff or meniscus is left in this home folder: nothing to take over."
    return 0
  fi
  if [ "$MODE" = dry ]; then
    echo "Dry run: nothing is changed."
  elif [ ! -e "$PLUGINS_DIR/vetro.menu" ]; then
    # The plugin ids in shell.json would name plugins that are not there.
    die "Vetro's own plugins are not linked yet: run $ROOT/install.sh, which links them and then runs this"
  fi
  if was_laid && [ "$MODE" != dry ]; then
    mkdir -p "$STATE"
    : >"$STATE/resume-on"
  fi
  stop_old_service
  migrate_hyprland
  migrate_bindings
  migrate_blocks
  migrate_shell_json
  migrate_links
  migrate_theme_names
  migrate_data
  if [ "$MODE" != dry ]; then
    systemctl --user daemon-reload 2>/dev/null || true
  fi
  mention_system
  if [ "$MODE" != dry ] && [ -n "$RUN" ]; then
    echo "The record of what was changed is $RUN (migrate.sh --undo puts it back)."
  fi
}

case $MODE in
  check)
    if leftovers; then
      echo "Nothing of liftoff or meniscus is left."
    else
      exit 1
    fi
    ;;
  undo) undo ;;
  *) run ;;
esac
