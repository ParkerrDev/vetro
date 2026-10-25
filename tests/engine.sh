#!/bin/bash
# The engine's build script and its pins: the pins file is well formed and names
# commits that this repository has, with an engine in them; and the script finds
# the Hyprland that is installed, fetches the pinned commit from this repository
# and not from a remote, builds it (a stand-in make does that here), puts the
# result where the command looks, refuses a Hyprland it has no pin for, never
# writes over a build with a broken one, and builds nothing twice.
# shellcheck source-path=SCRIPTDIR
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"
make_bin

# --- the pins file, as shipped
lines=0
while read -r hyprland engine folder _; do
  case $hyprland in "" | "#"*) continue ;; esac
  lines=$((lines + 1))
  [[ $hyprland =~ ^[0-9a-f]{40}$ ]] || fail "pins: '$hyprland' is not a commit of Hyprland's"
  [[ $engine =~ ^[0-9a-f]{40}$ ]] || fail "pins: '$engine' is not a commit"
  case ${folder:-.} in "#"*) folder=. ;; esac
  ok "pins: $engine is in this repository" git -C "$REPO" cat-file -e "$engine^{commit}"
  ok "pins: $engine has an engine Makefile in ${folder:-.}" git -C "$REPO" cat-file -e "$engine:${folder:-.}/Makefile"
  ok "pins: $engine has the engine's main source" git -C "$REPO" cat-file -e "$engine:${folder:-.}/src/main.cpp"
  real_folder=${folder:-.}
done <"$REPO/engine/pins"
is "pins: there is a line for the Hyprland in use" 1 "$((lines > 0))"

# --- the real script, the real pin, a stand-in make: the pinned commit comes
# out of this repository and is built where its Makefile is
new_home real
export GIT_CONFIG_NOSYSTEM=1
run "$REPO/engine/build.sh" >"$WORK/real.out" 2>&1
is "the real pin builds" 0 "$?"
has "it built" "$WORK/real.out" "Built $H/.local/lib/vetro/hyprglass.so"
has "make ran in the engine's folder, with its Makefile and main source" "$H/.standin/log" "make in $H/.cache/vetro/engine/${real_folder:-.} (Makefile) (src/main.cpp)"
is "the stamp names the Hyprland" "$PINNED_HYPRLAND" "$(sed -n 1p "$H/.local/lib/vetro/hyprglass.built-for")"
is "the stamp names the pin" "$PINNED_ENGINE" "$(sed -n 3p "$H/.local/lib/vetro/hyprglass.built-for")"
is "the clone is of this repository" "$REPO" "$(git -C "$H/.cache/vetro/engine" remote get-url origin)"
is "and at the pinned commit" "$PINNED_ENGINE" "$(git -C "$H/.cache/vetro/engine" rev-parse HEAD)"
forget_calls
run "$REPO/engine/build.sh" >"$WORK/real2.out" 2>&1
has "a second run builds nothing" "$WORK/real2.out" "already built"
if called "make in"; then fail "a second run ran make"; fi
run "$REPO/engine/build.sh" --check >/dev/null 2>&1
is "--check says it is built" 0 "$?"

# --- a repository of its own, with an engine in two layouts, to try the rest
S=$WORK/repo
mkdir -p "$S/engine"
cp "$REPO/engine/build.sh" "$S/engine/build.sh"
git -c init.defaultBranch=master init -q "$S"
g() { git -C "$S" -c commit.gpgsign=false -c user.name=t -c user.email=t@t "$@"; }
mkdir -p "$S/src"
echo "all:" >"$S/Makefile"
echo "int main() {}" >"$S/src/main.cpp"
g add Makefile src && g commit -q -m "the engine, as the whole tree"
first=$(g rev-parse HEAD)
g rm -q -r Makefile src
mkdir -p "$S/engine/src"
echo "all:" >"$S/engine/Makefile"
echo "int main() {}" >"$S/engine/src/main.cpp"
g add engine/Makefile engine/src && g commit -q -m "the engine, in its folder"
second=$(g rev-parse HEAD)
hy1=1111111111111111111111111111111111111111
hy2=2222222222222222222222222222222222222222
hy3=3333333333333333333333333333333333333333
{
  echo "# pins for the test"
  echo "$hy1 $first                              # the engine as the whole tree"
  echo "$hy2 $second engine                      # and in its folder"

} >"$S/engine/pins"
chmod +x "$S/engine/build.sh"

# built() HYPRLAND: the script in the repository of its own, for a Hyprland.
built() {
  printf '#define GIT_COMMIT_HASH "%s"\n#define GIT_TAG "vtest"\n' "$1" >"$H/include/hyprland/src/version.h"
  run "$S/engine/build.sh" "${@:2}"
}

