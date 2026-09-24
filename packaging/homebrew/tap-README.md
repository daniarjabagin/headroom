# Homebrew tap for Headroom

[Headroom](https://github.com/daniarjabagin/headroom) shows how much of your AI coding limits is
left (Codex, Claude Code and more) in the macOS menu bar and the Linux top panel.

## Install

```sh
brew install --cask daniarjabagin/tap/headroom
```

Requires macOS 14 Sonoma or newer, Apple silicon or Intel.

Headroom is ad-hoc signed and not notarized by Apple yet. The cask removes the quarantine
attribute from `Headroom.app` after installing, so Gatekeeper does not block the first launch.

## Updates

Headroom updates itself with Sparkle (menu-bar item → right-click → **Check for Updates…**), and
every update is verified with an EdDSA signature. `brew upgrade --cask --greedy headroom` also
works.

## Uninstall

```sh
brew uninstall --cask headroom          # remove the app
brew uninstall --cask --zap headroom    # also remove settings, logs and caches
```

API keys you added stay in the Keychain under `io.github.daniarjabagin.headroom` until you delete
them in Keychain Access.

## About this tap

`Casks/headroom.rb` is updated automatically by the Headroom release workflow for every release.
Report problems in the [Headroom repository](https://github.com/daniarjabagin/headroom/issues).
