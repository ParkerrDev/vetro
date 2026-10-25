# nest.sh: a hidden nested compositor for the regression and cost scripts.
#
# Sourced by run.sh. The live compositor is never addressed except by the
# read-only queries of `hyprctl instances` and `hyprctl clients`, and by the one
# window rule that puts the nest's window on a hidden workspace. The nest is
# known by one thing only: the process started with this work directory's own
# config file. No plugin is ever loaded from a config: `nest_load` loads it
# from here, into the nest only.

W="${REGRESS_WORK:-${XDG_CACHE_HOME:-$HOME/.cache}/hyprglass-regress}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NEST_CONFIG="$W/lab.lua"
NEST_FILE="$W/nest"

# The live compositor: the instance this shell belongs to.
live_pid() {
  hyprctl instances -j | SIG="${HYPRLAND_INSTANCE_SIGNATURE:-}" python3 -I -c '
import json, os, sys
for i in json.load(sys.stdin):
    if i["instance"] == os.environ["SIG"]:
        print(i["pid"])'
}

# True when PID is a Hyprland started with this work directory's config.
is_mine() {
  [ -n "$1" ] && [ "$1" != "$LIVE_PID" ] && [ -r "/proc/$1/cmdline" ] || return 1
  [ "$(tr '\0' ' ' < "/proc/$1/cmdline")" = "Hyprland -c $NEST_CONFIG " ]
}

# Reads the nest's signature, PID and socket, and refuses anything that is not this nest.
nest_read() {
  LIVE_PID="${LIVE_PID:-$(live_pid)}"
  [ -n "$LIVE_PID" ] || { echo "cannot tell which compositor is the live one" >&2; return 1; }
  [ -f "$NEST_FILE" ] || { echo "no nested compositor on record" >&2; return 1; }
  { read -r SIG; read -r NP; read -r WL; } < "$NEST_FILE"
  [ -n "$SIG" ] && [ -n "$NP" ] && [ -n "$WL" ] || { echo "nest file incomplete" >&2; return 1; }
  [ "$NP" != "$LIVE_PID" ] && [ "$SIG" != "${HYPRLAND_INSTANCE_SIGNATURE:-}" ] && [ "$WL" != "${WAYLAND_DISPLAY:-wayland-1}" ] || { echo "REFUSING: the nest file names the live compositor" >&2; return 1; }
  is_mine "$NP" || { echo "process $NP is not this nest (gone, or someone else's)" >&2; return 1; }
  [ -S "$XDG_RUNTIME_DIR/$WL" ] || { echo "nested socket $WL is missing" >&2; return 1; }
}

nctl() { hyprctl -i "$SIG" "$@"; }

# Puts the work directory together: the configs, the scenes and the backgrounds.
nest_prepare() {
  mkdir -p "$W/bg" "$W/scenes"
  sed "s|@WORK@|$W|g" "$HERE/lab.lua" > "$NEST_CONFIG"
  cp "$HERE/lab.qml" "$W/lab.qml"
  [ -f "$W/bg/bg-busy.png" ] && [ -f "$W/bg/bg-light.png" ] || python3 -I "$HERE/backgrounds.py" "$W/bg"
  python3 -I "$HERE/scenes.py" "$W/scenes"
  [ -f "$W/mode.txt" ] || echo off > "$W/mode.txt"
}

# Opens the nested compositor on a hidden workspace and records it.
nest_start() {
  nest_prepare
  LIVE_PID="$(live_pid)"
  if nest_read 2>/dev/null; then echo "nest already running: pid $NP"; return 0; fi
  rm -f "$NEST_FILE"
  # the one command against the live compositor, once for as long as it runs: where to put the nest's window
  if [ ! -f "$W/window-rule-$LIVE_PID" ]; then
    hyprctl eval 'hl.window_rule({ match = { class = "^aquamarine$" }, workspace = "special:glasslab silent", render_unfocused = true })' >/dev/null
    touch "$W/window-rule-$LIVE_PID"
  fi
  (setsid Hyprland -c "$NEST_CONFIG" > "$W/hypr.log" 2>&1 &)
  local found=""
  for _ in $(seq 1 60); do
    sleep 0.25
    found=$(hyprctl instances -j | LIVE_PID="$LIVE_PID" NEST_CONFIG="$NEST_CONFIG" python3 -I -c '
import json, os, sys
want = "Hyprland -c " + os.environ["NEST_CONFIG"] + " "
for i in json.load(sys.stdin):
    if str(i["pid"]) == os.environ["LIVE_PID"]:
        continue
    try:
        cmd = open("/proc/%d/cmdline" % i["pid"]).read().replace("\0", " ")
    except OSError:
        continue
    if cmd == want:
        print(i["instance"], i["pid"], i["wl_socket"])')
    [ -n "$found" ] && break
  done
  [ -n "$found" ] || { echo "the nested compositor did not appear; see $W/hypr.log" >&2; return 1; }
  [ "$(echo "$found" | wc -l)" = 1 ] || { echo "more than one compositor runs this config: $found" >&2; return 1; }
  set -- $found
  printf '%s\n%s\n%s\n' "$1" "$2" "$3" > "$NEST_FILE"
  nest_read || return 1
  # its window must sit on the hidden workspace; if it does not, close it at once
  local where=""
  for _ in $(seq 1 40); do
    where=$(hyprctl clients -j | NP="$NP" python3 -I -c '
import json, os, sys
for c in json.load(sys.stdin):
    if c["class"] == "aquamarine" and str(c["pid"]) == os.environ["NP"]: print(c["workspace"]["name"])')
    [ -n "$where" ] && break
    sleep 0.25
  done
  case "$where" in
    "special:glasslab") echo "nest: pid $NP on $where" ;;
    *) echo "the nest's window is on '$where', not special:glasslab: closing it" >&2; kill -TERM "$NP"; rm -f "$NEST_FILE"; return 1 ;;
  esac
}

