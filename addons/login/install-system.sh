#!/bin/bash
# The login screen: the SpaceX launch video behind black glass, the next
# launch and D-DIN, as an SDDM theme. It is the only part of Vetro that needs
# root, so it is not part of install.sh: run it with sudo.
#
#   addons/login/install-system.sh            put it in place (and, if the earlier
#                                             login screen is there, migrate-system.sh
#                                             clears it away)
#   addons/login/install-system.sh --remove   Omarchy's own login screen again
#
# SYSROOT=DIR does it all under DIR instead of / (what the tests do, and a way
# to look before leaping).
#
# It copies the theme into /usr/share/sddm/themes/vetro (copied, not linked:
# the sddm user cannot read your home), writes /etc/sddm.conf.d/99-zz-vetro.conf
# to choose it, and makes /var/lib/vetro, the folder the launch daemon fills
# with what the login screen shows (the clip, the launch list, your avatar).
set -euo pipefail

HERE=$(dirname "$(readlink -f "$0")")
SYSROOT=${SYSROOT:-}
THEMES=$SYSROOT/usr/share/sddm/themes
CONF_DIR=$SYSROOT/etc/sddm.conf.d
SHARED=$SYSROOT/var/lib/vetro
# The folder is the user's, not root's: the daemon writes to it. Whoever asked
# for sudo, or else whoever owns this checkout.
owner=${SUDO_USER:-$(stat -c %U "$HERE")}

if [ -z "$SYSROOT" ] && [ "$(id -u)" != 0 ]; then
  echo "install-system.sh: this needs root (sudo); SYSROOT=DIR tries it under DIR" >&2
  exit 1
fi

install_theme() {
  mkdir -p "$THEMES" "$CONF_DIR"
  rm -rf "$THEMES/vetro"
  cp -r "$HERE/sddm/vetro" "$THEMES/vetro"
  install -d -o "$owner" -g "$(id -gn "$owner")" -m 755 "$SHARED"
  cat >"$CONF_DIR/99-zz-vetro.conf" <<'CONF'
# Vetro: its login screen (/usr/share/sddm/themes/vetro), read after Omarchy's
# 99-omarchy-login.conf. The theme reads /var/lib/vetro with XMLHttpRequest,
# which Qt only allows with QML_XHR_ALLOW_FILE_READ.
[Theme]
Current=vetro

[General]
GreeterEnvironment=QML_XHR_ALLOW_FILE_READ=1
CONF
  echo "  /usr/share/sddm/themes/vetro, /etc/sddm.conf.d/99-zz-vetro.conf"
  # The earlier login screen's data is carried over before its files go.
  if [ -x "$HERE/../../migrate-system.sh" ]; then "$HERE/../../migrate-system.sh"; fi
}

remove_theme() {
  rm -rf "$THEMES/vetro"
  rm -f "$CONF_DIR/99-zz-vetro.conf"
  echo "  Omarchy's login screen is back (the data in /var/lib/vetro stays)"
}

case ${1:-} in
  "") install_theme ;;
  --remove) remove_theme ;;
  *)
    awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"
    exit 2
    ;;
esac
