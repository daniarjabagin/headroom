#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
package_dir="$(cd "$script_dir/.." && pwd)"
repo_root="$(cd "$package_dir/../.." && pwd)"
dist_dir="$package_dir/dist"
staged_helper="$dist_dir/headroom-daemon"
app="$dist_dir/Headroom.app"
bundle_id="io.github.daniarjabagin.headroom"
identity="${CODESIGN_IDENTITY:--}"
sparkle_feed_url="https://github.com/daniarjabagin/headroom/releases/latest/download/appcast.xml"
install=false
open_after=false
universal=false
build_cargo=true
make_dmg=false
swift_bin_path=""

usage() {
    cat <<'EOF'
Usage: script/bundle.sh [--install] [--open] [--universal] [--no-cargo] [--dmg]

Builds the headroom daemon (cargo) and the menu-bar app (SwiftPM: HeadroomKit, HeadroomUI,
HeadroomSettings, Headroom) in release mode and assembles dist/Headroom.app with Sparkle.framework
and the update feed keys (public EdDSA key from script/sparkle-public-key.txt).

  --install     copy the app to /Applications (quits a running Headroom first)
  --open        launch the app when done
  --universal   build arm64 + x86_64 instead of the native architecture only
  --no-cargo    skip the cargo build and reuse the helper from the previous dist/Headroom.app
                (or the last cargo release build) for UI-only rebuilds
  --dmg         also create dist/Headroom-<version>-<arch>.dmg (arch: native or universal)
                with a drag-to-Applications layout and its .sha256

Environment:
  CODESIGN_IDENTITY   signing identity, default "-" (ad-hoc). Use an "Apple Development: …"
                      identity so Keychain "Always Allow" grants survive rebuilds.
                      The DMG is signed only when an identity other than "-" is set.
EOF
}

parse_arguments() {
    while (($# > 0)); do
        case "$1" in
            --install) install=true ;;
            --open) open_after=true ;;
            --universal) universal=true ;;
            --no-cargo) build_cargo=false ;;
            --dmg) make_dmg=true ;;
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

bundle_build_number() {
    local version="$1" major minor patch
    if [[ ! "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-.*)?$ ]]; then
        echo "version $version is not semver" >&2
        return 1
    fi
    major="${BASH_REMATCH[1]}" minor="${BASH_REMATCH[2]}" patch="${BASH_REMATCH[3]}"
    if ((10#$minor > 99 || 10#$patch > 99)); then
        echo "version $version: minor and patch must be below 100 for CFBundleVersion" >&2
        return 1
    fi
    echo $((10#$major * 10000 + 10#$minor * 100 + 10#$patch))
}

sparkle_public_key() {
    local key
    key="$(tr -d '[:space:]' <"$script_dir/sparkle-public-key.txt")"
    if [[ ! "$key" =~ ^[A-Za-z0-9+/]{43}=$ ]]; then
        echo "script/sparkle-public-key.txt does not hold a base64 Ed25519 public key" >&2
        return 1
    fi
    echo "$key"
}

sparkle_plist_entries() {
    local key
    key="$(sparkle_public_key)"
    cat <<EOF
    <key>SUFeedURL</key><string>$sparkle_feed_url</string>
    <key>SUPublicEDKey</key><string>$key</string>
    <key>SUEnableAutomaticChecks</key><true/>
    <key>SUScheduledCheckInterval</key><integer>86400</integer>
EOF
}

write_info_plist() {
    local version="$1" build sparkle_entries
    build="$(bundle_build_number "$version")"
    sparkle_entries="$(sparkle_plist_entries)"
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
    <key>CFBundleVersion</key><string>$build</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
$sparkle_entries
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
    embed_sparkle
    build_icon
    write_info_plist "$version"
}

sparkle_framework_source() {
    local found
    found="$(find "$swift_bin_path" -maxdepth 2 -type d -name Sparkle.framework -print -quit)"
    if [[ -z "$found" ]]; then
        echo "Sparkle.framework not found in $swift_bin_path" >&2
        return 1
    fi
    echo "$found"
}

add_frameworks_rpath() {
    local binary="$app/Contents/MacOS/Headroom" rpath="@executable_path/../Frameworks"
    if otool -l "$binary" | grep -F "path $rpath (" >/dev/null; then return 0; fi
    install_name_tool -add_rpath "$rpath" "$binary"
}

embed_sparkle() {
    local source framework="$app/Contents/Frameworks/Sparkle.framework"
    source="$(sparkle_framework_source)"
    mkdir -p "$app/Contents/Frameworks"
    ditto "$source" "$framework"
    rm -rf "$framework/XPCServices" "$framework/Versions/B/XPCServices"
    add_frameworks_rpath
    echo "frameworks: Sparkle.framework (without XPC services, the app is not sandboxed)"
}

sign_sparkle() {
    local framework="$app/Contents/Frameworks/Sparkle.framework"
    codesign --force --timestamp=none --sign "$identity" "$framework/Versions/B/Autoupdate"
    codesign --force --timestamp=none --sign "$identity" "$framework/Versions/B/Updater.app"
    codesign --force --timestamp=none --sign "$identity" "$framework"
}

sign() {
    codesign --force --timestamp=none --sign "$identity" --identifier "$bundle_id.helper" \
        "$app/Contents/Helpers/headroom"
    sign_sparkle
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

dmg_arch() {
    if [[ "$universal" == true ]]; then echo universal; else uname -m; fi
}

stage_dmg_contents() {
    local staging="$1"
    ditto "$app" "$staging/Headroom.app"
    ln -s /Applications "$staging/Applications"
}

hdiutil_create() {
    local volume="$1" staging="$2" dmg="$3" attempt
    for attempt in 1 2 3; do
        if hdiutil create -volname "$volume" -srcfolder "$staging" -fs HFS+ \
            -format UDZO -imagekey zlib-level=9 -ov "$dmg"; then
            return 0
        fi
        echo "hdiutil create failed (attempt $attempt of 3)" >&2
        sleep $((attempt * 5))
    done
    return 1
}

sign_dmg() {
    local dmg="$1"
    [[ "$identity" != "-" ]] || return 0
    codesign --force --timestamp=none --sign "$identity" "$dmg"
    codesign --verify --verbose=1 "$dmg"
}

write_checksum() {
    local dmg="$1" name
    name="$(basename "$dmg")"
    (cd "$(dirname "$dmg")" && shasum -a 256 "$name" >"$name.sha256")
    echo "sha256: $(cat "$dmg.sha256")"
}

create_dmg() {
    local version="$1" dmg staging
    dmg="$dist_dir/Headroom-$version-$(dmg_arch).dmg"
    staging="$(mktemp -d)"
    stage_dmg_contents "$staging"
    rm -f "$dmg" "$dmg.sha256"
    hdiutil_create "Headroom $version" "$staging" "$dmg" || {
        rm -rf "$staging"
        echo "cannot create $dmg" >&2
        exit 1
    }
    rm -rf "$staging"
    hdiutil verify "$dmg" >/dev/null
    sign_dmg "$dmg"
    write_checksum "$dmg"
    echo "built $dmg"
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
    if [[ "$make_dmg" == true ]]; then create_dmg "$version"; fi
    if [[ "$install" == true ]]; then install_app; fi
    if [[ "$open_after" == true ]]; then open "$app"; fi
}

main "$@"
