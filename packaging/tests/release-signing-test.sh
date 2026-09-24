#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tag="v9.9.9"
sandbox="$(mktemp -d)"
trap 'rm -rf "$sandbox"' EXIT
failures=0
installer_args=(--no-gnome)

pass() {
    printf 'ok   %s\n' "$*"
}

flunk() {
    printf 'FAIL %s\n' "$*" >&2
    failures=$((failures + 1))
}

check() {
    local name="$1"
    shift
    if "$@"; then pass "$name"; else flunk "$name"; fi
}

arch() {
    case "$(uname -m)" in
        x86_64 | amd64) echo x86_64 ;;
        aarch64 | arm64) echo aarch64 ;;
    esac
}

tool_dir() {
    local dir="$1" tool
    shift
    mkdir -p "$dir"
    for tool in "$@"; do
        ln -sf "$(command -v "$tool")" "$dir/$tool"
    done
}

make_keys() {
    openssl genpkey -algorithm ed25519 -out "$sandbox/release.pem" 2>/dev/null
    openssl pkey -in "$sandbox/release.pem" -pubout -out "$sandbox/release.pub.pem"
    openssl genpkey -algorithm ed25519 -out "$sandbox/other.pem" 2>/dev/null
}

bundle() {
    echo "headroom-9.9.9-$(arch)-linux-musl"
}

tray_bundle() {
    echo "headroom-tray-9.9.9-$(arch)-$1"
}

make_release() {
    local release="$sandbox/release" bundle variant tray
    bundle="$(bundle)"
    mkdir -p "$release" "$sandbox/src/$bundle"
    cat >"$sandbox/src/$bundle/install.sh" <<'EOF'
printf '%s\n' "$@" >"$INSTALL_ARGS"
here="$(dirname "$0")"
if [ -f "$here/tray/variant" ]; then cat "$here/tray/variant" >"$TRAY_SEEN"; fi
EOF
    tar -C "$sandbox/src" -czf "$release/$bundle.tar.gz" "$bundle"
    for variant in linux-gnu linux-gnu-layershell; do
        tray="$(tray_bundle "$variant")"
        mkdir -p "$sandbox/src/$tray"
        echo "$variant" >"$sandbox/src/$tray/variant"
        tar -C "$sandbox/src" -czf "$release/$tray.tar.gz" "$tray"
    done
    (cd "$release" && sha256sum -- *.tar.gz >SHA256SUMS)
}

sign_release() {
    rm -f "$sandbox/release/SHA256SUMS.sig"
    RELEASE_SIGNING_KEY="$(cat "$1")" \
        "$root/packaging/release/sign-sums.sh" "$sandbox/release/SHA256SUMS" "$sandbox/release.pub.pem" >/dev/null
}

forge_signature() {
    openssl pkeyutl -sign -rawin -inkey "$sandbox/other.pem" -in "$sandbox/release/SHA256SUMS" \
        -out "$sandbox/forged"
    openssl base64 -A -in "$sandbox/forged" >"$sandbox/release/SHA256SUMS.sig"
}

installer_with_test_key() {
    awk -v key="$(cat "$sandbox/release.pub.pem")" '
        /^release_public_key=/ { print "release_public_key='\''" key "'\''"; skip = 1; next }
        skip && /-----END PUBLIC KEY-----/ { skip = 0; next }
        !skip { print }
    ' "$root/packaging/release/get-headroom.sh" >"$sandbox/get-headroom.sh"
}

write_stubs() {
    cat >"$sandbox/stubs/curl" <<'EOF'
#!/bin/sh
printf 'curl %s\n' "$*" >>"$STUB_LOG"
out="" url=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -o | -w | --proto) [ "$1" = -o ] && out="$2"; shift 2 ;;
        -*) shift ;;
        *) url="$1"; shift ;;
    esac
done
case "$url" in
    https://github.com/*/releases/latest) printf 'https://github.com/owner/repo/releases/tag/%s' "$FAKE_TAG" ;;
    https://github.com/*/releases/download/"$FAKE_TAG"/*)
        [ -f "$FAKE_RELEASE/${url##*/}" ] || exit 22
        cp "$FAKE_RELEASE/${url##*/}" "$out" ;;
    *) exit 22 ;;
esac
EOF
    cat >"$sandbox/stubs/wget" <<'EOF'
#!/bin/sh
printf 'wget %s\n' "$*" >>"$STUB_LOG"
out="" url=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -O) out="$2"; shift 2 ;;
        -*) shift ;;
        *) url="$1"; shift ;;
    esac
done
case "$url" in
    https://github.com/*/releases/latest)
        printf '  HTTP/1.1 302 Found\n  Location: https://github.com/owner/repo/releases/tag/%s\r\n' "$FAKE_TAG" >&2 ;;
    https://github.com/*/releases/download/"$FAKE_TAG"/*)
        [ -f "$FAKE_RELEASE/${url##*/}" ] || exit 8
        cp "$FAKE_RELEASE/${url##*/}" "$out" ;;
    *) exit 8 ;;
