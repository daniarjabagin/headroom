#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: packaging/release/stage-tray.sh --version VERSION --arch ARCH --variant VARIANT --binary PATH [--out DIR]

Assembles the Headroom tray release inputs from a built headroom-tray binary:
  DIR/tray-stage/headroom-tray        the binary nfpm packages as /usr/bin/headroom-tray
  DIR/tray-stage/*.desktop            autostart and menu entries that run /usr/bin/headroom-tray
  DIR/headroom-tray-VERSION-ARCH-VARIANT.tar.gz
                                      the tarball get-headroom.sh and `headroom update` install

VARIANT is linux-gnu (built without gtk4-layer-shell) or linux-gnu-layershell. DIR defaults to dist.
EOF
}

version=""
arch=""
variant=""
binary=""
out="dist"
while [ "$#" -gt 0 ]; do
    case "$1" in
        --version) version="$2"; shift 2 ;;
        --arch) arch="$2"; shift 2 ;;
        --variant) variant="$2"; shift 2 ;;
        --binary) binary="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        -h | --help) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done
if [ -z "$version" ] || [ -z "$arch" ] || [ -z "$variant" ] || [ -z "$binary" ]; then
    usage >&2
    exit 2
fi
case "$variant" in
    linux-gnu | linux-gnu-layershell) ;;
    *) printf 'stage-tray.sh: unknown variant %s\n' "$variant" >&2; exit 2 ;;
esac

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
templates="$root/packaging/tray"
stage="$out/tray-stage"
bundle_name="headroom-tray-$version-$arch-$variant"
bundle="$out/$bundle_name"
desktop_files=(headroom-tray.desktop io.github.daniarjabagin.HeadroomTray.desktop)

step() {
    printf '==> %s\n' "$*"
}

stage_package_files() {
    local file template
    step "Staging the tray for the packages"
    rm -rf "$stage"
    mkdir -p "$stage"
    install -m 0755 "$binary" "$stage/headroom-tray"
    for file in "${desktop_files[@]}"; do
        template="$(<"$templates/$file")"
        printf '%s\n' "${template//@EXEC@//usr/bin/headroom-tray}" >"$stage/$file"
        chmod 0644 "$stage/$file"
    done
}

assemble_bundle() {
    local file
    step "Assembling $bundle_name"
    rm -rf "$bundle"
    mkdir -p "$bundle"
    install -m 0755 "$binary" "$bundle/headroom-tray"
    install -m 0755 "$templates/install-tray.sh" "$bundle/install-tray.sh"
    for file in "${desktop_files[@]}"; do
        install -m 0644 "$templates/$file" "$bundle/$file"
    done
    printf '%s\n' "$variant" >"$bundle/variant"
    chmod 0644 "$bundle/variant"
    install -m 0644 "$root/LICENSE" "$root/README.md" "$bundle/"
}

archive_bundle() {
    step "Writing $bundle_name.tar.gz"
    tar -C "$out" --owner=0 --group=0 --numeric-owner -czf "$out/$bundle_name.tar.gz" "$bundle_name"
    rm -rf "$bundle"
}

stage_package_files
assemble_bundle
archive_bundle
