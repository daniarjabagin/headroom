cask "headroom" do
  version "0.4.1"
  sha256 :no_check

  url "https://github.com/daniarjabagin/headroom/releases/download/v#{version}/Headroom-#{version}-universal.dmg"
  name "Headroom"
  desc "Menu bar app showing how much of your AI coding limits is left"
  homepage "https://github.com/daniarjabagin/headroom"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates true
  depends_on macos: :sonoma

  app "Headroom.app"

  postflight_steps do
    run "/usr/bin/xattr",
        args:         ["-dr", "com.apple.quarantine", "{{appdir}}/Headroom.app"],
        must_succeed: false
  end

  uninstall quit: "io.github.daniarjabagin.headroom"

  zap trash: [
    "~/Library/Application Support/Headroom",
    "~/Library/Caches/headroom",
    "~/Library/Caches/io.github.daniarjabagin.headroom",
    "~/Library/HTTPStorages/io.github.daniarjabagin.headroom",
    "~/Library/Logs/Headroom",
    "~/Library/Preferences/io.github.daniarjabagin.headroom.plist",
    "~/Library/Saved Application State/io.github.daniarjabagin.headroom.savedState",
  ]

  caveats <<~EOS
    Headroom is ad-hoc signed and not notarized by Apple yet, so this cask removes the
    com.apple.quarantine attribute from Headroom.app after installing it. Without that,
    Gatekeeper would block the first launch.

    Headroom updates itself with Sparkle; every update is verified with an EdDSA signature.
    Because the signature is ad-hoc, macOS asks again for Keychain access to
    "Claude Code-credentials" after each update: choose Always Allow.
  EOS
end
