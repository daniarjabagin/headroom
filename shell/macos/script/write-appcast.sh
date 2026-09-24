#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
app=""
dmg=""
tag=""
sign_update=""
repo_url=""
notes=""
out=""

usage() {
    cat <<'EOF'
Usage: script/write-appcast.sh --app Headroom.app --dmg Headroom-<v>-universal.dmg --tag v<v>
                               --sign-update path/to/sign_update --repo-url https://…/owner/repo
                               --out appcast.xml [--notes release-notes.md]

Signs the DMG with Sparkle's sign_update (EdDSA private key read from the SPARKLE_ED_PRIVATE_KEY
environment variable and passed on stdin), checks the signature against
script/sparkle-public-key.txt and writes a single-item Sparkle appcast for the release.
EOF
}

parse_arguments() {
    while (($# > 0)); do
        (($# >= 2)) || { usage >&2; exit 2; }
        case "$1" in
            --app) app="$2" ;;
            --dmg) dmg="$2" ;;
            --tag) tag="$2" ;;
            --sign-update) sign_update="$2" ;;
            --repo-url) repo_url="$2" ;;
            --notes) notes="$2" ;;
            --out) out="$2" ;;
            *)
                usage >&2
                exit 2
                ;;
        esac
        shift 2
    done
    [[ -n "$app" && -n "$dmg" && -n "$tag" && -n "$sign_update" && -n "$repo_url" && -n "$out" ]] || {
        usage >&2
        exit 2
    }
}

plist_value() {
    /usr/libexec/PlistBuddy -c "Print :$1" "$app/Contents/Info.plist"
}

public_key() {
    tr -d '[:space:]' <"$script_dir/sparkle-public-key.txt"
}

sign_dmg() {
    [[ -n "${SPARKLE_ED_PRIVATE_KEY:-}" ]] || { echo "SPARKLE_ED_PRIVATE_KEY is not set" >&2; exit 1; }
    printf '%s\n' "$SPARKLE_ED_PRIVATE_KEY" | "$sign_update" -p --ed-key-file - "$dmg"
}

verify_signature() {
    local signature="$1"
    swift "$script_dir/verify-ed-signature.swift" "$(public_key)" "$signature" "$dmg"
}

notes_description() {
    [[ -n "$notes" && -s "$notes" ]] || return 0
    printf '      <description sparkle:format="markdown"><![CDATA[%s]]></description>\n' \
        "$(sed 's/]]>/]]]]><![CDATA[>/g' "$notes")"
}

appcast_xml() {
    local version="$1" build="$2" signature="$3" length="$4" published
    published="$(LC_ALL=C date -u '+%a, %d %b %Y %H:%M:%S +0000')"
    cat <<EOF
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>Headroom</title>
    <link>$repo_url</link>
    <description>Headroom for macOS</description>
    <language>en</language>
    <item>
      <title>Headroom $version</title>
      <pubDate>$published</pubDate>
      <sparkle:version>$build</sparkle:version>
      <sparkle:shortVersionString>$version</sparkle:shortVersionString>
      <sparkle:minimumSystemVersion>14.0</sparkle:minimumSystemVersion>
      <sparkle:fullReleaseNotesLink>$repo_url/releases/tag/$tag</sparkle:fullReleaseNotesLink>
$(notes_description)
      <enclosure url="$repo_url/releases/download/$tag/$(basename "$dmg")" length="$length"
                 type="application/octet-stream" sparkle:edSignature="$signature"/>
    </item>
  </channel>
</rss>
EOF
}

main() {
    parse_arguments "$@"
    local version build signature length
    version="$(plist_value CFBundleShortVersionString)"
    build="$(plist_value CFBundleVersion)"
    [[ "v$version" == "$tag" ]] || { echo "app version $version does not match tag $tag" >&2; exit 1; }
    [[ "$build" =~ ^[0-9]+$ ]] || { echo "CFBundleVersion $build is not a build number" >&2; exit 1; }
    signature="$(sign_dmg)"
    verify_signature "$signature"
    length="$(stat -f %z "$dmg")"
    appcast_xml "$version" "$build" "$signature" "$length" >"$out"
    xmllint --noout "$out"
    echo "wrote $out (version $version, build $build, $length bytes)"
}

main "$@"
