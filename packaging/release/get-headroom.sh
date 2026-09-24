#!/bin/sh
set -eu

default_repo="daniarjabagin/headroom"
repo="${HEADROOM_REPO:-$default_repo}"
version="${HEADROOM_VERSION:-latest}"
release_public_key='-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAAnmkvgoLXPACWO2aLcq1DkUPEQF+XnslimVdQLXh5J4=
-----END PUBLIC KEY-----'

usage() {
    cat <<EOF
Installs the latest Headroom release for the current user.

  curl -fsSL https://github.com/$default_repo/releases/latest/download/get-headroom.sh | sh
  curl -fsSL https://github.com/$default_repo/releases/latest/download/get-headroom.sh | sh -s -- --no-gnome

Environment:
  HEADROOM_VERSION  a release tag such as v0.1.0 (default: latest)
  HEADROOM_REPO     the GitHub repository to download from (default: $default_repo)

Arguments are passed to the bundled install.sh: --no-service, --no-gnome, --no-plasma.

  --tray     also install the Headroom tray (a tray icon with a popup for desktops other than
             GNOME Shell and KDE Plasma)
  --no-tray  never install the tray

Without either, the tray is installed when it is already installed, or when a graphical session
runs a desktop other than GNOME Shell or Plasma and GTK 4 and libadwaita are present. The build
with gtk4-layer-shell is chosen when that library is installed.

SHA256SUMS is checked against its Ed25519 signature (SHA256SUMS.sig) with OpenSSL 1.1.1 or
newer. Without such an OpenSSL the signature check is skipped with a warning and only the
checksums are verified.
EOF
}

fail() {
    printf 'get-headroom: %s\n' "$*" >&2
    exit 1
}

warn() {
    printf 'get-headroom: warning: %s\n' "$*" >&2
}

step() {
    printf '==> %s\n' "$*"
}

detect_arch() {
    case "$(uname -m)" in
        x86_64 | amd64) echo x86_64 ;;
        aarch64 | arm64) echo aarch64 ;;
        *) fail "unsupported architecture: $(uname -m)" ;;
    esac
}

have() {
    command -v "$1" >/dev/null 2>&1
}

download() {
    if have curl; then
        curl --proto '=https' --tlsv1.2 -fsSL -o "$2" "$1"
    elif have wget; then
        wget --https-only --secure-protocol=TLSv1_2 -q -O "$2" "$1"
    else
        fail "curl or wget is required"
    fi
}

latest_location() {
    url="https://github.com/$repo/releases/latest"
    if have curl; then
        curl --proto '=https' --tlsv1.2 -fsSL -o /dev/null -w '%{url_effective}' "$url"
    elif have wget; then
        wget --https-only --secure-protocol=TLSv1_2 -q -S -O /dev/null "$url" 2>&1 \
            | sed -n 's/^ *[Ll]ocation: *//p' | tr -d '\r' | tail -n 1
    else
        fail "curl or wget is required"
    fi
}

resolve_tag() {
    if [ "$version" != latest ]; then
        tag="$version"
    else
        location="$(latest_location)" || fail "could not find the latest release of $repo"
        tag="${location##*/releases/tag/}"
        [ "$tag" != "$location" ] || fail "could not find the latest release of $repo"
    fi
    case "$tag" in
        "" | *[!A-Za-z0-9._-]*) fail "not a release tag: $tag" ;;
    esac
    echo "$tag"
}

openssl_verifies_ed25519() {
    have openssl && openssl pkeyutl -help 2>&1 | grep -q -- '-rawin'
}

verify_signature() {
    if ! openssl_verifies_ed25519; then
        warn "OpenSSL 1.1.1 or newer was not found, so the signature of SHA256SUMS is not checked; relying on the checksums alone"
        return
    fi
    printf '%s\n' "$release_public_key" >"$1/release-key.pem"
    openssl base64 -d -A -in "$1/SHA256SUMS.sig" -out "$1/SHA256SUMS.sig.raw" 2>/dev/null \
        || fail "SHA256SUMS.sig is not a base64 signature"
    openssl pkeyutl -verify -rawin -pubin -inkey "$1/release-key.pem" \
        -in "$1/SHA256SUMS" -sigfile "$1/SHA256SUMS.sig.raw" >/dev/null 2>&1 \
        || fail "SHA256SUMS is not signed by the Headroom release key; the download was tampered with"
    step "Signature verified"
}

