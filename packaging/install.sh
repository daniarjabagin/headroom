#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: packaging/install.sh [--no-service] [--no-gnome]

Builds Headroom and installs it for the current user:
  ~/.local/bin/headroom
  ~/.local/share/icons/hicolor/{scalable,symbolic}/apps/headroom*.svg
  ~/.config/systemd/user/headroom.service   (enabled and started)
  ~/.local/share/dbus-1/services/io.github.headroom.Daemon.service
  the GNOME Shell extension, when GNOME Shell is installed

  --no-service  install the binary only; skip systemd and D-Bus activation
  --no-gnome    skip the GNOME Shell extension
EOF
}

install_service=1
install_gnome=1
for arg in "$@"; do
    case "$arg" in
        --no-service) install_service=0 ;;
        --no-gnome) install_gnome=0 ;;
        -h | --help) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin_dir="$HOME/.local/bin"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
unit_dir="$config_home/systemd/user"
dbus_dir="$data_home/dbus-1/services"
icon_dir="$data_home/icons/hicolor"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
extension_uuid="headroom@headroom.github.io"

step() {
    printf '==> %s\n' "$*"
}

build_binary() {
    step "Building headroom (release)"
    cargo build --release --manifest-path "$root/Cargo.toml" -p headroom
}

install_binary() {
    step "Installing $bin_dir/headroom"
    mkdir -p "$bin_dir"
    install -m 0755 "$target_dir/release/headroom" "$bin_dir/.headroom.new"
    mv -f "$bin_dir/.headroom.new" "$bin_dir/headroom"
}

install_icons() {
    step "Installing the app icons into $icon_dir"
    install -D -m 0644 "$root/packaging/icons/headroom.svg" "$icon_dir/scalable/apps/headroom.svg"
    install -D -m 0644 "$root/assets/brand/headroom-symbolic.svg" "$icon_dir/symbolic/apps/headroom-symbolic.svg"
}

install_units() {
    step "Installing the systemd user unit and D-Bus activation file"
    mkdir -p "$unit_dir" "$dbus_dir"
    install -m 0644 "$root/packaging/systemd/headroom.service" "$unit_dir/headroom.service"
    sed "s|@BINDIR@|$bin_dir|g" "$root/packaging/dbus/io.github.headroom.Daemon.service" \
        >"$dbus_dir/io.github.headroom.Daemon.service"
    chmod 0644 "$dbus_dir/io.github.headroom.Daemon.service"
}

start_service() {
    step "Enabling and (re)starting headroom.service"
    systemctl --user daemon-reload
    systemctl --user enable headroom.service
    systemctl --user restart headroom.service
}

install_extension() {
    step "Building and installing the GNOME Shell extension"
    make -C "$root/shell/gnome" install
    cat <<EOF

GNOME Shell extension installed. Enable it with:
  gnome-extensions enable $extension_uuid
On Wayland, GNOME Shell only sees newly installed extensions after you log out and back in.
EOF
}

build_binary
install_binary
install_icons
if [ "$install_service" -eq 1 ]; then
    install_units
    start_service
fi
if [ "$install_gnome" -eq 1 ] && command -v gnome-shell >/dev/null 2>&1; then
    install_extension
fi

case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) printf '\nNote: %s is not on your PATH.\n' "$bin_dir" ;;
esac
printf '\nDone. Try: headroom status\n'
