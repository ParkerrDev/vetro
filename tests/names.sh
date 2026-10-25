#!/bin/bash
# No file in the tree, and no file's name, has the name of an earlier project
# (liftoff, meniscus) in it, except where that is the point: the code that moves
# a home folder from them to Vetro, its tests, the history notes, and the engine
# (a fork that is being rewritten, whose lens model is called meniscus after the
# curved surface of a liquid; it is left as its authors wrote it). A name that
# turns up anywhere else is a path, a command or a folder that was not renamed,
# and would be a thing that does not exist at run time.
# shellcheck source-path=SCRIPTDIR
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"

cd "$REPO" || exit 2

# Where the old names are allowed, as globs on the path.
ALLOWED=(
  migrate.sh
  migrate-system.sh
  tests/migrate.sh
  tests/owner-home.sh
  tests/system.sh
  tests/names.sh
  docs/history.md
  engine/Makefile
  engine/README.md
  engine/CHANGELOG.md
  'engine/src/*'
  'engine/docs/*'
  'engine/tests/*'
  # The engine's Rust: its lens model keeps the name it was written under.
  'engine/crates/*'
  engine/BUILD.md
  engine/NOTICE
)

allowed() {
  local glob
  for glob in "${ALLOWED[@]}"; do
    # shellcheck disable=SC2053  # the right side is meant as a pattern
    [[ $1 == $glob ]] && return 0
  done
  return 1
}

mapfile -t files < <(git ls-files --cached --others --exclude-standard)
named=0
for file in "${files[@]}"; do
  [ -e "$file" ] || continue
  allowed "$file" && continue
  checks=$((checks + 1))
  if grep -qiE 'liftoff|meniscus' <<<"$file"; then
    fail "the name of an earlier project is in a file's name: $file"
    named=$((named + 1))
  fi
  if grep -Iiq -e liftoff -e meniscus "$file" 2>/dev/null; then
    fail "the name of an earlier project is in $file: $(grep -Iin -e liftoff -e meniscus "$file" | head -n 2 | cut -c1-100 | tr '\n' '|')"
    named=$((named + 1))
  fi
done
is "files looked at" "$((${#files[@]} > 100))" 1

# The allow-list is not a hiding place: each entry must be there, and need it.
for glob in "${ALLOWED[@]}"; do
  checks=$((checks + 1))
  found=0
  for file in "${files[@]}"; do
    # shellcheck disable=SC2053
    [[ $file == $glob ]] && found=1 && break
  done
  [ "$found" = 1 ] || fail "the allow-list names $glob, which is not in the tree"
done

finish