esac
EOF
    cat >"$sandbox/stubs/ldconfig" <<'EOF'
#!/bin/sh
for library in $FAKE_LIBRARIES; do
    printf '\t%s (libc6,x86-64) => /usr/lib/%s\n' "$library" "$library"
done
EOF
    chmod +x "$sandbox/stubs/curl" "$sandbox/stubs/wget" "$sandbox/stubs/ldconfig"
}

prepare_paths() {
    local base=(sh bash uname mktemp rm grep head sed tr tail tar gzip sha256sum cat cp mkdir awk mv dirname)
    mkdir -p "$sandbox/stubs"
    write_stubs
    tool_dir "$sandbox/path-curl" "${base[@]}" openssl
    tool_dir "$sandbox/path-bare" "${base[@]}"
    tool_dir "$sandbox/path-wget" "${base[@]}" openssl
    ln -sf "$sandbox/stubs/curl" "$sandbox/path-curl/curl"
    ln -sf "$sandbox/stubs/curl" "$sandbox/path-bare/curl"
    ln -sf "$sandbox/stubs/wget" "$sandbox/path-wget/wget"
    for dir in path-curl path-bare path-wget; do
        ln -sf "$sandbox/stubs/ldconfig" "$sandbox/$dir/ldconfig"
    done
}

run_installer() {
    local path="$1"
    shift
    rm -f "$sandbox/args" "$sandbox/log" "$sandbox/tray-seen"
    env -i HOME="$sandbox" PATH="$path" STUB_LOG="$sandbox/log" FAKE_TAG="$tag" \
        FAKE_RELEASE="$sandbox/release" INSTALL_ARGS="$sandbox/args" TRAY_SEEN="$sandbox/tray-seen" "$@" \
        sh "$sandbox/get-headroom.sh" "${installer_args[@]}" >"$sandbox/out" 2>&1
}

installed_with_tray() {
    [ "$(cat "$sandbox/args" 2>/dev/null)" = "$(printf '%s\n%s' --no-gnome --tray)" ] \
        && [ "$(cat "$sandbox/tray-seen" 2>/dev/null)" = "$1" ]
}

installed_without_tray() {
    installed && [ ! -e "$sandbox/tray-seen" ]
}

installed() {
    [ "$(cat "$sandbox/args" 2>/dev/null)" = "--no-gnome" ]
}

not_installed() {
    [ ! -e "$sandbox/args" ]
}

output_has() {
    grep -q -- "$1" "$sandbox/out"
}

every_call_is_https_only() {
    local flag="$1"
    [ -s "$sandbox/log" ] && ! grep -v -- "$flag" "$sandbox/log" | grep -q .
}

downloads_use_the_resolved_tag() {
    grep -q "releases/download/$tag/SHA256SUMS$" "$sandbox/log" \
        && grep -q "releases/download/$tag/SHA256SUMS.sig$" "$sandbox/log" \
        && ! grep -q "releases/latest/download" "$sandbox/log"
}

test_pinned_key() {
    local pinned
    pinned="$(sed -n '/^release_public_key=/,/END PUBLIC KEY/p' "$root/packaging/release/get-headroom.sh" \
        | sed -e "s/^release_public_key='//" -e "s/'$//")"
    check "get-headroom.sh pins the published release key" \
        [ "$pinned" = "$(cat "$root/packaging/release/release-signing-key.pub.pem")" ]
}

refuses_to_sign() {
    ! RELEASE_SIGNING_KEY="$1" "$root/packaging/release/sign-sums.sh" \
        "$sandbox/release/SHA256SUMS" "$sandbox/release.pub.pem" >/dev/null 2>&1
}

test_sign_sums() {
    sign_release "$sandbox/release.pem"
    openssl base64 -d -A -in "$sandbox/release/SHA256SUMS.sig" -out "$sandbox/signature.raw"
    check "sign-sums.sh writes a signature that openssl verifies" openssl pkeyutl -verify -rawin -pubin \
        -inkey "$sandbox/release.pub.pem" -in "$sandbox/release/SHA256SUMS" \
        -sigfile "$sandbox/signature.raw" -out /dev/null
    check "sign-sums.sh refuses a key that does not match the pinned one" \
        refuses_to_sign "$(cat "$sandbox/other.pem")"
    check "sign-sums.sh refuses an empty secret" refuses_to_sign ""
}

test_signed_install() {
    sign_release "$sandbox/release.pem"
    run_installer "$sandbox/path-curl" || true
    check "a signed release installs with curl" installed
    check "the signature is verified" output_has "Signature verified"
    check "every curl call is https-only" every_call_is_https_only "--proto =https"
    check "files come from the resolved tag" downloads_use_the_resolved_tag
    run_installer "$sandbox/path-wget" || true
    check "a signed release installs with wget" installed
    check "every wget call is https-only" every_call_is_https_only "--https-only"
    check "wget downloads use the resolved tag" downloads_use_the_resolved_tag
}

