#!/usr/bin/env bash
set -euo pipefail

export LC_ALL=C

binary="${1:?usage: check-layer-shell-link.sh <headroom-tray binary>}"

loaded="$(ldd "$binary" | awk '{ print $1 }')"
printf 'Load order of %s:\n%s\n' "$binary" "$loaded"

layer_line="$(printf '%s\n' "$loaded" | grep -n '^libgtk4-layer-shell\.so' | cut -d: -f1 || true)"
wayland_line="$(printf '%s\n' "$loaded" | grep -n '^libwayland-client\.so' | cut -d: -f1 || true)"

if [ -z "$layer_line" ]; then
    echo "libgtk4-layer-shell is not linked; build with --features layer-shell" >&2
    exit 1
fi
if [ -n "$wayland_line" ] && [ "$wayland_line" -lt "$layer_line" ]; then
    echo "libwayland-client loads before libgtk4-layer-shell; layer surfaces would silently break" >&2
    exit 1
fi
echo "libgtk4-layer-shell loads before libwayland-client"
