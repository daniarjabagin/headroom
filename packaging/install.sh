#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'EOF'
Usage: packaging/install.sh [--no-service] [--no-gnome]

Builds Headroom and installs it for the current user:
  ~/.local/bin/headroom
  ~/.local/share/icons/hicolor/{scalable,symbolic}/apps/headroom*.svg
  ~/.config/systemd/user/headroom.service   (enabled and started)
  ~/.local/share/dbus-1/services/io.github.daniarjabagin.Headroom.service
  ~/.local/share/headroom/install.json      (marks a source build: `headroom update` leaves it alone)
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
options=("$@")

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
prefix="$HOME/.local"
bin_dir="$prefix/bin"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
unit_dir="$config_home/systemd/user"
dbus_dir="$data_home/dbus-1/services"
icon_dir="$data_home/icons/hicolor"
receipt="$data_home/headroom/install.json"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
extension_uuid="headroom@daniarjabagin.github.io"

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

json_string() {
    local text="$1" out="" char code i
    for ((i = 0; i < ${#text}; i++)); do
        char="${text:i:1}"
        case "$char" in
            '"') out+='\"' ;;
            \\) out+="\\\\" ;;
            [[:cntrl:]])
                printf -v code '\\u%04x' "'$char"
                out+="$code"
                ;;
            *) out+="$char" ;;
        esac
    done
    printf '"%s"' "$out"
}

dbus_quoted() {
    local quote="'\\''"
    printf "'%s'" "${1//\'/"$quote"}"
}

write_receipt() {
    local version_line list="" option
    version_line="$("$bin_dir/headroom" --version)"
    for option in "${options[@]}"; do
        list="${list:+$list,}$(json_string "$option")"
    done
    step "Recording this install in $receipt"
    (
        umask 077
        mkdir -p "$(dirname "$receipt")"
        printf '{"method":"source","version":%s,"options":[%s],"prefix":%s}\n' \
            "$(json_string "${version_line##* }")" "$list" "$(json_string "$prefix")" >"$receipt.new"
    )
    mv -f "$receipt.new" "$receipt"
}

write_dbus_service() {
    local template
    case "$bin_dir" in
        *[[:cntrl:]]*)
            printf 'install.sh: %s contains control characters; D-Bus cannot start it\n' "$bin_dir" >&2
            exit 1
            ;;
    esac
    template="$(<"$1")"
    printf '%s\n' "${template//@EXEC@/"$(dbus_quoted "$bin_dir/headroom")"}" >"$2"
    chmod 0644 "$2"
}

install_units() {
    step "Installing the systemd user unit and D-Bus activation file"
    mkdir -p "$unit_dir" "$dbus_dir"
    install -m 0644 "$root/packaging/systemd/headroom.service" "$unit_dir/headroom.service"
    write_dbus_service "$root/packaging/dbus/io.github.daniarjabagin.Headroom.service" \
        "$dbus_dir/io.github.daniarjabagin.Headroom.service"
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

remove_legacy_install
build_binary
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

case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) printf '\nNote: %s is not on your PATH.\n' "$bin_dir" ;;
esac
printf '\nDone. Try: headroom status\n'
