#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: ./install.sh [--no-service] [--no-gnome] [--no-plasma]

Installs this Headroom release for the current user:
  ~/.local/bin/headroom
  ~/.local/share/icons/hicolor/{scalable,symbolic}/apps/headroom*.svg
  ~/.config/systemd/user/headroom.service   (enabled and started)
  ~/.local/share/dbus-1/services/io.github.daniarjabagin.Headroom.service
  ~/.local/share/headroom/install.json      (options for `headroom update`)
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
options=("$@")

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
prefix="$HOME/.local"
bin_dir="$prefix/bin"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
unit_dir="$config_home/systemd/user"
dbus_dir="$data_home/dbus-1/services"
icon_dir="$data_home/icons/hicolor"
receipt="$data_home/headroom/install.json"
extension_uuid="headroom@daniarjabagin.github.io"
extension_dir="$data_home/gnome-shell/extensions/$extension_uuid"
plasmoid_id="io.github.daniarjabagin.headroom"
plasmoid_dir="$data_home/plasma/plasmoids/$plasmoid_id"

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

install_binary() {
    step "Installing $bin_dir/headroom"
    mkdir -p "$bin_dir"
    install -m 0755 "$here/headroom" "$bin_dir/.headroom.new"
    mv -f "$bin_dir/.headroom.new" "$bin_dir/headroom"
}

install_icons() {
    step "Installing the app icons into $icon_dir"
    install -D -m 0644 "$here/icons/headroom.svg" "$icon_dir/scalable/apps/headroom.svg"
    install -D -m 0644 "$here/icons/headroom-symbolic.svg" "$icon_dir/symbolic/apps/headroom-symbolic.svg"
}

json_string() {
    local text="$1"
    text="${text//\\/\\\\}"
    text="${text//\"/\\\"}"
    printf '"%s"' "$text"
}

write_receipt() {
    local version_line list="" option
    version_line="$("$bin_dir/headroom" --version)"
    for option in "${options[@]}"; do
        list="${list:+$list,}$(json_string "$option")"
    done
    step "Recording this install in $receipt"
    mkdir -p "$(dirname "$receipt")"
    printf '{"method":"script","version":%s,"options":[%s],"prefix":%s}\n' \
        "$(json_string "${version_line##* }")" "$list" "$(json_string "$prefix")" >"$receipt.new"
    mv -f "$receipt.new" "$receipt"
}

install_units() {
    step "Installing the systemd user unit and D-Bus activation file"
    mkdir -p "$unit_dir" "$dbus_dir"
    install -m 0644 "$here/systemd/headroom.service" "$unit_dir/headroom.service"
    sed "s|@BINDIR@|$bin_dir|g" "$here/dbus/io.github.daniarjabagin.Headroom.service" \
        >"$dbus_dir/io.github.daniarjabagin.Headroom.service"
    chmod 0644 "$dbus_dir/io.github.daniarjabagin.Headroom.service"
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

remove_legacy_install
install_binary
install_icons
write_receipt
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
