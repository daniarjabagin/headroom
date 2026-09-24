#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: packaging/uninstall.sh [--no-gnome]

Stops and removes what install.sh installed, including the Plasma widget. Headroom's data is kept:
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
dbus_file="$data_home/dbus-1/services/io.github.daniarjabagin.Headroom.service"
icon_dir="$data_home/icons/hicolor"
receipt="$data_home/headroom/install.json"
extension_uuid="headroom@daniarjabagin.github.io"
plasmoid_dir="$data_home/plasma/plasmoids/io.github.daniarjabagin.headroom"

step() {
    printf '==> %s\n' "$*"
}

legacy_extension_uuid="headroom@headroom.github.io"
legacy_plasmoid_id="io.github.headroom.plasmoid"
legacy_dbus_file="$data_home/dbus-1/services/io.github.headroom.Daemon.service"

remove_legacy_install() {
    local old_extension_dir="$data_home/gnome-shell/extensions/$legacy_extension_uuid"
    local old_plasmoid_dir="$data_home/plasma/plasmoids/$legacy_plasmoid_id"
    if [ ! -e "$old_extension_dir" ] && [ ! -e "$old_plasmoid_dir" ] && [ ! -e "$legacy_dbus_file" ]; then
        return
    fi
    step "Removing the GNOME extension, Plasma widget and D-Bus file installed under the old names"
    if [ -e "$old_extension_dir" ] && command -v gnome-extensions >/dev/null 2>&1; then
        gnome-extensions disable "$legacy_extension_uuid" >/dev/null 2>&1 || true
    fi
    rm -rf "$old_extension_dir" "$old_plasmoid_dir"
    rm -f "$legacy_dbus_file"
}

remove_legacy_install

if command -v systemctl >/dev/null 2>&1; then
    step "Stopping and disabling headroom.service"
    systemctl --user disable --now headroom.service 2>/dev/null || true
fi

step "Removing the unit, D-Bus activation file, app icons, Plasma widget, install receipt and binary"
rm -f "$unit" "$dbus_file" "$bin_dir/headroom" "$receipt" \
    "$icon_dir/scalable/apps/headroom.svg" "$icon_dir/symbolic/apps/headroom-symbolic.svg"
rm -rf "$plasmoid_dir"

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
