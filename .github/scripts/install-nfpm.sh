#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: .github/scripts/install-nfpm.sh BIN_DIR" >&2
    exit 2
fi

version="2.47.0"
bin_dir="$1"

case "$(uname -m)" in
    x86_64)
        asset="nfpm_${version}_Linux_x86_64.tar.gz"
        sha256="0660ca602b2d2d2ae4781a06c692b3eeb9d437ffea05b831d76e41f4a3188783"
        ;;
    aarch64)
        asset="nfpm_${version}_Linux_arm64.tar.gz"
        sha256="1c0f5f2999b9a974bfb04fdb0cc3306096de530ac5dbb25d739cc5f5219c919c"
        ;;
    *)
        echo "no pinned nfpm build for $(uname -m)" >&2
        exit 1
        ;;
esac

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
curl --proto '=https' --tlsv1.2 -fsSLo "$work/$asset" \
    "https://github.com/goreleaser/nfpm/releases/download/v$version/$asset"
echo "$sha256  $work/$asset" | sha256sum -c -
mkdir -p "$bin_dir"
tar -xzf "$work/$asset" -C "$bin_dir" nfpm
"$bin_dir/nfpm" --version