new_home scratch
built "$hy1" >"$WORK/b1.out" 2>&1
is "a pin for the engine as the whole tree builds" 0 "$?"
has "it built at the root" "$H/.standin/log" "make in $H/.cache/vetro/engine/. (Makefile) (src/main.cpp)"
is "the stamp: Hyprland" "$hy1" "$(sed -n 1p "$H/.local/lib/vetro/hyprglass.built-for")"
is "the stamp: tag" vtest "$(sed -n 2p "$H/.local/lib/vetro/hyprglass.built-for")"
is "the stamp: pin" "$first" "$(sed -n 3p "$H/.local/lib/vetro/hyprglass.built-for")"
is "the engine is the one that was built" "engine built in $H/.cache/vetro/engine/." "$(cat "$H/.local/lib/vetro/hyprglass.so")"
forget_calls
built "$hy1" >"$WORK/b2.out" 2>&1
has "a second run is already built" "$WORK/b2.out" "already built"
if called "make in"; then fail "built again for nothing"; fi
built "$hy1" --force >"$WORK/b3.out" 2>&1
called "make in" || fail "--force did not build"
forget_calls
built "$hy2" >"$WORK/b4.out" 2>&1
is "a pin for the engine in its folder builds" 0 "$?"
has "it built in the engine's folder" "$H/.standin/log" "make in $H/.cache/vetro/engine/engine (Makefile) (src/main.cpp)"
is "the stamp follows" "$second" "$(sed -n 3p "$H/.local/lib/vetro/hyprglass.built-for")"

# a Hyprland with no pin: refused, and the engine already there is left alone
before=$(md5sum "$H/.local/lib/vetro/hyprglass.so")
built "$hy3" >"$WORK/b5.out" 2>&1
is "a Hyprland with no pin is refused" 3 "$?"
has "and the reason is given" "$WORK/b5.out" "no engine is pinned for Hyprland vtest"
is "and the engine already built is left alone" "$before" "$(md5sum "$H/.local/lib/vetro/hyprglass.so")"
built "$hy3" --check >/dev/null 2>&1
is "--check says the same" 3 "$?"

# a build that breaks does not write over the one that works
forget_calls
before=$(md5sum "$H/.local/lib/vetro/hyprglass.so")
stamp=$(cat "$H/.local/lib/vetro/hyprglass.built-for")
printf '#define GIT_COMMIT_HASH "%s"\n#define GIT_TAG "vtest"\n' "$hy1" >"$H/include/hyprland/src/version.h"
run FAKE_BUILD_FAILS=1 "$S/engine/build.sh" --force >"$WORK/b6.out" 2>&1
is "a failed build is a failure" 1 "$?"
has "it says so" "$WORK/b6.out" "the build failed"
is "the engine that worked is still there" "$before" "$(md5sum "$H/.local/lib/vetro/hyprglass.so")"
is "and so is its stamp" "$stamp" "$(cat "$H/.local/lib/vetro/hyprglass.built-for")"

# a pin for a commit this repository does not have
sed -i "s/^$hy1 .*/$hy1 deadbeefdeadbeefdeadbeefdeadbeefdeadbeef/" "$S/engine/pins"
rm -f "$H/.local/lib/vetro/hyprglass.built-for"
built "$hy1" >"$WORK/b7.out" 2>&1
is "a pin for a commit that is not here fails" 1 "$?"
has "and says which" "$WORK/b7.out" "has no commit deadbeef"

# tools and headers
new_home bare
printf '#define GIT_COMMIT_HASH "%s"\n' "$hy1" >"$H/include/hyprland/src/version.h"
run FAKE_NO_HYPRLAND_HEADERS=1 "$S/engine/build.sh" >"$WORK/b8.out" 2>&1
is "no Hyprland headers is a failure" 1 "$?"
has "and says so" "$WORK/b8.out" "Hyprland's headers are not installed"
run "$S/engine/build.sh" --nonsense >/dev/null 2>&1
is "an unknown word is refused" 2 "$?"

# the update hook never fails an update
new_home update
printf '#define GIT_COMMIT_HASH "%s"\n#define GIT_TAG "vtest"\n' "$hy3" >"$H/include/hyprland/src/version.h"
cp "$REPO/engine/post-update.sh" "$S/engine/post-update.sh"
run "$S/engine/post-update.sh" >"$WORK/hook.out" 2>&1
is "the update hook succeeds when the build is refused" 0 "$?"
has "and says the panes use plain blur" "$WORK/hook.out" "plain blur"
has "and leaves the log" "$H/.local/state/vetro/build.log" "no engine is pinned"

finish
