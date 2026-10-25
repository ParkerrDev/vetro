#!/bin/bash
# run.sh: proof that a build draws the same glass as another, and costs no more.
#
#   run.sh pictures OLD.so NEW.so [SCENE ...]   the same scenes through both builds, pixel by pixel
#   run.sh cost OLD.so NEW.so [ROUNDS [SECONDS]]  the benchmark scene's GPU time per frame, alternating
#   run.sh inspect OLD NEW [SCENE]               what each build decided for a scene (hyprctl hyprglass model and panes)
#   run.sh start | stop                          the hidden nested compositor by hand
#
# Each build is loaded into a nested compositor of its own on a hidden workspace,
# started for the length of the run and stopped after it (a run takes well under
# a minute), and nothing is ever loaded into the live compositor. The nest
# draws on the same GPU the session does, so runs are short and few.
#
# REGRESS_WORK is where the scenes, backgrounds and pictures go
# (default ~/.cache/hyprglass-regress). The pictures never leave the machine.
# REGRESS_PRESET names the preset the layers get (default regular; pomme, glass and
# high_contrast run the classic shader, which takes every option through the model too).
set -u
here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=/dev/null
. "$here/nest.sh"

SCENES_DEFAULT="desk desk-light full dimmed"
NAMESPACES="lab:bar lab:launch lab:weather lab:notif lab:menu lab:full"

scene_value() { python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1])).get(sys.argv[2], sys.argv[3]))' "$W/scenes/$1.json" "$2" "$3"; }

# Photographs one scene through the builds one after the other in ONE nested
# compositor, so that the scene's own surfaces, drawn once by their clients, are
# exactly the same under every build. The first build is photographed again at
# the end: the two photographs of it say how much the setup itself moves.
shoot() {
  local scene="$1" old="$2" new="$3"
  local theme=dark; case "$scene" in *light*) theme=light ;; esac
  nest_start || return 1
  mkdir -p "$W/out"
  nest_glass "$theme" "${REGRESS_PRESET:-regular}" "$NAMESPACES" "$(scene_value "$scene" threshold 0.04)" || { nest_stop; return 1; }
  nest_scene "$scene" || { nest_stop; return 1; }
  sleep 1.0
  local name so
  for pair in "old=$old" "new=$new" "again=$old"; do
    name="${pair%%=*}"; so="${pair#*=}"
    nest_load "$so" || { nest_stop; return 1; }
    sleep 2.2
    nest_shot "$W/out/$name-$scene.png" || { nest_stop; return 1; }
    nest_unload "$so"
  done
  nest_stop
}

