#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: install-tray.sh [--binary PATH]

Installs the Headroom tray for the current user and (re)starts it in a running graphical session:
  ~/.local/bin/headroom-tray
  ~/.config/autostart/headroom-tray.desktop                          (starts it at login)
  ~/.local/share/applications/io.github.daniarjabagin.HeadroomTray.desktop  (menu entry)

  --binary PATH  the headroom-tray binary to install (default: next to this script)

The main install.sh runs this with --tray; it is not meant to be the only thing you install.
EOF
}

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
binary="$here/headroom-tray"
while [ "$#" -gt 0 ]; do
    case "$1" in
        --binary)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            binary="$2"
            shift 2
            ;;
        -h | --help) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done

bin_dir="$HOME/.local/bin"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
autostart_file="$config_home/autostart/headroom-tray.desktop"
menu_file="$data_home/applications/io.github.daniarjabagin.HeadroomTray.desktop"
target="$bin_dir/headroom-tray"
stop_wait_steps=50

step() {
    printf '==> %s\n' "$*"
}

desktop_exec_quoted() {
    local text="$1" quoted
    quoted="${text//\\/\\\\}"
    quoted="${quoted//\"/\\\"}"
    quoted="${quoted//\`/\\\`}"
    quoted="${quoted//\$/\\\$}"
    quoted="\"$quoted\""
    quoted="${quoted//\\/\\\\}"
    printf '%s' "${quoted//%/%%}"
}

write_desktop_file() {
    local template exec
    template="$(<"$1")"
    exec="$(desktop_exec_quoted "$target")"
    mkdir -p "$(dirname "$2")"
    printf '%s\n' "${template//@EXEC@/"$exec"}" >"$2.new"
    chmod 0644 "$2.new"
    mv -f "$2.new" "$2"
}

tray_running() {
    pgrep -x -u "$(id -u)" headroom-tray >/dev/null 2>&1
}

stop_tray() {
    local waited=0
    pkill -x -u "$(id -u)" headroom-tray >/dev/null 2>&1 || true
    while tray_running && [ "$waited" -lt "$stop_wait_steps" ]; do
        sleep 0.1
        waited=$((waited + 1))
    done
}

start_tray() {
    if command -v setsid >/dev/null 2>&1; then
        setsid -f "$target" </dev/null >/dev/null 2>&1
    else
        nohup "$target" </dev/null >/dev/null 2>&1 &
    fi
}

graphical_session() {
    [ -n "${WAYLAND_DISPLAY:-}" ] || [ -n "${DISPLAY:-}" ]
}

if [ ! -x "$binary" ]; then
    printf 'install-tray.sh: %s is not an executable headroom-tray binary\n' "$binary" >&2
    exit 1
fi
case "$target" in
    *[[:cntrl:]]*)
        printf 'install-tray.sh: %s contains control characters; a desktop entry cannot start it\n' "$target" >&2
        exit 1
        ;;
esac

fresh_install=1
[ -e "$target" ] && fresh_install=0
was_running=0
tray_running && was_running=1

step "Installing $target"
mkdir -p "$bin_dir"
install -m 0755 "$binary" "$bin_dir/.headroom-tray.new"
mv -f "$bin_dir/.headroom-tray.new" "$target"

step "Installing the tray's autostart and menu entries"
write_desktop_file "$here/headroom-tray.desktop" "$autostart_file"
write_desktop_file "$here/io.github.daniarjabagin.HeadroomTray.desktop" "$menu_file"

if ! graphical_session; then
    printf 'The Headroom tray starts at your next login.\n'
elif [ "$was_running" -eq 1 ] || [ "$fresh_install" -eq 1 ]; then
    step "Starting headroom-tray"
    [ "$was_running" -eq 1 ] && stop_tray
    start_tray
fi
