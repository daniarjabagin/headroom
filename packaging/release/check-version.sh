#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: packaging/release/check-version.sh TAG" >&2
    exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tag="$1"
expected="${tag#v}"

workspace_version() {
    awk '
        /^\[/ { section = $0 }
        section == "[workspace.package]" && $1 == "version" {
            gsub(/"/, "", $3)
            print $3
            exit
        }
    ' "$root/Cargo.toml"
}

gnome_version() {
    python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["version-name"])' \
        "$root/shell/gnome/metadata.json"
}

gnome_package_version() {
    python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["version"])' \
        "$root/shell/gnome/package.json"
}

gnome_lock_version() {
    python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["packages"][""]["version"])' \
        "$root/shell/gnome/package-lock.json"
}

plasma_version() {
    python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["KPlugin"]["Version"])' \
        "$root/shell/plasma/package/metadata.json"
}

status=0
check() {
    if [ "$2" = "$expected" ]; then
        printf '%-28s %s\n' "$1" "$2"
    else
        printf '%-28s %s, expected %s from tag %s\n' "$1" "$2" "$expected" "$tag" >&2
        status=1
    fi
}

check "Cargo.toml workspace" "$(workspace_version)"
check "GNOME extension metadata" "$(gnome_version)"
check "GNOME package.json" "$(gnome_package_version)"
check "GNOME package-lock.json" "$(gnome_lock_version)"
check "Plasma widget metadata" "$(plasma_version)"
exit "$status"
