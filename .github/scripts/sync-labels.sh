#!/usr/bin/env bash
set -euo pipefail

usage() {
    echo "Usage: .github/scripts/sync-labels.sh [OWNER/REPO]" >&2
    echo "Creates or updates the labels in .github/labels.yml with gh label create --force." >&2
}

if [ "$#" -gt 1 ]; then
    usage
    exit 2
fi

repo="${1:-daniarjabagin/headroom}"
labels_file="$(dirname "$0")/../labels.yml"

command -v gh >/dev/null || { echo "gh is required" >&2; exit 1; }

name=""
color=""
while IFS= read -r line; do
    case "$line" in
        "- name: "*) name="${line#- name: }" ;;
        "  color: "*) color="${line#  color: }"; color="${color//\"/}" ;;
        "  description: "*)
            description="${line#  description: }"
            gh label create "$name" --repo "$repo" --color "$color" --description "$description" --force
            ;;
    esac
done < "$labels_file"
