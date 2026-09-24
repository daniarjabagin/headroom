#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
icon_dir="$(cd "$script_dir/../Icon" && pwd)"
source_svg="$icon_dir/AppIcon.svg"
iconset="$icon_dir/AppIcon.iconset"

usage() {
    cat <<'EOF'
Usage: script/render-icon.sh

Renders Icon/AppIcon.svg into the committed Icon/AppIcon.iconset PNGs with rsvg-convert.
Run it after changing the SVG; script/bundle.sh packs the iconset with iconutil.
EOF
}

render() {
    local points="$1" scale="$2" suffix="$3"
    local pixels=$((points * scale))
    rsvg-convert --width "$pixels" --height "$pixels" --keep-aspect-ratio \
        --output "$iconset/icon_${points}x${points}${suffix}.png" "$source_svg"
}

main() {
    if (($# > 0)); then
        usage
        [[ "$1" == "-h" || "$1" == "--help" ]] && exit 0
        exit 2
    fi
    command -v rsvg-convert >/dev/null || { echo "rsvg-convert is required (librsvg)" >&2; exit 1; }
    mkdir -p "$iconset"
    local points
    for points in 16 32 128 256 512; do
        render "$points" 1 ""
        render "$points" 2 "@2x"
    done
    echo "rendered $iconset"
}

main "$@"
