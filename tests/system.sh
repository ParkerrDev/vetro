#!/bin/bash
# The login screen, the one part that needs root, tried under a scratch
# SYSROOT as an ordinary user: the theme and the file that chooses it go in,
# the earlier login screen's files go out and its data is carried across, a
# second run changes nothing, --remove gives Omarchy's own back, and without a
# SYSROOT and without root it refuses before touching anything.
# shellcheck source-path=SCRIPTDIR
set -uo pipefail
# shellcheck source=lib.sh
. "$(dirname "$(readlink -f "$0")")/lib.sh"
make_bin
new_home system

if [ "$(id -u)" = 0 ]; then
  echo "skip  tests/system.sh will not run as root"
  exit 0
fi

R=$WORK/sysroot
mkdir -p "$R/usr/share/sddm/themes/liftoff" "$R/usr/share/sddm/themes/omarchy" "$R/etc/sddm.conf.d" "$R/var/lib/liftoff"
echo "old theme" >"$R/usr/share/sddm/themes/liftoff/Main.qml"
echo "omarchy's theme" >"$R/usr/share/sddm/themes/omarchy/Main.qml"
printf '[Theme]\nCurrent=liftoff\n' >"$R/etc/sddm.conf.d/99-zz-liftoff.conf"
printf '[Theme]\nCurrent=omarchy\n' >"$R/etc/sddm.conf.d/99-omarchy-login.conf"
echo "the clip" >"$R/var/lib/liftoff/idle.mp4"
echo '{"upcoming": []}' >"$R/var/lib/liftoff/launches.json"

run SYSROOT="$R" "$REPO/addons/login/install-system.sh" >"$WORK/sys1.out" 2>&1 || fail "install-system.sh failed: $(tail -n 3 "$WORK/sys1.out")"
ok "the theme is in place" test -f "$R/usr/share/sddm/themes/vetro/Main.qml"
ok "with its metadata" test -f "$R/usr/share/sddm/themes/vetro/metadata.desktop"
has "the metadata names the theme" "$R/usr/share/sddm/themes/vetro/metadata.desktop" "Name=vetro"
cmp -s "$REPO/addons/login/sddm/vetro/Main.qml" "$R/usr/share/sddm/themes/vetro/Main.qml" || fail "the theme was not copied as it is"
has "the file that chooses it" "$R/etc/sddm.conf.d/99-zz-vetro.conf" "Current=vetro"
has "and lets it read the folder" "$R/etc/sddm.conf.d/99-zz-vetro.conf" "QML_XHR_ALLOW_FILE_READ=1"
is "the data moved across" "the clip" "$(cat "$R/var/lib/vetro/idle.mp4")"
ok "all of it" test -s "$R/var/lib/vetro/launches.json"
no "the earlier theme is gone" test -e "$R/usr/share/sddm/themes/liftoff"
no "and the file that chose it" test -e "$R/etc/sddm.conf.d/99-zz-liftoff.conf"
no "and its data folder" test -e "$R/var/lib/liftoff"
is "Omarchy's own are untouched" "omarchy's theme" "$(cat "$R/usr/share/sddm/themes/omarchy/Main.qml")"
ok "and its choice" test -f "$R/etc/sddm.conf.d/99-omarchy-login.conf"
(cd "$R" && find . -type f -exec md5sum {} + | sort -k2) >"$WORK/sys.1"
run SYSROOT="$R" "$REPO/addons/login/install-system.sh" >/dev/null 2>&1 || fail "install-system.sh a second time failed"
(cd "$R" && find . -type f -exec md5sum {} + | sort -k2) >"$WORK/sys.2"
is "a second run changes nothing" "$(cat "$WORK/sys.1")" "$(cat "$WORK/sys.2")"

run SYSROOT="$R" "$REPO/addons/login/install-system.sh" --remove >/dev/null 2>&1 || fail "install-system.sh --remove failed"
no "--remove takes the theme out" test -e "$R/usr/share/sddm/themes/vetro"
no "and the choice" test -e "$R/etc/sddm.conf.d/99-zz-vetro.conf"
ok "the data stays" test -s "$R/var/lib/vetro/idle.mp4"
ok "Omarchy's choice is still there" test -f "$R/etc/sddm.conf.d/99-omarchy-login.conf"

# the root half of the move alone, with nothing to do
run SYSROOT="$R" "$REPO/migrate-system.sh" >/dev/null 2>&1 || fail "migrate-system.sh with nothing to do failed"

# no SYSROOT and no root: refused, and nothing is touched
run "$REPO/addons/login/install-system.sh" >"$WORK/sys3.out" 2>&1
is "without root and without a SYSROOT it is refused" 1 "$?"
has "and says why" "$WORK/sys3.out" "needs root"
run "$REPO/migrate-system.sh" >/dev/null 2>&1
is "so is the move" 1 "$?"
run SYSROOT="$R" "$REPO/addons/login/install-system.sh" --nonsense >/dev/null 2>&1
is "an unknown word is refused" 2 "$?"

finish
