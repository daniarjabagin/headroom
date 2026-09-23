#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: .github/scripts/check-static.sh BINARY" >&2
    exit 2
fi

binary="$1"
description="$(file -b "$binary")"
linkage="$(ldd "$binary" 2>&1 || true)"
printf 'file: %s\nldd:  %s\n' "$description" "$linkage"

case "$description" in
    *"static-pie linked"* | *"statically linked"*) ;;
    *) echo "$binary is not statically linked" >&2; exit 1 ;;
esac
case "$linkage" in
    *"not a dynamic executable"* | *"statically linked"*) ;;
    *) echo "ldd reports dynamic dependencies for $binary" >&2; exit 1 ;;
esac
"$binary" --version
