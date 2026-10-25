#!/bin/bash
# The root half of the move from liftoff: its login screen's files and data
# make way for Vetro's. addons/login/install-system.sh runs it after it has put
# Vetro's login screen in place; it can be run alone too, and twice. It needs
# root (sudo). SYSROOT=DIR does it all
# under DIR instead of / (what the tests do).
#
# What goes: the SDDM theme liftoff copied into /usr/share/sddm/themes, the file
# in /etc/sddm.conf.d that chose it, and /var/lib/liftoff, whose data (the
# launch clip and list, the avatar) is carried over to /var/lib/vetro first.
set -euo pipefail

SYSROOT=${SYSROOT:-}
if [ -z "$SYSROOT" ] && [ "$(id -u)" != 0 ]; then
  echo "migrate-system.sh: this needs root (sudo)" >&2
  exit 1
fi

old_data=$SYSROOT/var/lib/liftoff
new_data=$SYSROOT/var/lib/vetro

if [ -d "$old_data" ]; then
  mkdir -p "$new_data"
  cp -a -n "$old_data"/. "$new_data"/
  rm -rf "$old_data"
  echo "  /var/lib/liftoff moved to /var/lib/vetro"
fi
rm -rf "$SYSROOT/usr/share/sddm/themes/liftoff"
rm -f "$SYSROOT/etc/sddm.conf.d/99-zz-liftoff.conf"
