#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: packaging/homebrew/update-tap.sh VERSION [--sha256-file FILE] [--output FILE]

Renders the Homebrew cask packaging/homebrew/headroom.rb for release VERSION (0.4.1 or v0.4.1)
with the checksum of Headroom-VERSION-universal.dmg, taken from the release's .dmg.sha256.

Without --output it clones git@github.com:daniarjabagin/homebrew-tap.git, writes
Casks/headroom.rb (and README.md from tap-README.md when the tap has none), commits and pushes.
The SSH key comes from HOMEBREW_TAP_DEPLOY_KEY (the private key text of a deploy key with write
access to the tap only). With --output it writes the rendered cask to FILE and pushes nothing.

  --sha256-file FILE   read the checksum from FILE instead of downloading it

Environment: HEADROOM_REPO (default daniarjabagin/headroom), HOMEBREW_TAP_REPO (default
daniarjabagin/homebrew-tap), TAP_GIT_NAME, TAP_GIT_EMAIL.
EOF
}

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
template="$root/packaging/homebrew/headroom.rb"
tap_readme="$root/packaging/homebrew/tap-README.md"
repo="${HEADROOM_REPO:-daniarjabagin/headroom}"
tap_repo="${HOMEBREW_TAP_REPO:-daniarjabagin/homebrew-tap}"
git_name="${TAP_GIT_NAME:-Daniar Jabagin}"
git_email="${TAP_GIT_EMAIL:-150630975+daniarjabagin@users.noreply.github.com}"
github_host_fingerprint="SHA256:+DiY3wvvV6TuJJhbpZisF/zLDA0zPMSvHdkr4UvCOqU"

version=""
sha256_file=""
output=""
work=""

fail() {
    echo "update-tap: $*" >&2
    exit 1
}

step() {
    printf '==> %s\n' "$*" >&2
}

cleanup() {
    [ -z "$work" ] || rm -rf "$work"
}

parse_args() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --sha256-file) sha256_file="${2:?--sha256-file needs a file}"; shift 2 ;;
            --output) output="${2:?--output needs a file}"; shift 2 ;;
            -h | --help) usage; exit 0 ;;
            -*) usage >&2; exit 2 ;;
            *) [ -z "$version" ] || { usage >&2; exit 2; }; version="${1#v}"; shift ;;
        esac
    done
    [ -n "$version" ] || { usage >&2; exit 2; }
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "$version is not a release version like 0.4.1"
}

dmg_name() {
    printf 'Headroom-%s-universal.dmg\n' "$version"
}

fetch_checksum_file() {
    local target="$work/dmg.sha256"
    if [ -n "$sha256_file" ]; then
        cp "$sha256_file" "$target"
    else
        step "Downloading $(dmg_name).sha256"
        curl -fsSL --retry 3 -o "$target" \
            "https://github.com/$repo/releases/download/v$version/$(dmg_name).sha256"
    fi
    printf '%s\n' "$target"
}

dmg_checksum() {
    local file="$1" name sum
    name="$(dmg_name)"
    sum="$(awk -v name="$name" '$2 == name || $2 == "*" name { print $1 }' "$file")"
    [[ "$sum" =~ ^[0-9a-f]{64}$ ]] || fail "$(basename "$file") has no single checksum for $name"
    printf '%s\n' "$sum"
}

render() {
    local sum="$1" dest="$2"
    mkdir -p "$(dirname "$dest")"
    sed -e "s/^  version \".*\"$/  version \"$version\"/" \
        -e "s/^  sha256 .*/  sha256 \"$sum\"/" \
        "$template" > "$dest.new"
    grep -qx "  version \"$version\"" "$dest.new" || fail "version not written"
    grep -qx "  sha256 \"$sum\"" "$dest.new" || fail "sha256 not written"
    mv "$dest.new" "$dest"
}

ssh_command() {
    local key="$work/deploy_key" known="$work/known_hosts"
    [ -n "${HOMEBREW_TAP_DEPLOY_KEY:-}" ] || fail "HOMEBREW_TAP_DEPLOY_KEY is not set"
    (umask 077 && printf '%s\n' "$HOMEBREW_TAP_DEPLOY_KEY" > "$key")
    ssh-keyscan -t ed25519 github.com 2>/dev/null > "$known"
    ssh-keygen -lf "$known" | grep -qF "$github_host_fingerprint" ||
        fail "github.com did not present the expected host key $github_host_fingerprint"
    printf 'ssh -i %s -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=%s\n' \
        "$key" "$known"
}

publish() {
    local sum="$1" clone="$work/tap"
    GIT_SSH_COMMAND="$(ssh_command)"
    export GIT_SSH_COMMAND
    step "Cloning $tap_repo"
    git -c init.defaultBranch=main clone -q "git@github.com:$tap_repo.git" "$clone"
    render "$sum" "$clone/Casks/headroom.rb"
    [ -f "$clone/README.md" ] || cp "$tap_readme" "$clone/README.md"
    git -C "$clone" add Casks/headroom.rb README.md
    if git -C "$clone" diff --cached --quiet; then
        step "The tap already has headroom $version"
        return
    fi
    git -C "$clone" -c user.name="$git_name" -c user.email="$git_email" \
        commit -q -m "headroom $version"
    step "Pushing headroom $version to $tap_repo"
    git -C "$clone" push -q origin HEAD
}

main() {
    parse_args "$@"
    work="$(mktemp -d)"
    trap cleanup EXIT
    local checksum_file sum
    checksum_file="$(fetch_checksum_file)"
    sum="$(dmg_checksum "$checksum_file")"
    if [ -n "$output" ]; then
        render "$sum" "$output"
        step "Rendered the headroom $version cask into $output"
    else
        publish "$sum"
    fi
}

main "$@"
