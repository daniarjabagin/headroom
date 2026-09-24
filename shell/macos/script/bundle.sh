#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
package_dir="$(cd "$script_dir/.." && pwd)"
repo_root="$(cd "$package_dir/../.." && pwd)"
dist_dir="$package_dir/dist"
staged_helper="$dist_dir/headroom-daemon"
app="$dist_dir/Headroom.app"
bundle_id="io.github.headroom"
identity="${CODESIGN_IDENTITY:--}"
install=false
open_after=false
universal=false
build_cargo=true
swift_bin_path=""

usage() {
    cat <<'EOF'
Usage: script/bundle.sh [--install] [--open] [--universal] [--no-cargo]

Builds the headroom daemon (cargo) and the menu-bar app (SwiftPM: HeadroomKit, HeadroomUI,
HeadroomSettings, Headroom) in release mode and assembles dist/Headroom.app.

  --install     copy the app to /Applications (quits a running Headroom first)
  --open        launch the app when done
  --universal   build arm64 + x86_64 instead of the native architecture only
  --no-cargo    skip the cargo build and reuse the helper from the previous dist/Headroom.app
                (or the last cargo release build) for UI-only rebuilds

Environment:
  CODESIGN_IDENTITY   signing identity, default "-" (ad-hoc). Use an "Apple Development: …"
                      identity so Keychain "Always Allow" grants survive rebuilds.
EOF
}

parse_arguments() {
    while (($# > 0)); do
        case "$1" in
            --install) install=true ;;
            --open) open_after=true ;;
            --universal) universal=true ;;
            --no-cargo) build_cargo=false ;;
            -h | --help)
                usage
                exit 0
                ;;
            *)
                echo "unknown option: $1" >&2
                usage >&2
                exit 2
                ;;
        esac
        shift
    done
}

workspace_version() {
    awk '
        /^\[workspace\.package\]/ { in_section = 1; next }
        /^\[/ { in_section = 0 }
        in_section && /^version[[:space:]]*=/ {
            gsub(/.*=[[:space:]]*"|".*/, "")
            print
            exit
        }
    ' "$repo_root/Cargo.toml"
}

cargo_target_dir() {
    (cd "$repo_root" && cargo metadata --format-version 1 --no-deps) |
        sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p'
}

build_helper() {
    local target_dir
    target_dir="$(cargo_target_dir)"
    if [[ "$universal" == true ]]; then
        (cd "$repo_root" && cargo build --release -p headroom --target aarch64-apple-darwin)
        (cd "$repo_root" && cargo build --release -p headroom --target x86_64-apple-darwin)
        lipo -create -output "$staged_helper" \
            "$target_dir/aarch64-apple-darwin/release/headroom" \
            "$target_dir/x86_64-apple-darwin/release/headroom"
    else
        (cd "$repo_root" && cargo build --release -p headroom)
        cp "$target_dir/release/headroom" "$staged_helper"
    fi
}

reuse_helper() {
    local previous="$app/Contents/Helpers/headroom" built
    if [[ -x "$previous" ]]; then
        cp "$previous" "$staged_helper"
        return
    fi
    built="$(cargo_target_dir)/release/headroom"
    if [[ ! -x "$built" ]]; then
        echo "--no-cargo: no helper at $previous or $built; run once without --no-cargo" >&2
        exit 1
    fi
    cp "$built" "$staged_helper"
}

swift_arguments() {
    local arguments=(-c release --package-path "$package_dir" --product Headroom)
    if [[ "$universal" == true ]]; then
        arguments+=(--arch arm64 --arch x86_64)
    fi
    printf '%s\n' "${arguments[@]}"
}

build_app_binary() {
    local arguments=()
    while IFS= read -r argument; do arguments+=("$argument"); done < <(swift_arguments)
    swift build "${arguments[@]}"
    swift_bin_path="$(swift build "${arguments[@]}" --show-bin-path)"
    cp "$swift_bin_path/Headroom" "$dist_dir/Headroom"
}

copy_resource_bundles() {
    local bundle copied=0
    for bundle in "$swift_bin_path"/*.bundle; do
        [[ -d "$bundle" ]] || continue
        [[ "$(basename "$bundle")" == *Tests.bundle ]] && continue
        cp -R "$bundle" "$app/Contents/Resources/"
        echo "resources: $(basename "$bundle")"
        copied=$((copied + 1))
    done
    if ((copied == 0)); then
        echo "no SwiftPM resource bundles found in $swift_bin_path" >&2
        exit 1
    fi
}

write_info_plist() {
    local version="$1"
    cat >"$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key><string>en</string>
    <key>CFBundleLocalizations</key><array><string>en</string><string>ru</string></array>
    <key>CFBundleDisplayName</key><string>Headroom</string>
    <key>CFBundleExecutable</key><string>Headroom</string>
    <key>CFBundleIdentifier</key><string>$bundle_id</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleName</key><string>Headroom</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
    <key>LSMinimumSystemVersion</key><string>14.0</string>
    <key>LSUIElement</key><true/>
    <key>NSHumanReadableCopyright</key><string>© 2026 Daniar Jabagin</string>
</dict>
</plist>
EOF
    plutil -lint "$app/Contents/Info.plist" >/dev/null
}

build_icon() {
    iconutil -c icns -o "$app/Contents/Resources/AppIcon.icns" "$package_dir/Icon/AppIcon.iconset"
}

assemble() {
    local version="$1"
    rm -rf "$app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Helpers" "$app/Contents/Resources"
    mv "$dist_dir/Headroom" "$app/Contents/MacOS/Headroom"
    mv "$staged_helper" "$app/Contents/Helpers/headroom"
    copy_resource_bundles
    build_icon
    write_info_plist "$version"
}

sign() {
    codesign --force --timestamp=none --sign "$identity" --identifier "$bundle_id.helper" \
        "$app/Contents/Helpers/headroom"
    codesign --force --timestamp=none --sign "$identity" --identifier "$bundle_id" "$app"
    codesign --verify --strict --verbose=1 "$app"
}

install_app() {
    osascript -e "tell application id \"$bundle_id\" to quit" >/dev/null 2>&1 || true
    sleep 1
    rm -rf /Applications/Headroom.app
    ditto "$app" /Applications/Headroom.app
    app="/Applications/Headroom.app"
    echo "installed $app"
}

main() {
    parse_arguments "$@"
    local version
    version="$(workspace_version)"
    [[ -n "$version" ]] || { echo "cannot read the workspace version from Cargo.toml" >&2; exit 1; }
    mkdir -p "$dist_dir"
    if [[ "$build_cargo" == true ]]; then build_helper; else reuse_helper; fi
    build_app_binary
    assemble "$version"
    sign
    echo "built $app ($version, signed with '$identity')"
    if [[ "$install" == true ]]; then install_app; fi
    if [[ "$open_after" == true ]]; then open "$app"; fi
}

main "$@"
