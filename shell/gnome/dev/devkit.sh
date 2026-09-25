#!/usr/bin/env bash
set -euo pipefail

UUID=headroom@daniarjabagin.github.io
DEV_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

usage() {
    echo "usage: devkit.sh BUNDLE.zip" >&2
    echo "  SCENARIO=full|showcase|showcase-update|combined|providers|several|several-auto|icon|collapsed|live|incident|onboarding|critical|empty|spend-only|offline|single|no_subscription  sample state" >&2
    echo "  HEADLESS=1                         run without a window (virtual monitor)" >&2
    echo "  COLOR_SCHEME=prefer-dark|default   color scheme inside the nested shell" >&2
    exit 2
}

sandbox_env() {
    local sandbox=${HEADROOM_DEVKIT_HOME:-${XDG_RUNTIME_DIR:-/tmp}/headroom-devkit}
    local host_display=${WAYLAND_DISPLAY:-wayland-0}
    [[ $host_display == /* ]] || host_display=${XDG_RUNTIME_DIR}/${host_display}
    export WAYLAND_DISPLAY=$host_display
    export HOME=$sandbox/home
    export XDG_DATA_HOME=$HOME/.local/share
    export XDG_CONFIG_HOME=$HOME/.config
    export XDG_CACHE_HOME=$HOME/.cache
    export XDG_STATE_HOME=$HOME/.local/state
    mkdir -p "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_STATE_HOME"
}

install_bundle() {
    local target=$XDG_DATA_HOME/gnome-shell/extensions/$UUID
    rm -rf "$target"
    mkdir -p "$target"
    python3 -m zipfile -e "$1" "$target"
}

configure_shell() {
    gsettings set org.gnome.shell disable-user-extensions false
    gsettings set org.gnome.shell enabled-extensions "['$UUID']"
    gsettings set org.gnome.shell welcome-dialog-last-shown-version '999'
    gsettings set org.gnome.desktop.interface color-scheme "${COLOR_SCHEME:-default}"
}

run_inner() {
    export PATH="$DEV_DIR:$PATH"
    configure_shell
    python3 "$DEV_DIR/mock-daemon.py" --scenario "${SCENARIO:-full}" --interval 30 &
    local mock=$!
    trap 'kill $mock 2>/dev/null' EXIT
    if [[ -n ${HEADLESS:-} ]]; then
        env -u WAYLAND_DISPLAY -u DISPLAY gnome-shell --headless --virtual-monitor 1280x800 --wayland-display headroom-devkit --no-x11
    else
        gnome-shell --devkit --wayland
    fi
}

main() {
    if [[ ${1:-} == --inner ]]; then
        run_inner
        return
    fi
    [[ $# -eq 1 && -f $1 ]] || usage
    local bundle
    bundle=$(realpath "$1")
    sandbox_env
    install_bundle "$bundle"
    exec dbus-run-session -- "$DEV_DIR/devkit.sh" --inner
}

main "$@"