# Closes the nest's clients and the nest, each by its own PID.
nest_stop() {
  LIVE_PID="${LIVE_PID:-$(live_pid)}"
  if [ -f "$W/pid.qs" ]; then
    local p; p=$(cat "$W/pid.qs")
    if [ -n "$p" ] && [ "$p" != "$LIVE_PID" ] && grep -qa "$W/lab.qml" "/proc/$p/cmdline" 2>/dev/null; then kill -TERM "$p" 2>/dev/null; fi
    rm -f "$W/pid.qs"
  fi
  if nest_read 2>/dev/null; then
    kill -TERM "$NP"
    for _ in $(seq 1 25); do kill -0 "$NP" 2>/dev/null || break; sleep 0.2; done
    kill -0 "$NP" 2>/dev/null && echo "nest $NP is still alive" >&2
  fi
  rm -f "$NEST_FILE"
  echo "nest stopped"
}

# The host can put the nested output back to its tile's size at any moment;
# a reload applies the 2560x1600 mode again.
nest_mode() {
  local size
  for _ in 1 2 3; do
    size=$(nctl monitors -j | python3 -I -c 'import json,sys; m=json.load(sys.stdin)[0]; print(m["width"], m["height"])')
    [ "$size" = "2560 1600" ] && return 0
    nctl reload >/dev/null; sleep 1.2
  done
  echo "the nested output is $size, not 2560 1600" >&2; return 1
}

# Loads a built plugin into the nest only, under its own file name.
nest_load() {
  nest_read || return 1
  [ -f "$1" ] || { echo "no such file $1" >&2; return 1; }
  case "$1" in "$HOME/.local/lib/"*) echo "REFUSING a file under ~/.local/lib" >&2; return 1 ;; esac
  nctl plugin load "$1" >/dev/null || return 1
  sleep 0.6
  nctl reload >/dev/null; sleep 1.0
  nctl configerrors | grep -v "^$" | head -5
  return 0
}

nest_unload() {
  nest_read || return 1
  nctl plugin unload "$1" >/dev/null
  sleep 0.4
}

# Sets what the nest's config says about glass: THEME, PRESET, the namespaces that get it, the mask threshold.
nest_glass() {
  cat > "$W/glass-config.lua" <<LUA
return { theme = "$1", preset = "$2", only = "$3", threshold = ${4:-0.04}, timers = ${5:-false} }
LUA
  nctl reload >/dev/null; sleep 0.8
  nest_mode
}

# Shows a scene: the quickshell that draws the scenes watches one file.
nest_scene() {
  python3 -I - "$W/scenes/$1.json" "$W/current.json" <<'PY'
import json, sys
json.dump(json.load(open(sys.argv[1])), open(sys.argv[2], "w"))
PY
  local running=""
  if [ -f "$W/pid.qs" ]; then
    local p; p=$(cat "$W/pid.qs")
    if [ -n "$p" ] && grep -qa "$W/lab.qml" "/proc/$p/cmdline" 2>/dev/null && grep -qa "WAYLAND_DISPLAY=$WL" "/proc/$p/environ" 2>/dev/null; then running=1; fi
  fi
  if [ -z "$running" ]; then
    WAYLAND_DISPLAY="$WL" HYPRLAND_INSTANCE_SIGNATURE="$SIG" LAB_SCENE="$W/current.json" LAB_DIR="$W" \
      setsid quickshell -p "$W/lab.qml" > "$W/qs.log" 2>&1 &
    echo $! > "$W/pid.qs"
    sleep 1.5
    kill -0 "$(cat "$W/pid.qs")" 2>/dev/null || { echo "quickshell exited; see $W/qs.log" >&2; return 1; }
  fi
}

# Photographs the nest only. A hidden window the host has stopped sending frames to never answers.
nest_shot() {
  nest_mode || return 1
  WAYLAND_DISPLAY="$WL" timeout 8 grim "$1"
  local rc=$?
  [ $rc = 124 ] && echo "no frame from the nest in 8 s: the host has stopped driving it" >&2
  return $rc
}

# GPU time the nest's GL context has used, nanoseconds, from the kernel's own accounting.
nest_gpu_ns() {
  local best=0 v fd
  for fd in /proc/"$NP"/fdinfo/*; do
    v=$(awk '/^drm-engine-render:/ {print $2}' "$fd" 2>/dev/null)
    [ -n "$v" ] && [ "$v" -gt "$best" ] && best=$v
  done
  echo "$best"
}
