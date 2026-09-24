#!/usr/bin/env bash
set -euo pipefail

export LC_ALL=C

if [ "$#" -ne 2 ]; then
    echo "Usage: .github/scripts/check-glibc.sh BINARY MAX_GLIBC_VERSION" >&2
    exit 2
fi
binary="$1"
max="$2"

needed="$(objdump -T "$binary" | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/^GLIBC_//' | sort -u -V | tail -n 1)"
if [ -z "$needed" ]; then
    echo "$binary needs no versioned glibc symbols" >&2
    exit 1
fi
printf '%s needs glibc %s (allowed: %s)\n' "$binary" "$needed" "$max"
if [ "$(printf '%s\n%s\n' "$needed" "$max" | sort -V | tail -n 1)" != "$max" ]; then
    printf '%s needs glibc %s, newer than %s; it would not start on the oldest supported distribution\n' \
        "$binary" "$needed" "$max" >&2
    exit 1
fi
