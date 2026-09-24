#!/bin/sh
set -eu

default_repo="daniarjabagin/headroom"
repo="${HEADROOM_REPO:-$default_repo}"
version="${HEADROOM_VERSION:-latest}"

usage() {
    cat <<EOF
Installs the latest Headroom release for the current user.

  curl -fsSL https://github.com/$default_repo/releases/latest/download/get-headroom.sh | sh
  curl -fsSL https://github.com/$default_repo/releases/latest/download/get-headroom.sh | sh -s -- --no-gnome

Environment:
  HEADROOM_VERSION  a release tag such as v0.1.0 (default: latest)
  HEADROOM_REPO     the GitHub repository to download from (default: $default_repo)

Arguments are passed to the bundled install.sh: --no-service, --no-gnome, --no-plasma.
EOF
}

fail() {
    printf 'get-headroom: %s\n' "$*" >&2
    exit 1
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

release_url() {
    if [ "$version" = latest ]; then
        echo "https://github.com/$repo/releases/latest/download"
    else
        echo "https://github.com/$repo/releases/download/$version"
    fi
}

download() {
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -fsSL -o "$2" "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$2" "$1"
    else
        fail "curl or wget is required"
    fi
}

verify_checksum() {
    (
        cd "$1"
        grep "  $2\$" SHA256SUMS >"$2.sha256" || fail "$2 is not listed in SHA256SUMS"
        if command -v sha256sum >/dev/null 2>&1; then
            sha256sum -c "$2.sha256" >/dev/null
        elif command -v shasum >/dev/null 2>&1; then
            shasum -a 256 -c "$2.sha256" >/dev/null
        else
            fail "sha256sum or shasum is required"
        fi
    ) || fail "checksum mismatch for $2"
}

tarball_name() {
    name="$(grep -o "headroom-[^ ]*-$1-linux-musl\.tar\.gz\$" "$2/SHA256SUMS" | head -n 1)"
    [ -n "$name" ] || fail "no $1 tarball in this release"
    echo "$name"
}

main() {
    case "${1:-}" in
        -h | --help) usage; exit 0 ;;
    esac
    arch="$(detect_arch)"
    base="$(release_url)"
    work="$(mktemp -d)"
    trap 'rm -rf "$work"' EXIT INT TERM
    step "Downloading the checksum list from $base"
    download "$base/SHA256SUMS" "$work/SHA256SUMS"
    name="$(tarball_name "$arch" "$work")"
    step "Downloading $name"
    download "$base/$name" "$work/$name"
    verify_checksum "$work" "$name"
    step "Checksum verified"
    tar -xzf "$work/$name" -C "$work"
    bash "$work/${name%.tar.gz}/install.sh" "$@"
}

main "$@"