test_rejected_signatures() {
    forge_signature
    run_installer "$sandbox/path-curl" && flunk "a forged signature must fail" || true
    check "a signature by another key stops the install" not_installed
    check "the forgery is reported" output_has "not signed by the Headroom release key"
    rm -f "$sandbox/release/SHA256SUMS.sig"
    run_installer "$sandbox/path-curl" && flunk "a missing signature must fail" || true
    check "a missing signature stops the install" not_installed
    check "the missing signature is reported" output_has "has no SHA256SUMS.sig"
}

test_without_openssl() {
    forge_signature
    run_installer "$sandbox/path-bare" || true
    check "without openssl the checksum alone is trusted" installed
    check "without openssl a warning is printed" output_has "signature of SHA256SUMS is not checked"
    printf '%064d  %s\n' 0 "$(bundle).tar.gz" >"$sandbox/release/SHA256SUMS"
    run_installer "$sandbox/path-bare" && flunk "a checksum mismatch must fail" || true
    check "a checksum mismatch still stops the install" not_installed
    make_release
}

test_bad_tag() {
    sign_release "$sandbox/release.pem"
    run_installer "$sandbox/path-curl" HEADROOM_VERSION='v1;rm -rf x' && flunk "a bad tag must fail" || true
    check "a tag with shell characters is refused" output_has "not a release tag"
}

gtk_libraries="libgtk-4.so.1 libadwaita-1.so.0"

run_on_desktop() {
    local desktop="$1" libraries="$2"
    shift 2
    run_installer "$sandbox/path-curl" DISPLAY=:0 XDG_CURRENT_DESKTOP="$desktop" \
        FAKE_LIBRARIES="$libraries" "$@" || true
}

test_tray_selection() {
    sign_release "$sandbox/release.pem"
    run_on_desktop XFCE "$gtk_libraries"
    check "Xfce gets the plain tray" installed_with_tray linux-gnu
    check "the tray tarball's checksum is verified" output_has "Downloading $(tray_bundle linux-gnu).tar.gz"
    run_on_desktop sway "$gtk_libraries libgtk4-layer-shell.so.0"
    check "a desktop with gtk4-layer-shell gets the layer-shell tray" installed_with_tray linux-gnu-layershell
    run_on_desktop Budgie:GNOME "$gtk_libraries"
    check "Budgie gets the tray although it names GNOME" installed_with_tray linux-gnu
    run_on_desktop ubuntu:GNOME "$gtk_libraries"
    check "Ubuntu's GNOME Shell gets no tray" installed_without_tray
    run_on_desktop KDE "$gtk_libraries"
    check "Plasma gets no tray" installed_without_tray
    run_on_desktop XFCE ""
    check "without GTK 4 the tray is skipped" installed_without_tray
    check "the skipped tray is explained" output_has "Skipping the Headroom tray"
    run_installer "$sandbox/path-curl" XDG_CURRENT_DESKTOP=XFCE FAKE_LIBRARIES="$gtk_libraries" || true
    check "without a graphical session there is no tray" installed_without_tray
    mkdir -p "$sandbox/.local/bin"
    install -m 0755 /dev/null "$sandbox/.local/bin/headroom-tray"
    run_installer "$sandbox/path-curl" FAKE_LIBRARIES="$gtk_libraries" || true
    check "an installed tray is kept up to date even outside a session" installed_with_tray linux-gnu
    rm -rf "$sandbox/.local"
    installer_args=(--no-gnome --no-tray)
    run_on_desktop XFCE "$gtk_libraries"
    check "--no-tray keeps the tray out" installed_without_tray
    installer_args=(--tray --no-gnome)
    run_on_desktop ubuntu:GNOME "$gtk_libraries"
    check "--tray installs the tray on any desktop" installed_with_tray linux-gnu
    installer_args=(--no-gnome)
}

test_tampered_tray() {
    local tray
    tray="$(tray_bundle linux-gnu).tar.gz"
    cp "$sandbox/release/$tray" "$sandbox/tray-original"
    printf 'tampered' >>"$sandbox/release/$tray"
    sign_release "$sandbox/release.pem"
    run_on_desktop XFCE "$gtk_libraries"
    check "a tampered tray tarball stops the install" not_installed
    check "the tray checksum mismatch is reported" output_has "checksum mismatch for $tray"
    mv "$sandbox/tray-original" "$sandbox/release/$tray"
}

make_keys
make_release
installer_with_test_key
prepare_paths
test_pinned_key
test_sign_sums
test_signed_install
test_rejected_signatures
test_without_openssl
test_bad_tag
test_tray_selection
test_tampered_tray
if [ "$failures" -gt 0 ]; then
    printf '%d check(s) failed\n' "$failures" >&2
    exit 1
fi
printf 'all release signing checks passed\n'
