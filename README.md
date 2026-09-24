<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/headroom-logo-on-dark.svg">
  <img alt="headroom by asteru studio" src="assets/brand/headroom-logo-on-light.svg" width="318">
</picture>

### Know what's left.

How much of your AI coding limits is left, in your Linux top panel and macOS menu bar.

[![Latest release](https://img.shields.io/github/v/release/daniarjabagin/headroom?label=release&color=A9B5FF&labelColor=151617)](https://github.com/daniarjabagin/headroom/releases/latest)
[![CI](https://github.com/daniarjabagin/headroom/actions/workflows/ci.yml/badge.svg)](https://github.com/daniarjabagin/headroom/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-A9B5FF.svg?labelColor=151617)](LICENSE)
[![GNOME 46–50](https://img.shields.io/badge/GNOME-46%E2%80%9350-4a86cf.svg?logo=gnome&logoColor=white&labelColor=151617)](#requirements)
[![Plasma 6.2+](https://img.shields.io/badge/Plasma-6.2%2B-1d99f3.svg?logo=kde&logoColor=white&labelColor=151617)](#requirements)
[![macOS 14+](https://img.shields.io/badge/macOS-14%2B-F0F0ED.svg?logo=apple&logoColor=white&labelColor=151617)](#macos)

[Install](#install) · [Updates](#updates) · [Quick start](#quick-start) · [Providers](#providers) ·
[Privacy](#privacy) · [How it works](#how-it-works) · [Changelog](CHANGELOG.md)

</div>

<br>

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/popup-dark.png">
    <img src="docs/screenshots/popup-light.png" width="368" alt="The Headroom popup under the GNOME top panel: a spend donut for Claude, Codex and Grok, and Claude, Codex, Copilot and Grok limits with pace ticks, one weekly limit running over pace">
  </picture>
</p>

You are halfway through a refactor and the agent stops: *limit reached, resets in 3 hours.*
Headroom exists so that never comes as a surprise. It sits in your panel or menu bar as a small
ring, tells you at a glance how much is left, and warns you while there is still time to slow down
or switch accounts.

<table>
  <tr>
    <td align="center" valign="top" width="25%">
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/mac-popup-dark.png">
        <img src="docs/screenshots/mac-popup-light.png" alt="The Headroom popup under the macOS menu bar with Claude and Codex limits and a spend donut, Russian interface">
      </picture>
      <br><sub><b>macOS</b> · menu-bar app</sub>
    </td>
    <td align="center" valign="top" width="25%">
      <img src="docs/screenshots/plasma.png" alt="The Headroom widget popup in KDE Plasma, dark theme">
      <br><sub><b>KDE Plasma</b> · panel widget</sub>
    </td>
    <td align="center" valign="top" width="25%">
      <img src="docs/screenshots/gnome-settings.png" alt="Headroom preferences in GNOME: theme, language, popup and panel options">
      <br><sub><b>GNOME</b> · preferences</sub>
    </td>
    <td align="center" valign="top" width="25%">
      <img src="docs/screenshots/mac-settings.png" alt="The Headroom settings window on macOS with General, Accounts, Notifications and Service tabs, Russian interface">
      <br><sub><b>macOS</b> · settings</sub>
    </td>
  </tr>
</table>

<sub>Linux screenshots use sample data. The macOS shots show the Russian interface; English is the
default.</sub>

## Highlights

- **Every limit, one glance.** Session, weekly, per-model and monthly windows, credit balances and
  reset countdowns for 14 providers, with several accounts each.
- **Pace forecast.** Every meter carries an even-pace tick. Headroom projects your burn rate and
  tells you *"At this pace: runs out in 2d 1h"* long before you hit the wall. Color follows pace,
  not just level.
- **Exact token counts.** Local logs of Codex, Claude Code and Grok are read incrementally and
  deduplicated by stable response ids, never by summing streaming chunks. Cache reads, 5-minute and
  1-hour cache writes and reasoning tokens are kept apart as raw integers.
- **Honest spend.** Cost is computed in integer micro-dollars from bundled LiteLLM and models.dev
  price lists that refresh in the background and re-cost history when prices change. Models without
  a price are flagged as *partial*, never guessed. Where a tool logs its own cost (Grok), that exact
  figure is used.
- **Notifications that matter.** *Under 10 % left*, *projected to run out in …*, *limit reset*.
  Sent once per window, remembered across restarts, through your desktop's notifications or macOS
  Notification Center.
- **Native everywhere.** A GNOME Shell extension, a KDE Plasma 6 widget, a GTK 4 tray app for every
  other Linux desktop and a SwiftUI menu-bar app with the same popup: spend donut, limits with pace,
  30-day trend and model breakdown. Plus a CLI and a Waybar module.
- **Stays current.** Linux installs tell you when a new release is out and update in one click;
  the macOS app updates itself with Sparkle.
- **Light and dark, English and Russian.** Both themes are first-class and follow the system;
  motion respects reduced-motion settings.
- **Tiny footprint.** One ≈11 MB binary. On Linux the daemon idles at ≈19 MB RSS on two worker
  threads and watches logs with inotify instead of polling.
- **Private by design.** No telemetry, no accounts, no server of its own. See [Privacy](#privacy).

## Providers

| Provider | What you see | How an account is added |
| --- | --- | --- |
| **Codex** | Session and weekly limits, extra per-model limits, credits, tokens and spend from local logs | Found from the `codex` CLI; more accounts with `headroom accounts add codex` |
| **Claude** | Session, weekly and per-model limits, tokens and spend from local logs | Found from Claude Code config dirs; more with `headroom accounts add claude` |
| **OpenCode** | OpenCode Go session, weekly and monthly limits | Paste an API key, or detected from `opencode auth login` |
| **OpenRouter** | Credit balance, key limit, spend today, this week and this month | Paste an API key |
| **Z.ai** | Session, weekly and monthly limits, web searches | Paste an API key |
| **Kimi Code** | Session, weekly and per-model limits | Paste a Kimi Code API key, or sign in with `kimi` |
| **MiniMax** | Session and weekly Token Plan limits | Paste a Token Plan key |
| **Grok** | Weekly credit usage, extra-usage cap, tokens and exact logged cost | Sign in with `grok`, or detected from `~/.grok` |
| **Cline** | Personal and organization credits | Sign in with `cline`, or detected from `~/.cline` |
| **Devin** | Daily and weekly limits, extra-usage balance | Sign in with `devin`, or detected from the Devin CLI or app |
| **Copilot** | Chat, completions and credit quotas, extra usage | Every account signed in to `gh` is found; more with `headroom accounts add copilot` |
| **Cursor** | Total, Auto, API and request usage, on-demand spend | Detected from the Cursor app or `agent login` (one account) |
| **Antigravity** | Session and weekly limits, Claude model limits | Detected from the running app or `agy` (one account). **Linux only** for now |
| **Ollama Cloud** | Session, weekly and monthly limits, extra usage | Detected from `~/.ollama/id_ed25519` when linked to ollama.com (one account) |
| **Kilo Code** | Credit balance in USD (personal or organization), used-up warning | Sign in with `kilo`, paste a Kilo API key, or detected from `kilo auth login` |
| **Warp** | Monthly credits and reset time, bonus credits | Paste a Warp API key (`wk-…`) |
| **Poe** | Point balance | Paste a Poe API key |
| **DeepSeek** | API balance in each currency the account holds (¥ or $) | Paste an API key |
| **Moonshot API** | Kimi Open Platform balance, vouchers and cash, in $ (platform.kimi.ai) or ¥ (mainland) | Paste an API key |

On macOS, Claude Code and Codex sign-ins are read from the Keychain (read-only, after an "Always
Allow" prompt), Copilot asks `gh`, which keeps its tokens there too, and API keys you add go into
the Keychain. `headroom providers` prints the same list for your build. Missing a provider? The
[provider guide](docs/architecture.md#adding-a-provider) shows how one is added.

## Privacy

- **Local only.** Headroom talks to the providers you use and fetches public price lists (LiteLLM,
  models.dev). It has no telemetry, no analytics and no server of its own.
- **One update check a day.** The Linux daemon asks GitHub's release API once a day whether a newer
  Headroom exists; the macOS app reads the release feed (`appcast.xml`) from GitHub once a day.
  Nothing is sent besides the request itself: no identifiers, accounts, usage or settings. Turn it
  off in the settings (Check for updates on Linux, Settings → Service → App updates on macOS).
- **CLI credentials are read-only.** `~/.codex/auth.json`, `~/.claude/.credentials.json`, the
  Claude Code and Codex Keychain items on macOS and their friends are never written or refreshed.
  Headroom refreshes only the sign-ins it created itself, in its own data directory
  (`~/.local/share/headroom/accounts/` on Linux, `~/Library/Application Support/Headroom/` on
  macOS, mode `0700`).
- **API keys** live in the Secret Service keyring on Linux and in the Keychain on macOS, with a
  `0600` file as fallback when no keyring is available. They never appear in logs, errors, process
  arguments or D-Bus and socket payloads.
- **Shells never see secrets.** Panels and the menu-bar app talk only to the daemon, over the
  session bus or a socket that only your user can open.

## Install

| | Recommended | Alternatives |
| --- | --- | --- |
| **Linux** | [one-line installer](#linux-one-line-installer) | [deb, rpm and Arch packages](#linux-packages), [from source](#linux-from-source) |
| **Other Linux desktops** (Xfce, Cinnamon, MATE, Budgie, LXQt, Hyprland, Sway, niri…) | [one-line installer](#linux-one-line-installer), which adds the [Headroom tray](#linux-other-desktops-headroom-tray) | [tray packages](#linux-packages), [from source](#linux-from-source) |
| **macOS** | [Homebrew](#macos) | [DMG from the release page](#macos-dmg), [from source](docs/macos.md) |

### Linux: one-line installer

The recommended way on every distribution:

```sh
curl -fsSL https://github.com/daniarjabagin/headroom/releases/latest/download/get-headroom.sh | sh
```

It downloads the static binary for x86_64 or aarch64, checks the Ed25519 signature of the
release's `SHA256SUMS` (`SHA256SUMS.sig`, with OpenSSL 1.1.1 or newer; without it the script warns
and relies on the checksums alone), verifies the binary against `SHA256SUMS` and installs into
`~/.local`: the binary, the systemd user service, D-Bus activation,
the GNOME Shell extension and the Plasma widget. No root needed, and Headroom updates itself in one
click.

```sh
# skip parts you do not use
curl -fsSL https://github.com/daniarjabagin/headroom/releases/latest/download/get-headroom.sh | sh -s -- --no-gnome
# flags: --no-gnome, --no-plasma, --no-service (binary only), --tray / --no-tray
# pin a release: HEADROOM_VERSION=v0.4.0
```

On a desktop other than GNOME Shell and Plasma the installer also adds the
[Headroom tray](#linux-other-desktops-headroom-tray) (`--tray` forces it, `--no-tray` skips it).

On Wayland, log out and back in once, then enable the extension:
`gnome-extensions enable headroom@daniarjabagin.github.io`. On Plasma, add the **Headroom** widget
to a panel.

### Linux: other desktops (Headroom tray)

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/tray-dark.png">
    <img src="docs/screenshots/tray-light.png" width="318" alt="The Headroom tray popup: a spend donut and Claude and Codex limits with pace ticks, the same cards as the GNOME and Plasma popups">
  </picture>
</p>

Xfce, Cinnamon, MATE, Budgie, LXQt, Hyprland, Sway, niri and every other desktop with a system tray
get `headroom-tray`: a tray icon with the usage ring and the same popup as GNOME and Plasma. Left
click opens the popup, right click shows Open, Refresh now, Settings… and Quit. The one-line
installer picks it by itself from `XDG_CURRENT_DESKTOP` and installs `~/.local/bin/headroom-tray`,
an autostart entry and a menu entry, and starts it. It is updated together with Headroom.

- It needs GTK ≥ 4.14, libadwaita ≥ 1.5 (Ubuntu 24.04 / Mint 22, Debian 13, Fedora 40 and newer,
  Arch) and a StatusNotifierItem tray host. With `gtk4-layer-shell` installed (Debian 13,
  Ubuntu 25.04+, Fedora, Arch) the installer picks the build that places the popup next to the tray
  on wlroots compositors (Sway, Hyprland, niri, river, labwc, Wayfire); without it the popup opens
  centered there. On X11 it opens at the tray icon.
- Tiling window managers start it from their config: `exec headroom-tray` (Sway, i3) or
  `exec-once = headroom-tray` (Hyprland). Bind `headroom-tray --toggle` to a key to open the popup
  without a tray.
- i3bar, polybar, tint2 and other XEmbed-only trays show it through
  [snixembed](https://git.sr.ht/~steef/snixembed): run `snixembed --fork` before `headroom-tray`.
- Waybar needs its `tray` module; swaybar opens the popup on click but has no right-click menu.

### Linux: packages

Every [release](https://github.com/daniarjabagin/headroom/releases/latest) ships `.deb`, `.rpm` and
Arch Linux `.pkg.tar.zst` packages for x86_64 and aarch64. They install `/usr/bin/headroom`, the
user service, D-Bus activation, the GNOME extension and the Plasma widget system-wide.

```sh
sudo apt install ./headroom_*_amd64.deb          # Debian, Ubuntu
sudo dnf install ./headroom-*.x86_64.rpm         # Fedora (zypper install on openSUSE)
sudo pacman -U ./headroom-*-x86_64.pkg.tar.zst   # Arch Linux
```

For desktops other than GNOME and Plasma add the `headroom-tray` package from the same release: the
`.deb` runs on Ubuntu 24.04 / Mint 22 and newer and Debian 13, the `.rpm` (Fedora 41+) and the Arch
package use gtk4-layer-shell. They install `/usr/bin/headroom-tray`, a menu entry and
`/etc/xdg/autostart/headroom-tray.desktop`, which starts the tray at login everywhere except GNOME
Shell and Plasma.

Then, as your user:

```sh
systemctl --user enable --now headroom.service
gnome-extensions enable headroom@daniarjabagin.github.io   # GNOME; on Plasma, add the Headroom widget
```

Verify any download with `sha256sum -c SHA256SUMS --ignore-missing` or
`gh attestation verify <file> --repo daniarjabagin/headroom`. `SHA256SUMS` itself is signed with the
Headroom release key ([`release-signing-key.pub.pem`](packaging/release/release-signing-key.pub.pem)):

```sh
openssl base64 -d -A -in SHA256SUMS.sig -out SHA256SUMS.sig.raw
openssl pkeyutl -verify -rawin -pubin -inkey release-signing-key.pub.pem -in SHA256SUMS -sigfile SHA256SUMS.sig.raw
```

### Linux: from source

```sh
packaging/install.sh                # binary, user service, D-Bus activation, GNOME extension
packaging/install.sh --no-gnome     # skip the GNOME Shell extension
packaging/install.sh --no-service   # install ~/.local/bin/headroom only
packaging/install.sh --tray         # also build and install the Headroom tray
make -C shell/plasma install        # Plasma widget
```

The script is idempotent, so run it again to upgrade. `packaging/uninstall.sh` (also in every
release tarball) removes everything and keeps your data in `~/.local/share/headroom`,
`~/.local/state/headroom` and `~/.cache/headroom`.

### macOS

The recommended way is the Homebrew cask:

```sh
brew install --cask daniarjabagin/tap/headroom
```

The app is ad-hoc signed and not notarized yet; the cask removes the quarantine flag after
installing, so it opens without the Gatekeeper steps below. `brew uninstall --cask --zap headroom`
removes it together with its settings, logs and caches.

Headroom lives in the menu bar, with no Dock icon: click it for the popup, right-click for Refresh,
Settings, Check for Updates and Quit. Settings → Service turns on launch at login. When macOS asks
for access to "Claude Code-credentials" in the Keychain, choose **Always Allow**; because the app is
ad-hoc signed, macOS asks once more after each update.

#### macOS: DMG

Download `Headroom-<version>-universal.dmg` from the
[latest release](https://github.com/daniarjabagin/headroom/releases/latest), open it and drag
**Headroom** to **Applications**.

Because the app is not notarized, macOS blocks the first launch of a downloaded DMG:

1. Open Headroom once and click **Done** in the warning.
2. Go to System Settings → Privacy & Security and click **Open Anyway**, then confirm with your
   password or Touch ID.

Or remove the quarantine flag in Terminal and open the app normally:

```sh
xattr -dr com.apple.quarantine /Applications/Headroom.app
```

To build the app yourself, see the [macOS guide](docs/macos.md), which also covers files, logs and
troubleshooting.

### Requirements

- Linux on x86_64 or aarch64 with a systemd user session and a D-Bus session bus, Wayland or X11
- A panel: GNOME Shell 46–50, KDE Plasma 6.2+ (live updates on 6.4+, polling before), Waybar, or
  any StatusNotifierItem tray for the Headroom tray (GTK ≥ 4.14, libadwaita ≥ 1.5; optional
  gtk4-layer-shell for popup placement on wlroots compositors)
- Optional: a Secret Service keyring (GNOME Keyring, KWallet) for API keys
- macOS 14 Sonoma or newer, Apple silicon or Intel
- Building from source: Rust 1.98; the GNOME extension also needs `gnome-extensions`,
  `glib-compile-schemas` and `python3`; the macOS app needs Xcode (Swift 6)

## Updates

**Linux.** Once a day the daemon checks for a new release. When there is one, the popup shows
**Headroom X is available**:

- Installed with the one-line installer: press **Update**, or run `headroom update` in a terminal.
  It downloads the release, checks the signature of `SHA256SUMS` against the release key built
  into Headroom, verifies the download against `SHA256SUMS` and reinstalls with the same options
  you chose the first time, the Headroom tray included.
- Installed from a package: **How to update** shows what to do; download the new package from the
  [release page](https://github.com/daniarjabagin/headroom/releases/latest) and install it the same
  way as the first one.
- Built from source: pull and run `packaging/install.sh` again; `headroom update` leaves source
  builds alone.

`headroom update --check` compares your version with the latest release without installing
anything.

**macOS.** The app updates itself with [Sparkle](https://sparkle-project.org). It checks once a day
and offers the new version with its release notes; right-click the menu-bar item → **Check for
Updates…** to check at once. Every update is verified with an EdDSA signature before it is
installed. Installed with Homebrew, the app still updates itself; `brew upgrade --cask --greedy
headroom` works too.

Both checks are a single request to GitHub and can be turned off, see [Privacy](#privacy).

## Quick start

Signed in to `codex` or `claude` already? Headroom finds those accounts by itself. For everything
else, open **Settings → Accounts → Add account…** from the popup, pick a provider and sign in or
paste a key. The same works from a terminal (on macOS the CLI is
`/Applications/Headroom.app/Contents/Helpers/headroom`):

```sh
headroom providers                          # what this build supports and how to add each one
headroom accounts add codex --label work    # runs `codex login` in a separate Headroom-owned home
headroom accounts add openrouter            # asks for the API key without echoing it
headroom accounts add zai --api-key-stdin < key.txt
```

### CLI

```text
$ headroom status
Claude · work@example.com  Max 5x
  Session  ━━━━━━━━━━━━━━━━━━━─   95% left  resets in 4h 35m
  Weekly   ━━━━━━━━━━━━━──┃────   66% left  resets in 5d 7h    limit in 3d 6h

Codex · work@example.com  Pro
  Weekly   ━━━━━━━━━━━━━━━━━━━━   99% left  resets in 5d 22h

Spend  estimated from local logs
  Today          $9.69  9.8M tokens
  Yesterday     $30.46  87M tokens
  30 days      $285.74  290M tokens
```

```sh
headroom status --json               # the full state payload, see docs/dbus-api.md
headroom refresh --now               # refresh every account right away
headroom accounts                    # list accounts with their ids
headroom accounts label ID "Work"    # rename; also: hide, show, order ID1 ID2 …
headroom accounts remove ID          # delete an account Headroom added (never ~/.codex or ~/.claude)
headroom update                      # install the latest release (script installs)
```

Without a running daemon, `status` shows the last cached data.

### Waybar

`headroom waybar` streams one JSON line per change: the headline percentage, a tooltip with every
account and your spend, and a `good`, `warning`, `critical` or `neutral` class.

```jsonc
"custom/headroom": {
    "exec": "headroom waybar",
    "return-type": "json",
    "format": "{text}",
    "tooltip": true
}
```

```css
#custom-headroom.warning  { color: #ffd60a; }
#custom-headroom.critical { color: #ff453a; }
#custom-headroom.neutral  { opacity: 0.6; }
```

### Custom CLI homes

The daemon finds Codex in `$CODEX_HOME` (default `~/.codex`) and Claude Code in
`$CLAUDE_CONFIG_DIR` (default `~/.claude`, plus other Claude config dirs it discovers). If you set
these in your shell, set them for the user service too, in
`~/.config/environment.d/headroom.conf`, then log in again. Logs: `journalctl --user -u headroom -f`.

## How it works

```mermaid
flowchart LR
    P["Provider APIs<br/>local CLI logs"] --> C
    subgraph D["headroom daemon (Rust)"]
        direction TB
        C[collector + scheduler] --> S[(state + SQLite cache)]
        S --> B[D-Bus service]
        S --> U[Unix socket · JSON-RPC]
    end
    B -- "session bus (Linux)" --> G[GNOME Shell extension]
    B -- "session bus (Linux)" --> K[Plasma widget]
    B -- "session bus (Linux)" --> W[CLI · Waybar]
    U -- "socket (macOS)" --> M[macOS menu-bar app · CLI]
```

One core, thin shells. On Linux the daemon runs as a systemd user service and speaks D-Bus; on
macOS the menu-bar app starts it as a bundled helper and talks to it over a Unix socket with the
same JSON. The Rust daemon owns all network access, credentials and files. It refreshes each
account every 5 minutes with jitter, backs off exponentially on errors and honours `Retry-After` on
429s. Pace, tone and totals are computed once, in the daemon; the panels only render them, so every
surface shows the same number on every platform.

- [Architecture](docs/architecture.md): domain model, pacing formula, providers, daemon
- [D-Bus API](docs/dbus-api.md): methods, signals and the JSON state payload
- [Socket API](docs/ipc.md): the same commands and events as line-delimited JSON-RPC
- [macOS app](docs/macos.md): how the app, the helper, the Keychain and Sparkle fit together

## How it compares

Headroom stands on the shoulders of two excellent MIT-licensed projects. They are good choices
too; this is what differs.

| | Headroom | [OpenUsage](https://github.com/robinebers/openusage) | [OpenQuota](https://github.com/deviffyy/OpenQuota) |
| --- | --- | --- | --- |
| Platforms | Linux + macOS 14+ | macOS 15+ | Windows, macOS, Linux |
| Built with | Rust daemon + native shells (GJS, QML, SwiftUI) | Swift, SwiftUI | Tauri 2, Rust, Svelte |
| Lives in | GNOME top panel, Plasma panel, system tray of other Linux desktops, Waybar, macOS menu bar | macOS menu bar | System tray, app window |
| Providers | 14 | 11 | 12 |
| License | MIT | MIT | MIT |

## Roadmap

- [x] GNOME Shell extension and KDE Plasma widget
- [x] 14 providers, several accounts each
- [x] Native macOS menu-bar app on the same Rust core
- [x] Update notices on Linux, Sparkle updates on macOS
- [ ] Signed and notarized macOS build
- [x] Homebrew cask
- [ ] AUR package
- [ ] apt and dnf repositories
- [x] Tray icon with a GTK 4 popup for desktops without GNOME or Plasma
- [ ] Antigravity on macOS
- [ ] More providers

## Contributing

Issues and pull requests are welcome. [CONTRIBUTING.md](CONTRIBUTING.md) explains the architecture,
the engineering rules (no floats for money or tokens, typed errors, small functions, fixture-based
tests) and how to run and test every part, from the daemon to the GNOME extension, the Plasma widget
and the macOS app.

**Community:** ask questions in [Discussions](https://github.com/daniarjabagin/headroom/discussions),
report bugs or request providers with the [issue forms](https://github.com/daniarjabagin/headroom/issues/new/choose),
report vulnerabilities privately as described in [SECURITY.md](SECURITY.md), and follow the
[Code of Conduct](CODE_OF_CONDUCT.md).

## License

The source code is licensed under the [MIT license](LICENSE) © 2026 Daniar Jabagin.

The Headroom name, logo, mark and wordmark are © Asteru Studio / Daniar Jabagin, all rights
reserved, and are not covered by the MIT license; see [NOTICE.md](NOTICE.md) and the
[brand guide](assets/brand/README.md).

## Acknowledgements

- [OpenUsage](https://github.com/robinebers/openusage) by Robin Ebers, the look Headroom's popup is
  modelled on.
- [OpenQuota](https://github.com/deviffyy/OpenQuota), whose provider research informed Headroom's
  data collection and pacing.
- [Sparkle](https://sparkle-project.org) for macOS updates.
- [Simple Icons](https://simpleicons.org) (CC0-1.0) for the provider logos. Logos are trademarks of
  their owners; Headroom is not affiliated with or endorsed by any provider.
- Price data from [LiteLLM](https://github.com/BerriAI/litellm) and [models.dev](https://models.dev).
