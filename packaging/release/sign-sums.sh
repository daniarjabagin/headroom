#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: RELEASE_SIGNING_KEY="$(cat key.pem)" packaging/release/sign-sums.sh SHA256SUMS PUBLIC_KEY.pem

Signs SHA256SUMS with the Ed25519 private key (PEM) in $RELEASE_SIGNING_KEY and writes
SHA256SUMS.sig next to it: the raw 64-byte signature, base64-encoded on one line. The signature
is checked against PUBLIC_KEY.pem, the key pinned in headroom and get-headroom.sh, before it is
written, so a mismatched secret fails the release instead of shipping an unverifiable one.
EOF
}

fail() {
    printf 'sign-sums: %s\n' "$*" >&2
    exit 1
}

if [ "$#" -ne 2 ]; then
    usage >&2
    exit 2
fi
sums="$1"
public_key="$2"
[ -f "$sums" ] || fail "$sums does not exist"
[ -f "$public_key" ] || fail "$public_key does not exist"
[ -n "${RELEASE_SIGNING_KEY:-}" ] || fail "RELEASE_SIGNING_KEY is empty"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT INT TERM
(
    umask 077
    printf '%s\n' "$RELEASE_SIGNING_KEY" >"$work/key.pem"
)
openssl pkeyutl -sign -rawin -inkey "$work/key.pem" -in "$sums" -out "$work/signature" \
    || fail "could not sign $sums; RELEASE_SIGNING_KEY must be an Ed25519 private key in PEM form"
openssl pkeyutl -verify -rawin -pubin -inkey "$public_key" -in "$sums" -sigfile "$work/signature" >/dev/null \
    || fail "RELEASE_SIGNING_KEY does not match the pinned public key $public_key"
openssl base64 -A -in "$work/signature" >"$sums.sig.new"
printf '\n' >>"$sums.sig.new"
mv -f "$sums.sig.new" "$sums.sig"
printf 'Signed %s\n' "$sums"
