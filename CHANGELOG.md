# Changelog

All notable changes to Headroom are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Headroom follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-09-24

Headroom now runs on macOS too.

### Added

- **Native macOS menu-bar app** (macOS 14 Sonoma or newer), built with SwiftUI and AppKit. It shows
  the same popup as on Linux: the spend donut, every limit with its pace forecast, usage trends and
  provider notices. The menu-bar item shows the limit that needs attention most, or one you pin, as
  a percentage or with the provider and limit name.
- A native preferences window with toolbar tabs (General, Accounts, Notifications, Service).
- Add and remove accounts on macOS: CLI sign-ins with the sign-in page, device code and progress
  shown in the window, API keys stored in the Keychain, and a "Detect again" button for apps
  Headroom finds by itself.
- Limit alerts on macOS through Notification Center; clicking one opens the popup.
- Launch at login, light, dark or system theme, an optional translucent popup, and English and
  Russian texts on macOS.
- Unix-socket transport: `headroom daemon --socket` serves the same commands, state and
  notifications as the D-Bus API as line-delimited JSON-RPC, with subscription topics for state,
  alerts and open requests. See [docs/ipc.md](docs/ipc.md). The CLI uses it on macOS.
- macOS credentials: Claude Code and Codex sign-ins are read from the Keychain (read-only), Copilot
  accounts come from `gh`, and Headroom's own API keys are kept in the Keychain.
- The state payload reports the daemon release as `app_version`, so a shell can tell when it is
  talking to a daemon from another install.
- Remove any account from Settings. Accounts found from a CLI sign-in are dismissed instead of
  deleted, so the CLI itself stays signed in; `headroom accounts restore` brings them back.

### Changed

- Retry on a signed-out account now shows progress and re-reads the CLI credentials, so signing in
  again in a terminal and pressing Retry is enough.
- The spend donut is shown even when only one provider has spend.
- Hover highlights are smoother and follow the pointer, in both light and dark themes.
- Account discovery logs are quieter.
- Informational provider notes (for example Grok's extra-usage cap) are a quiet line instead of a
  warning banner, and provider notes are translated to Russian in GNOME.
- Grok no longer shows "Extra usage off"; an extra-usage cap is shown in dollars.

### Fixed

- With the translucent background on GNOME, the popup is fully redrawn while open, and hovering no
  longer leaves dark or light squares behind buttons and rows.
- Translucent hover in Plasma is a light tint instead of an almost opaque patch.
- Refreshing a single account always refreshes it at once and shows it as refreshing.

### Security

- The socket is private to your user: the socket file is `0600`, and a fallback directory is used
  only if it is a real `0700` directory owned by you. A lock file keeps two daemons from sharing one
  socket.
- On macOS, secrets reach the Keychain through `security` on stdin, never on a command line, and
  the app passes API keys to the CLI on stdin as well.

## [0.3.0] - 2026-09-23

### Added

- Twelve new providers, for 14 in total: OpenCode Go, OpenRouter, Z.ai, Kimi Code, MiniMax, Grok,
  Cline, Devin, GitHub Copilot, Cursor, Antigravity and Ollama Cloud.
- API-key accounts. Keys are checked with the provider, stored in the Secret Service keyring (with a
  private `0600` file as fallback) and entered through a hidden prompt or `--api-key-stdin`.
- Add-account flow in the GNOME and Plasma settings: pick a provider, then sign in with its CLI or
  paste a key. Each provider has its own icon and chart color.
- `headroom providers` lists every supported provider and how to add an account.
- Exact cost for tools that log their own (Grok), used instead of the price list.
- Command-line sign-ins that need a terminal (Cline) work from the settings window.
- `headroom refresh --now`, and the popup's refresh button, refresh every account immediately.

### Changed

- Ollama Cloud lists an account only when the local key is linked to an ollama.com account.
- The refresh button is smaller and spins with easing.

### Fixed

- API keys are kept until their account is safely deleted.
- Sign-ins started by Headroom ignore environment variables that would send credentials elsewhere
  or sign in to a different account.
- More robust Antigravity detection and clearer messages for rejected keys.
- Account labels are limited to 64 characters, and errors from the provider list are translated.

### Performance

- Price feeds are filtered while streaming instead of being parsed whole, which lowers memory use
  during price updates.

## [0.2.0] - 2026-09-23

### Added

- Accounts without an active subscription show a clear "subscription inactive" state instead of an
  error.
- Refresh and settings buttons in the popup header.
- The spend donut draws its segments as filled sectors with subtly rounded corners.

### Changed

- Claude Code is shown as "Claude".

### Performance

- Lower idle memory: log files are read line by line, usage homes are read one at a time, the
  runtime uses two worker threads, all providers share one HTTP client and the SQLite cache is
  bounded.
- The release binary shrank from 15.5 MB to 9.7 MB.

## [0.1.0] - 2026-09-23

First version.

### Added

- Codex and Claude Code providers: session, weekly and per-model limits, Codex credits, reset
  times, and exact token counts from local logs, deduplicated by response id.
- Pace forecast on every limit, with an even-pace tick and "at this pace" projections.
- Spend in integer micro-dollars from bundled LiteLLM and models.dev price lists that refresh in the
  background; models without a price mark totals as partial.
- The `headroom` daemon as a systemd user service with a D-Bus API, an SQLite cache, refresh
  scheduling with backoff, and desktop notifications for low, projected-to-run-out and reset limits.
- GNOME Shell extension (GNOME 46 and later) and Plasma 6 widget: a panel ring and a popup with
  limits, spend donut, 30-day trend and top models, animations, skeleton loading, live countdowns and
  an optional translucent popup.
- Preferences in both shells: theme, language, remaining or used values, panel limit, notifications,
  hidden limits, account labels and drag-and-drop ordering.
- Several accounts per provider: `headroom accounts add` signs in to Codex or Claude in a separate
  Headroom-owned home, from the terminal or the settings window.
- Usage is read from every Codex and Claude home with logs, even without a signed-in account.
- CLI: `status` (also `--json`), `refresh`, `accounts` and a `waybar` module.
- English and Russian translations.
- Installer script, a one-line installer, and release builds with static x86_64 and aarch64
  binaries plus `.deb`, `.rpm` and Arch Linux packages.

### Fixed

- Codex account switches are detected, and rate-limit and sign-in errors stay visible.
- Codex auto-review usage is priced by the model it was billed as on that day.
- Zero balances are hidden.
- Settings changes from several windows no longer overwrite each other.
- Cancelling an account sign-in cleans up after itself.

[Unreleased]: https://github.com/OWNER/headroom/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/OWNER/headroom/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/OWNER/headroom/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/OWNER/headroom/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/OWNER/headroom/releases/tag/v0.1.0