verify_checksum() {
    (
        cd "$1"
        grep "  $2\$" SHA256SUMS >"$2.sha256" || fail "$2 is not listed in SHA256SUMS"
        if have sha256sum; then
            sha256sum -c "$2.sha256" >/dev/null
        elif have shasum; then
            shasum -a 256 -c "$2.sha256" >/dev/null
        else
            fail "sha256sum or shasum is required"
        fi
    ) || fail "checksum mismatch for $2"
}

library_listing() {
    for candidate in ldconfig /sbin/ldconfig /usr/sbin/ldconfig; do
        if command -v "$candidate" >/dev/null 2>&1; then
            "$candidate" -p 2>/dev/null || true
            return
        fi
    done
}

has_library() {
    printf '%s\n' "$libraries" | grep -q "$1"
}

desktop_has_own_shell() {
    case "${XDG_CURRENT_DESKTOP:-}" in
        GNOME | GNOME:* | ubuntu | ubuntu:* | pop | pop:* | Zorin | Zorin:* | GNOME-Classic | GNOME-Classic:*)
            return 0 ;;
    esac
    case ":${XDG_CURRENT_DESKTOP:-}:" in
        *:KDE:*) return 0 ;;
    esac
    return 1
}

graphical_session() {
    [ -n "${WAYLAND_DISPLAY:-}" ] || [ -n "${DISPLAY:-}" ]
}

has_gtk_runtime() {
    has_library 'libgtk-4\.so\.1' && has_library 'libadwaita-1\.so\.0'
}

wants_tray() {
    case "$tray_mode" in
        off) return 1 ;;
        on)
            has_gtk_runtime || warn "GTK 4 or libadwaita was not found; the Headroom tray needs both to start"
            return 0
            ;;
    esac
    [ -x "$HOME/.local/bin/headroom-tray" ] && return 0
    graphical_session || return 1
    desktop_has_own_shell && return 1
    if ! has_gtk_runtime; then
        printf 'Skipping the Headroom tray: GTK 4 and libadwaita are needed; install them and run this again with --tray\n'
        return 1
    fi
    return 0
}

tray_variant() {
    if has_library 'libgtk4-layer-shell\.so\.0'; then
        echo linux-gnu-layershell
    else
        echo linux-gnu
    fi
}

tray_tarball_name() {
    name="$(grep -o "headroom-tray-[^ ]*-$1-$2\.tar\.gz\$" "$3/SHA256SUMS" | head -n 1)"
    [ -n "$name" ] || fail "no $1 $2 Headroom tray in this release"
    echo "$name"
}

fetch_verified() {
    step "Downloading $2"
    download "$base/$2" "$1/$2"
    verify_checksum "$1" "$2"
    step "Checksum verified"
    tar -xzf "$1/$2" -C "$1"
}

tarball_name() {
    name="$(grep -o "headroom-[^ ]*-$1-linux-musl\.tar\.gz\$" "$2/SHA256SUMS" | head -n 1)"
    [ -n "$name" ] || fail "no $1 tarball in this release"
    echo "$name"
}

main() {
    tray_mode=auto
    count=$#
    while [ "$count" -gt 0 ]; do
        arg="$1"
        shift
        count=$((count - 1))
        case "$arg" in
            -h | --help) usage; exit 0 ;;
            --tray) tray_mode=on ;;
            --no-tray) tray_mode=off ;;
            *) set -- "$@" "$arg" ;;
        esac
    done
    libraries="$(library_listing)"
    arch="$(detect_arch)"
    tag="$(resolve_tag)"
    base="https://github.com/$repo/releases/download/$tag"
    work="$(mktemp -d)"
    trap 'rm -rf "$work"' EXIT INT TERM
    step "Downloading the signed checksum list for $tag from $base"
    download "$base/SHA256SUMS" "$work/SHA256SUMS"
    download "$base/SHA256SUMS.sig" "$work/SHA256SUMS.sig" || fail "release $tag has no SHA256SUMS.sig"
    verify_signature "$work"
    name="$(tarball_name "$arch" "$work")"
    fetch_verified "$work" "$name"
    if wants_tray; then
        tray="$(tray_tarball_name "$arch" "$(tray_variant)" "$work")"
        fetch_verified "$work" "$tray"
        mv "$work/${tray%.tar.gz}" "$work/${name%.tar.gz}/tray"
        set -- "$@" --tray
    fi
    bash "$work/${name%.tar.gz}/install.sh" "$@"
}

main "$@"
