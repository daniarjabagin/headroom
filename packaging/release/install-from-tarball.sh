#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: ./install.sh [--no-service] [--no-gnome] [--no-plasma]

Installs this Headroom release for the current user:
  ~/.local/bin/headroom
  ~/.config/systemd/user/headroom.service   (enabled and started)
  ~/.local/share/dbus-1/services/io.github.headroom.Daemon.service
  the GNOME Shell extension, when GNOME Shell is installed
  the Plasma widget, when KDE Plasma is installed

  --no-service  install the binary only; skip systemd and D-Bus activation
  --no-gnome    skip the GNOME Shell extension
  --no-plasma   skip the Plasma widget
EOF
}

install_service=1
install_gnome=1
install_plasma=1
for arg in "$@"; do
    case "$arg" in
        --no-service) install_service=0 ;;
        --no-gnome) install_gnome=0 ;;
        --no-plasma) install_plasma=0 ;;
        -h | --help) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bin_dir="$HOME/.local/bin"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
unit_dir="$config_home/systemd/user"
dbus_dir="$data_home/dbus-1/services"
extension_uuid="headroom@headroom.github.io"
extension_dir="$data_home/gnome-shell/extensions/$extension_uuid"
plasmoid_id="io.github.headroom.plasmoid"
plasmoid_dir="$data_home/plasma/plasmoids/$plasmoid_id"

step() {
    printf '==> %s\n' "$*"
}

install_binary() {
    step "Installing $bin_dir/headroom"
    mkdir -p "$bin_dir"
    install -m 0755 "$here/headroom" "$bin_dir/.headroom.new"
    mv -f "$bin_dir/.headroom.new" "$bin_dir/headroom"
}

install_units() {
    step "Installing the systemd user unit and D-Bus activation file"
    mkdir -p "$unit_dir" "$dbus_dir"
    install -m 0644 "$here/systemd/headroom.service" "$unit_dir/headroom.service"
    sed "s|@BINDIR@|$bin_dir|g" "$here/dbus/io.github.headroom.Daemon.service" \
        >"$dbus_dir/io.github.headroom.Daemon.service"
    chmod 0644 "$dbus_dir/io.github.headroom.Daemon.service"
}

user_systemd_available() {
    command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1
}

start_service() {
    if ! user_systemd_available; then
        printf 'No systemd user session found; start the daemon with: headroom daemon\n'
        return
    fi
    step "Enabling and (re)starting headroom.service"
    systemctl --user daemon-reload
    systemctl --user enable headroom.service
    systemctl --user restart headroom.service
}

install_extension() {
    step "Installing the GNOME Shell extension"
    gnome-extensions install --force "$here/gnome/$extension_uuid.shell-extension.zip"
    glib-compile-schemas --strict "$extension_dir/schemas"
    cat <<EOF

GNOME Shell extension installed. Enable it with:
  gnome-extensions enable $extension_uuid
On Wayland, GNOME Shell only sees newly installed extensions after you log out and back in.
EOF
}

install_plasmoid() {
    step "Installing the Plasma widget"
    mkdir -p "$(dirname "$plasmoid_dir")"
    rm -rf "$plasmoid_dir.new"
    cp -R "$here/plasma/$plasmoid_id" "$plasmoid_dir.new"
    rm -rf "$plasmoid_dir"
    mv "$plasmoid_dir.new" "$plasmoid_dir"
    printf '\nPlasma widget installed. Add "Headroom" to your panel from the widget explorer.\n'
}

install_binary
if [ "$install_service" -eq 1 ]; then
    install_units
    start_service
fi
if [ "$install_gnome" -eq 1 ] && command -v gnome-shell >/dev/null 2>&1; then
    install_extension
fi
if [ "$install_plasma" -eq 1 ] && command -v plasmashell >/dev/null 2>&1; then
    install_plasmoid
fi

case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) printf '\nNote: %s is not on your PATH.\n' "$bin_dir" ;;
esac
printf '\nDone. Try: headroom status\n'
