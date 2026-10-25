#!/bin/sh
# render.sh OUT.png: draws reference.html with the job.js beside it in headless
# Chromium and saves the canvas, pixel for pixel. Software GL is asked for, so
# the picture does not depend on the machine's GPU or driver.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
browser=$(command -v chromium || command -v chromium-browser || command -v google-chrome-stable || true)
[ -n "$browser" ] || { echo "render: no Chromium found" >&2; exit 1; }
profile=$(mktemp -d)
trap 'rm -rf "$profile"' EXIT
"$browser" --headless=new --no-first-run --user-data-dir="$profile" --allow-file-access-from-files \
    --use-angle=swiftshader --enable-unsafe-swiftshader --ignore-gpu-blocklist --force-device-scale-factor=1 \
    --run-all-compositor-stages-before-draw --virtual-time-budget=120000 --timeout=180000 \
    --dump-dom "file://$here/reference.html" 2>/dev/null |
python3 -I -c '
import base64, re, sys
dom = sys.stdin.read()
# the page reports through its one link: a picture in its address, or a failure in its text
failed = re.search(r"<a id=\"result\"[^>]*>FAILED: ([^<]*)</a>", dom)
if failed:
    sys.exit("render: the page failed: " + failed.group(1))
found = re.search(r"<a id=\"result\"[^>]*href=\"data:image/png;base64,([^\"]+)\"", dom)
if not found:
    sys.exit("render: the page produced no picture")
open(sys.argv[1], "wb").write(base64.b64decode(found.group(1)))
' "$1"
