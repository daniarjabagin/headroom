#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
sandbox="$(mktemp -d)"
trap 'rm -rf "$sandbox"' EXIT
failures=0
dbus_file=".local/share/dbus-1/services/io.github.daniarjabagin.Headroom.service"
receipt=".local/share/headroom/install.json"
autostart_file=".config/autostart/headroom-tray.desktop"
menu_file=".local/share/applications/io.github.daniarjabagin.HeadroomTray.desktop"
extra_env=()

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
    for tool in systemctl gnome-extensions cargo setsid pkg-config; do
        logging_stub >"$sandbox/stubs/$tool"
    done
    cat >"$sandbox/stubs/pgrep" <<'EOF'
#!/bin/sh
test -e "$TRAY_RUNNING"
EOF
    cat >"$sandbox/stubs/pkill" <<'EOF'
#!/bin/sh
printf 'pkill %s\n' "$*" >>"$STUB_LOG"
rm -f "$TRAY_RUNNING"
EOF
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
    mkdir -p "$sandbox/target/release" "$bundle/tray"
    install -m 0755 "$sandbox/headroom" "$sandbox/target/release/headroom"
    install -m 0755 "$sandbox/headroom" "$sandbox/target/release/headroom-tray"
    install -m 0755 "$sandbox/headroom" "$bundle/tray/headroom-tray"
    install -m 0755 "$root/packaging/tray/install-tray.sh" "$bundle/tray/install-tray.sh"
    cp "$root/packaging/tray/"*.desktop "$bundle/tray/"
    echo linux-gnu-layershell >"$bundle/tray/variant"
}

run_script() {
    rm -rf "$1" "$sandbox/log" "$sandbox/running"
    mkdir -p "$1"
    rerun_script "$@"
}

