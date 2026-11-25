#!/usr/bin/env bash
# Run inside xvfb-run and dbus-run-session. Never launches or installs Office.
set -euo pipefail
app=${1:?Usage: smoke-gui.sh EXECUTABLE}
log=$(mktemp)
export WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export LIBGL_ALWAYS_SOFTWARE=1
"$app" >"$log" 2>&1 &
pid=$!
cleanup() {
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  cat "$log"
  rm -f "$log"
}
trap cleanup EXIT
for attempt in $(seq 1 40); do
  if ! kill -0 "$pid" 2>/dev/null; then
    echo 'Installer exited before its window appeared.' >&2
    exit 1
  fi
  window=$(xdotool search --onlyvisible --name 'Office363' 2>/dev/null | head -n1 || true)
  if [[ -n "$window" ]]; then
    xdotool getwindowgeometry "$window"
    sleep 5
    kill -0 "$pid"
    echo 'PASS: installer window is visible and remains running.'
    exit 0
  fi
  sleep 0.5
done
echo 'Installer window did not become visible.' >&2
exit 1
