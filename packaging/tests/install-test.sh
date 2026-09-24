#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
sandbox="$(mktemp -d)"
trap 'rm -rf "$sandbox"' EXIT
failures=0
dbus_file=".local/share/dbus-1/services/io.github.daniarjabagin.Headroom.service"
receipt=".local/share/headroom/install.json"

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

logging_stub() {
    cat <<'EOF'
#!/bin/sh
printf '%s %s\n' "$(basename "$0")" "$*" >>"$STUB_LOG"
EOF
}

write_stubs() {
    local tool
    mkdir -p "$sandbox/stubs"
    for tool in systemctl gnome-extensions cargo; do
        logging_stub >"$sandbox/stubs/$tool"
    done
    printf '#!/bin/sh\necho "headroom 9.9.9"\n' >"$sandbox/headroom"
    chmod +x "$sandbox/stubs/"* "$sandbox/headroom"
}

make_bundle() {
    local bundle="$sandbox/bundle"
    mkdir -p "$bundle/systemd" "$bundle/dbus" "$bundle/icons"
    install -m 0755 "$root/packaging/release/install-from-tarball.sh" "$bundle/install.sh"
    install -m 0755 "$sandbox/headroom" "$bundle/headroom"
    cp "$root/packaging/systemd/headroom.service" "$bundle/systemd/"
    cp "$root/packaging/dbus/io.github.daniarjabagin.Headroom.service" "$bundle/dbus/"
    cp "$root/packaging/icons/headroom.svg" "$root/assets/brand/headroom-symbolic.svg" "$bundle/icons/"
    mkdir -p "$sandbox/target/release"
    install -m 0755 "$sandbox/headroom" "$sandbox/target/release/headroom"
}

run_script() {
    local home="$1" script="$2"
    shift 2
    rm -rf "$home" "$sandbox/log"
    mkdir -p "$home"
    env -i HOME="$home" PATH="$sandbox/stubs:/usr/bin:/bin" STUB_LOG="$sandbox/log" \
        CARGO_TARGET_DIR="$sandbox/target" LC_ALL=C.UTF-8 \
        bash "$script" "$@" >"$sandbox/out" 2>&1
}

exec_argv() {
    python3 - "$1/$dbus_file" <<'EOF'
import shlex, sys
for line in open(sys.argv[1], encoding="utf-8"):
    if line.startswith("Exec="):
        print("\n".join(shlex.split(line[len("Exec="):].rstrip("\n"))))
EOF
}

exec_line_runs_the_installed_binary() {
    [ "$(exec_argv "$1")" = "$(printf '%s\n%s' "$1/.local/bin/headroom" daemon)" ]
}

receipt_field() {
    python3 - "$1/$receipt" "$2" <<'EOF'
import json, sys
value = json.load(open(sys.argv[1], encoding="utf-8"))[sys.argv[2]]
print(json.dumps(value) if isinstance(value, list) else value, end="")
EOF
}

mode_of() {
    stat -c '%a' "$1"
}

systemd_was_started() {
    grep -q "systemctl --user restart headroom.service" "$sandbox/log"
}

test_tarball_install_with_awkward_home() {
    local home="$sandbox/home dir & co|pipe\\slash 'quoted'"
    run_script "$home" "$sandbox/bundle/install.sh" --no-gnome --no-plasma || cat "$sandbox/out" >&2
    check "the binary is installed under an awkward HOME" test -x "$home/.local/bin/headroom"
    check "the D-Bus Exec line names the installed binary" exec_line_runs_the_installed_binary "$home"
    check "the receipt records the prefix" [ "$(receipt_field "$home" prefix)" = "$home/.local" ]
    check "the receipt records the options" \
        [ "$(receipt_field "$home" options)" = '["--no-gnome", "--no-plasma"]' ]
    check "the receipt is private" [ "$(mode_of "$home/$receipt")" = 600 ]
    check "the receipt directory is private" [ "$(mode_of "$(dirname "$home/$receipt")")" = 700 ]
    check "the service is (re)started through systemctl" systemd_was_started
}

test_control_characters_in_home() {
    local home
    home="$sandbox/$(printf 'tab\there "q" \\b')"
    run_script "$home" "$sandbox/bundle/install.sh" --no-service --no-gnome --no-plasma \
        || cat "$sandbox/out" >&2
    check "control characters in HOME keep the receipt valid JSON" \
        [ "$(receipt_field "$home" prefix)" = "$home/.local" ]
    check "control characters are escaped in the receipt" grep -q 'tab\\u0009here \\"q\\" \\\\b' \
        "$home/$receipt"
    if run_script "$home" "$sandbox/bundle/install.sh" --no-gnome --no-plasma; then
        flunk "a D-Bus file for a path with control characters must be refused"
    fi
    check "the D-Bus file is refused for such a path" grep -q "contains control characters" "$sandbox/out"
}

test_source_install_with_awkward_home() {
    local home="$sandbox/src home & co|x\\y"
    run_script "$home" "$root/packaging/install.sh" --no-gnome || cat "$sandbox/out" >&2
    check "the source install writes a working D-Bus Exec line" exec_line_runs_the_installed_binary "$home"
    check "the source install records a source receipt" [ "$(receipt_field "$home" method)" = source ]
    check "the source install receipt is private" [ "$(mode_of "$home/$receipt")" = 600 ]
}

write_stubs
make_bundle
test_tarball_install_with_awkward_home
test_control_characters_in_home
test_source_install_with_awkward_home
if [ "$failures" -gt 0 ]; then
    printf '%d check(s) failed\n' "$failures" >&2
    exit 1
fi
printf 'all install script checks passed\n'
