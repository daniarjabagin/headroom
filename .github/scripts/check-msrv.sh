#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

toml_string() {
    awk -v key="$2" '$1 == key && $2 == "=" { gsub(/"/, "", $3); print $3; exit }' "$1"
}

channel="$(toml_string "$root/rust-toolchain.toml" channel)"
msrv="$(toml_string "$root/Cargo.toml" rust-version)"

if [ "$channel" != "$msrv" ]; then
    echo "rust-toolchain.toml pins $channel but Cargo.toml declares rust-version $msrv" >&2
    exit 1
fi
echo "toolchain and rust-version agree: $channel"
