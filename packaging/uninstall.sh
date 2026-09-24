#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: packaging/uninstall.sh [--no-gnome]

Stops and removes what install.sh installed. Headroom's data is kept:
  ~/.local/state/headroom      cached limits and usage
  ~/.cache/headroom            price catalog cache
  ~/.local/share/headroom      accounts added with `headroom accounts add`

  --no-gnome  leave the GNOME Shell extension installed
EOF
}

remove_gnome=1
for arg in "$@"; do
    case "$arg" in
        --no-gnome) remove_gnome=0 ;;
        -h | --help) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done

bin_dir="$HOME/.local/bin"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
unit="$config_home/systemd/user/headroom.service"
dbus_file="$data_home/dbus-1/services/io.github.headroom.Daemon.service"
icon_dir="$data_home/icons/hicolor"
extension_uuid="headroom@headroom.github.io"

step() {
    printf '==> %s\n' "$*"
}

if command -v systemctl >/dev/null 2>&1; then
    step "Stopping and disabling headroom.service"
    systemctl --user disable --now headroom.service 2>/dev/null || true
fi

step "Removing the unit, D-Bus activation file, app icons and binary"
rm -f "$unit" "$dbus_file" "$bin_dir/headroom" \
    "$icon_dir/scalable/apps/headroom.svg" "$icon_dir/symbolic/apps/headroom-symbolic.svg"

if command -v systemctl >/dev/null 2>&1; then
    systemctl --user daemon-reload
fi

if [ "$remove_gnome" -eq 1 ] && command -v gnome-extensions >/dev/null 2>&1; then
    if gnome-extensions info "$extension_uuid" >/dev/null 2>&1; then
        step "Uninstalling the GNOME Shell extension"
        gnome-extensions uninstall "$extension_uuid"
    fi
fi

cat <<EOF

Headroom is uninstalled. Your data was kept; remove it with:
  rm -rf "${XDG_STATE_HOME:-$HOME/.local/state}/headroom" "${XDG_CACHE_HOME:-$HOME/.cache}/headroom" "$data_home/headroom"
EOF