rerun_script() {
    local home="$1" script="$2"
    shift 2
    rm -f "$sandbox/log"
    env -i HOME="$home" PATH="$sandbox/stubs:/usr/bin:/bin" STUB_LOG="$sandbox/log" \
        TRAY_RUNNING="$sandbox/running" CARGO_TARGET_DIR="$sandbox/target" LC_ALL=C.UTF-8 \
        "${extra_env[@]}" bash "$script" "$@" >"$sandbox/out" 2>&1
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

desktop_exec_argv() {
    python3 - "$1" <<'EOF'
import sys

STRING_ESCAPES = {"s": " ", "n": "\n", "t": "\t", "r": "\r", "\\": "\\"}
QUOTED_ESCAPES = '"`$\\'

def unescape_string(raw):
    value, i = "", 0
    while i < len(raw):
        if raw[i] == "\\" and raw[i + 1:i + 2] in STRING_ESCAPES:
            value += STRING_ESCAPES[raw[i + 1]]
            i += 2
        else:
            value += raw[i]
            i += 1
    return value

def split_exec(value):
    args, current, quoted, i = [], None, False, 0
    while i < len(value):
        char = value[i]
        if quoted and char == "\\" and value[i + 1:i + 2] and value[i + 1] in QUOTED_ESCAPES:
            current, i = (current or "") + value[i + 1], i + 2
            continue
        if char == '"':
            quoted, current = not quoted, current or ""
        elif char == " " and not quoted:
            if current is not None:
                args.append(current)
            current = None
        else:
            current = (current or "") + char
        i += 1
    if current is not None:
        args.append(current)
    return [arg.replace("%%", "%") for arg in args]

for line in open(sys.argv[1], encoding="utf-8"):
    if line.startswith("Exec="):
        print("\n".join(split_exec(unescape_string(line[len("Exec="):].rstrip("\n")))))
        break
EOF
}

tray_entry_runs() {
    [ "$(desktop_exec_argv "$1/$2")" = "$(printf '%s\n%s' "$1/.local/bin/headroom-tray" "$3")" ]
}

logged() {
    grep -qF -- "$1" "$sandbox/log" 2>/dev/null
}

not_logged() {
    ! logged "$1"
}

output_has() {
    grep -qF -- "$1" "$sandbox/out"
}

output_lacks() {
    ! output_has "$1"
}

receipt_has_no_tray() {
    ! grep -q '"tray"' "$1/$receipt"
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
    if run_script "$home" "$sandbox/bundle/install.sh" --no-service --no-gnome --no-plasma --tray; then
        flunk "a tray desktop entry for a path with control characters must be refused"
    fi
    check "the tray's desktop entries are refused for such a path" output_has "a desktop entry cannot start it"
}

test_source_install_with_awkward_home() {
    local home="$sandbox/src home & co|x\\y"
    run_script "$home" "$root/packaging/install.sh" --no-gnome || cat "$sandbox/out" >&2
    check "the source install writes a working D-Bus Exec line" exec_line_runs_the_installed_binary "$home"
    check "the source install records a source receipt" [ "$(receipt_field "$home" method)" = source ]
    check "the source install receipt is private" [ "$(mode_of "$home/$receipt")" = 600 ]
}

test_tarball_install_with_the_tray() {
    local home="$sandbox/tray home & \$HOME \"q\" 50%|x\\y 'z'"
    extra_env=(DISPLAY=:0 XDG_CURRENT_DESKTOP=XFCE)
    run_script "$home" "$sandbox/bundle/install.sh" --no-gnome --no-plasma --tray || cat "$sandbox/out" >&2
    check "the tray binary is installed" test -x "$home/.local/bin/headroom-tray"
    check "the autostart entry starts the installed tray" \
        tray_entry_runs "$home" "$autostart_file" --autostart
    check "the menu entry toggles the installed tray" tray_entry_runs "$home" "$menu_file" --toggle
    check "the receipt records the tray variant" [ "$(receipt_field "$home" tray)" = linux-gnu-layershell ]
    check "the receipt records --tray" \
        [ "$(receipt_field "$home" options)" = '["--no-gnome", "--no-plasma", "--tray"]' ]
    check "a fresh tray starts in the running session" logged "setsid -f $home/.local/bin/headroom-tray"
    check "a fresh tray stops nothing" not_logged pkill
    touch "$sandbox/running"
    rerun_script "$home" "$sandbox/bundle/install.sh" --no-gnome --no-plasma --tray || cat "$sandbox/out" >&2
    check "a running tray is stopped before the new one starts" logged "pkill -x -u $(id -u) headroom-tray"
    check "the updated tray starts again" logged "setsid -f"
    extra_env=(XDG_CURRENT_DESKTOP=XFCE)
    rerun_script "$home" "$sandbox/bundle/install.sh" --no-gnome --no-plasma --tray || cat "$sandbox/out" >&2
    check "without a graphical session the tray waits for the next login" output_has "next login"
    check "without a graphical session nothing is started" not_logged setsid
    rerun_script "$home" "$root/packaging/uninstall.sh" --no-gnome || cat "$sandbox/out" >&2
    check "uninstall removes the tray binary" test ! -e "$home/.local/bin/headroom-tray"
    check "uninstall removes the tray's autostart entry" test ! -e "$home/$autostart_file"
    check "uninstall removes the tray's menu entry" test ! -e "$home/$menu_file"
    check "uninstall stops the tray" logged "pkill -x -u $(id -u) headroom-tray"
    extra_env=()
}

test_tray_needs_its_bundle() {
    local home="$sandbox/no-tray-home"
    mv "$sandbox/bundle/tray" "$sandbox/tray-aside"
    if run_script "$home" "$sandbox/bundle/install.sh" --no-service --no-gnome --no-plasma --tray; then
        flunk "--tray without a tray directory must fail"
    fi
    mv "$sandbox/tray-aside" "$sandbox/bundle/tray"
    check "--tray without the tray bundle says where it is missing" output_has "get-headroom.sh --tray downloads it"
    check "--tray without the tray bundle installs nothing" test ! -e "$home/.local/bin/headroom"
}

test_tray_hint_follows_the_desktop() {
    local home="$sandbox/hint-home"
    extra_env=(DISPLAY=:0 XDG_CURRENT_DESKTOP=Budgie:GNOME)
    run_script "$home" "$sandbox/bundle/install.sh" --no-service --no-gnome --no-plasma || cat "$sandbox/out" >&2
    check "Budgie is told about the tray" output_has "get-headroom.sh --tray"
    check "no tray is installed without --tray" test ! -e "$home/.local/bin/headroom-tray"
    check "the receipt has no tray without --tray" receipt_has_no_tray "$home"
    extra_env=(DISPLAY=:0 XDG_CURRENT_DESKTOP=ubuntu:GNOME)
    run_script "$home" "$sandbox/bundle/install.sh" --no-service --no-gnome --no-plasma || cat "$sandbox/out" >&2
    check "GNOME Shell is not told about the tray" output_lacks "get-headroom.sh --tray"
    extra_env=(WAYLAND_DISPLAY=wayland-0 XDG_CURRENT_DESKTOP=KDE)
    run_script "$home" "$sandbox/bundle/install.sh" --no-service --no-gnome --no-plasma || cat "$sandbox/out" >&2
    check "Plasma is not told about the tray" output_lacks "get-headroom.sh --tray"
    extra_env=()
}

test_source_install_with_the_tray() {
    local home="$sandbox/src tray home"
    extra_env=(WAYLAND_DISPLAY=wayland-0 XDG_CURRENT_DESKTOP=sway)
    run_script "$home" "$root/packaging/install.sh" --no-gnome --tray || cat "$sandbox/out" >&2
    check "the source install builds the tray with gtk4-layer-shell when pkg-config finds it" \
        logged "-p headroom-tray --features layer-shell"
    check "the source install installs the tray" test -x "$home/.local/bin/headroom-tray"
    check "the source install writes the tray's autostart entry" \
        tray_entry_runs "$home" "$autostart_file" --autostart
    extra_env=()
}

write_stubs
make_bundle
test_tarball_install_with_awkward_home
test_control_characters_in_home
test_source_install_with_awkward_home
test_tarball_install_with_the_tray
test_tray_needs_its_bundle
test_tray_hint_follows_the_desktop
test_source_install_with_the_tray
if [ "$failures" -gt 0 ]; then
    printf '%d check(s) failed\n' "$failures" >&2
    exit 1
fi
printf 'all install script checks passed\n'
