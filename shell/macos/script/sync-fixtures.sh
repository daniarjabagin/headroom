#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../../.." && pwd)"
fixtures="$script_dir/../Tests/HeadroomKitTests/Fixtures"

sources=(
    "crates/headroom-daemon/src/state/snapshots/state_full.json"
    "crates/headroom-daemon/src/state/snapshots/state_empty.json"
    "crates/headroom-daemon/src/state/snapshots/state_combined.json"
    "crates/headroom-daemon/src/state/snapshots/account_no_subscription.json"
    "crates/headroom/src/render/fixtures/providers.json"
)

mkdir -p "$fixtures"
for source in "${sources[@]}"; do
    cp "$repo_root/$source" "$fixtures/"
    echo "synced $source"
done