pictures() {
  local old="$1" new="$2"; shift 2
  local scenes=("$@"); [ ${#scenes[@]} -gt 0 ] || read -r -a scenes <<< "$SCENES_DEFAULT"
  nest_prepare
  local scene status=0
  for scene in "${scenes[@]}"; do
    shoot "$scene" "$old" "$new" || return 1
    echo "scene $scene"
    echo " the old build against itself, photographed again after the new one:"
    python3 -I "$here/diff.py" "$W/out/old-$scene.png" "$W/out/again-$scene.png" "$W/scenes/$scene.json" 1.6 | head -1
    echo " the new build against the old one:"
    python3 -I "$here/diff.py" "$W/out/old-$scene.png" "$W/out/new-$scene.png" "$W/scenes/$scene.json" 1.6 "$W/out/diff-$scene.png" || status=1
  done
  [ $status = 0 ] && echo "SAME: no pixel differs by more than one level" || echo "DIFFERENT: see above"
  return $status
}

# Per-frame GPU time of the nest for SECONDS: the kernel's account of the render engine, and the plugin's own stage timers.
sample() {
  local secs="$1" n0 n1
  nctl hyprglass stats reset >/dev/null
  n0=$(nest_gpu_ns); sleep "$secs"; n1=$(nest_gpu_ns)
  nctl -j hyprglass stats | NS=$((n1 - n0)) python3 -I -c '
import json, os, sys
m = json.load(sys.stdin)["monitors"][0]
frames = max(m["frames"], 1)
t = m["stageTimersAvgMicroseconds"]
model = sum(t.get(k, 0) for k in ("model_field", "model_sample", "model_glass"))
print("%d %.1f %.1f" % (m["frames"], int(os.environ["NS"]) / 1000.0 / frames, model))'
}

# One nest, the builds one after the other in the order given. Each is measured
# twice on the benchmark scene: with its glass switched off for every layer
# (what the compositor costs with the plugin loaded and nothing to draw) and
# with it on. Prints "label frames kernel_us_per_frame stage_us" for both.
cost_round() {
  local secs="$1"; shift
  nest_start >/dev/null || return 1
  nest_glass dark regular "" 0.04 true >/dev/null || { nest_stop >/dev/null; return 1; }
  nest_scene desk-live || { nest_stop >/dev/null; return 1; }
  local pair label so
  for pair in "$@"; do
    label="${pair%%=*}"; so="${pair#*=}"
    nest_load "$so" >/dev/null || { nest_stop >/dev/null; return 1; }
    nest_glass dark regular "" 0.04 true >/dev/null
    sleep 1.5
    echo "none-$label $(sample "$secs")"
    nest_glass dark regular "$NAMESPACES" 0.04 true >/dev/null
    sleep 1.5
    echo "$label $(sample "$secs")"
    nest_unload "$so"
  done
  nest_stop >/dev/null
}

cost() {
  local old="$1" new="$2" rounds="${3:-4}" secs="${4:-5}"
  nest_prepare
  local rows="$W/cost.txt"; : > "$rows"
  local r
  for r in $(seq 1 "$rounds"); do
    if [ $((r % 2)) = 1 ]; then cost_round "$secs" "old=$old" "new=$new" >> "$rows"; else cost_round "$secs" "new=$new" "old=$old" >> "$rows"; fi
  done
  python3 -I - "$rows" <<'PY'
import statistics, sys
rows = {}
for line in open(sys.argv[1]):
    label, frames, kernel, stage = line.split()
    rows.setdefault(label, []).append((float(kernel), float(stage), int(frames)))
print("benchmark scene: a bar, two cards, a notification and a menu over a wallpaper that moves every frame")
for label in ("old", "new"):
    r, n = rows.get(label, []), rows.get("none-" + label, [])
    if not r:
        continue
    k = [x[0] for x in r]; s = [x[1] for x in r]; base = statistics.median(x[0] for x in n)
    print("  %-3s kernel us/frame, whole compositor: median %6.0f  (runs %s); with the glass off %6.0f; the glass adds %+.0f" % (
        label, statistics.median(k), " ".join("%.0f" % x for x in k), base, statistics.median(k) - base))
    print("      plugin's own stage timers (field + sample + glass), us: median %6.1f  (runs %s)" % (statistics.median(s), " ".join("%.0f" % x for x in s)))
PY
}

# What a build resolved its preset to and which panes it found, as `hyprctl hyprglass` prints them.
decisions() {
  local name="$1" so="$2" scene="$3"
  nest_start || return 1
  nest_load "$so" || { nest_stop; return 1; }
  nest_glass dark regular "$NAMESPACES" "$(scene_value "$scene" threshold 0.04)" || { nest_stop; return 1; }
  nest_scene "$scene" || { nest_stop; return 1; }
  sleep 2.5
  mkdir -p "$W/out"
  { nctl hyprglass model regular dark; nctl hyprglass model clear light; nctl hyprglass panes; } > "$W/out/$name-decisions.txt" 2>&1
  nest_unload "$so"
  nest_stop
}

inspect() {
  local old="$1" new="$2" scene="${3:-desk}"
  nest_prepare
  decisions old "$old" "$scene" || return 1
  decisions new "$new" "$scene" || return 1
  if diff <(sort "$W/out/old-decisions.txt") <(sort "$W/out/new-decisions.txt"); then echo "SAME: the two builds resolved and found exactly the same"; else echo "DIFFERENT: see above"; return 1; fi
}

case "${1:-}" in
  inspect) shift; inspect "$@" ;;
  pictures) shift; pictures "$@" ;;
  cost) shift; cost "$@" ;;
  start) nest_start ;;
  stop) nest_read 2>/dev/null; nest_stop ;;
  *) sed -n '2,15p' "$0"; exit 2 ;;
esac
