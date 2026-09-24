#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: packaging/release/stage.sh --version VERSION --arch ARCH --binary PATH --extension-zip PATH [--out DIR]

Assembles release inputs from a built headroom binary and the packed GNOME Shell extension:
  DIR/stage/headroom                  the binary nfpm packages as /usr/bin/headroom
  DIR/stage/gnome-extension/          the unpacked extension with compiled schemas
  DIR/headroom-VERSION-ARCH-linux-musl.tar.gz
                                      the self-contained tarball for ~/.local installs

DIR defaults to dist.
EOF
}

version=""
arch=""
binary=""
extension_zip=""
out="dist"
while [ "$#" -gt 0 ]; do
    case "$1" in
        --version) version="$2"; shift 2 ;;
        --arch) arch="$2"; shift 2 ;;
        --binary) binary="$2"; shift 2 ;;
        --extension-zip) extension_zip="$2"; shift 2 ;;
        --out) out="$2"; shift 2 ;;
        -h | --help) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done
if [ -z "$version" ] || [ -z "$arch" ] || [ -z "$binary" ] || [ -z "$extension_zip" ]; then
    usage >&2
    exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
stage="$out/stage"
bundle_name="headroom-$version-$arch-linux-musl"
bundle="$out/$bundle_name"

step() {
    printf '==> %s\n' "$*"
}

stage_binary() {
    step "Staging the binary"
    rm -rf "$stage"
    mkdir -p "$stage"
    install -m 0755 "$binary" "$stage/headroom"
}

stage_extension() {
    step "Unpacking the GNOME Shell extension and compiling its schemas"
    mkdir -p "$stage/gnome-extension"
    python3 -m zipfile -e "$extension_zip" "$stage/gnome-extension"
    find "$stage/gnome-extension" -type d -exec chmod 0755 {} +
    find "$stage/gnome-extension" -type f -exec chmod 0644 {} +
    glib-compile-schemas --strict "$stage/gnome-extension/schemas"
}

assemble_bundle() {
    step "Assembling $bundle_name"
    rm -rf "$bundle"
    mkdir -p "$bundle/systemd" "$bundle/dbus" "$bundle/gnome" "$bundle/plasma" "$bundle/icons"
    install -m 0755 "$stage/headroom" "$bundle/headroom"
    install -m 0755 "$root/packaging/release/install-from-tarball.sh" "$bundle/install.sh"
    install -m 0755 "$root/packaging/uninstall.sh" "$bundle/uninstall.sh"
    install -m 0644 "$root/packaging/systemd/headroom.service" "$bundle/systemd/headroom.service"
    install -m 0644 "$root/packaging/dbus/io.github.daniarjabagin.Headroom.service" \
        "$bundle/dbus/io.github.daniarjabagin.Headroom.service"
    install -m 0644 "$root/packaging/icons/headroom.svg" "$bundle/icons/headroom.svg"
    install -m 0644 "$root/assets/brand/headroom-symbolic.svg" "$bundle/icons/headroom-symbolic.svg"
    install -m 0644 "$extension_zip" "$bundle/gnome/headroom@daniarjabagin.github.io.shell-extension.zip"
    cp -R "$root/shell/plasma/package" "$bundle/plasma/io.github.daniarjabagin.headroom"
    install -m 0644 "$root/LICENSE" "$root/README.md" "$bundle/"
}

archive_bundle() {
    step "Writing $bundle_name.tar.gz"
    tar -C "$out" --owner=0 --group=0 --numeric-owner -czf "$out/$bundle_name.tar.gz" "$bundle_name"
    rm -rf "$bundle"
}

stage_binary
stage_extension
assemble_bundle
archive_bundle
