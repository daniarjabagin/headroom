# Changelog

All notable changes to Headroom are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Headroom follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Welcome window on the first launch on macOS.** It shows where Headroom lives in the menu bar and
  what it shows, offers **Open at login** (on by default, applied when you continue; errors appear in
  place), explains that Claude Code and Codex CLI accounts are found automatically and others are
  added in Settings → Accounts, and leads to the popup or to Settings. It appears only once. The
  Homebrew cask's caveats now say to start the app with `open -a Headroom`.
- **DeepSeek provider.** Paste an API key to see the account balance in each currency it holds
  (yuan and dollars shown separately, never added up), with a warning when DeepSeek reports the
  balance cannot pay for API calls.
- **Moonshot API provider** for the Kimi Open Platform (pay-as-you-go, separate from Kimi Code).
  Paste a key from platform.kimi.ai (USD) or the mainland platform (CNY); Headroom finds which one
  it belongs to and shows the balance, vouchers and cash, with a red notice when the balance is used
  up or the account is in debt.
- **Balances in other currencies.** Balances now carry their currency: the state payload has a new
  balance kind `money` (`currency` ISO 4217 code, `micros` in millionths of that currency) and
  `headroom status` prints `¥12.50`, `€3.00` or `12.50 GBP`. Amounts are read from the exact
  decimal text, never through floating point. Existing `usd` balances are unchanged.
- **Combine accounts of the same provider.** The new `display.combine_accounts` setting (off by
  default) makes the daemon publish a `combined` list in the state: for every provider with two or
  more signed-in, visible accounts it sums each window across the accounts (capacity 100 % per
  account, earliest reset, per-account segments) and computes one pace and tone for the total. The
  headline then shows the combined window, and a pin on any grouped account resolves to it. Accounts
  stay in `accounts` unchanged and notifications remain per account.

### Security

- **The Sparkle signing key never reaches a build job.** The release workflow builds and tests the
  macOS DMG in one job without secrets, then signs it and writes `appcast.xml` in a separate job that
  runs no build tools: it checks the DMG's SHA-256, reads the version from the mounted DMG, and uses
  the pinned, checksum-verified `sign_update`. A stable release without the signing key now fails
  instead of shipping without an update feed. `bundle.sh` builds the helper with `cargo --locked`.
- **The GNOME extension bundle is packed without npm.** CI packs the release zip in its own job
  with only GNOME Shell tools installed; eslint, prettier and the unit tests run in a separate job.
- **Dependabot** keeps the pinned GitHub Actions, Cargo crates, the GNOME extension's dev tools and
  the Swift packages current, weekly and with a small limit on open pull requests.
- **Provider requests no longer follow redirects to another origin.** A redirect is followed only
  to the same scheme, host and port, at most five times; anything else fails the request, so API
  keys and refresh tokens in request bodies can never be replayed to a different server.
- **Linux releases are signed.** The release workflow signs `SHA256SUMS` with an Ed25519 key and
  publishes `SHA256SUMS.sig`; a stable release without the key fails. `headroom update` checks the
  signature against the key built into Headroom before it trusts any checksum and refuses a release
  whose signature is missing or wrong. `get-headroom.sh` pins the same key and checks it with
  OpenSSL 1.1.1 or newer; without such an OpenSSL it warns and relies on the checksums alone.
- **`headroom update` downloads only from GitHub over HTTPS.** Its own HTTP client follows
  redirects only to `api.github.com`, `github.com`, `objects.githubusercontent.com` and
  `release-assets.githubusercontent.com`, and caps the size of every response.
- **`get-headroom.sh` resolves the release once** and downloads `SHA256SUMS`, its signature and the
  tarball from that tag, with `wget` limited to HTTPS as `curl` already was.
- **One malformed log line can no longer stop usage tracking.** A record with a timestamp after
  2262 or a token count above 2^63 − 1 is skipped instead of failing the whole import and leaving
  the log position behind, which dropped every later event.
- **The usage database is private.** Its directory is created `0700` and the database, WAL and SHM
  files `0600` whatever the umask; existing installs are tightened on the next start. The install
  receipt is written `0600` as well.
- **Terminal output cannot be hijacked by provider text.** `headroom status` and `headroom accounts`
  drop control characters (including escape sequences) from provider messages, plan names, labels
  and emails, and the daemon rejects account labels that contain control characters.
- **A socket client can run at most 16 commands at once;** the daemon stops reading from that
  connection until one finishes.
- **`install.sh` handles any home directory.** The D-Bus activation file quotes the binary's path,
  so spaces, `&`, `|` and quotes in `$HOME` work, and the install receipt escapes control characters.
- **A server's `Retry-After` can no longer pause a provider for days.** Waits longer than a day are
  ignored, and rate-limit holds are capped at one hour.
- **Grok, Kimi and Cline save a refreshed sign-in even when the fetch is cut short.** The refresh
  and the write run to completion on their own, and each provider's requests fit inside the 30 s
  fetch timeout, so a rotated refresh token is never lost. Kimi now also locks its credentials file
  during a refresh and never overwrites a file that changed meanwhile.
- **The Ollama signing key and the Antigravity CSRF token are redacted in debug output.**
- **Antigravity uses `--extension_server_port` only when the language server owns that port,** so
  another local process can no longer receive its CSRF token.

### Fixed

- **macOS: the login-shell environment is captured even when `.zshrc` starts background jobs.**
  Headroom stops reading at an end marker instead of waiting for every process holding the shell's
  output to exit, and parses `env -0` output, so values with newlines survive. Before, a background
  job made the capture time out and the daemon ran without the login `PATH`.
- **GNOME preferences and the Plasma widget show daemon and provider text as plain text.** Error
  toasts, the update row, the API-key form and the "No providers available" page no longer interpret
  Pango markup, and the Plasma panel labels and tooltips no longer interpret rich text.

### Project

- **Community files:** issue forms for bugs, feature requests and new providers, a pull request
  checklist, a security policy with private vulnerability reporting, a code of conduct and a small
  set of labels, including `good first issue`.

## [0.4.1] - 2026-09-24

### Added

- **Homebrew cask, now the recommended way to install on macOS:**
  `brew install --cask daniarjabagin/tap/headroom`. The cask installs the universal DMG, clears the
  quarantine flag of the ad-hoc signed app so the first launch needs no Gatekeeper steps, leaves
  updates to Sparkle, and `--zap` removes settings, logs and caches. The release workflow updates
  the tap for every release.

### Changed

- **Install docs lead with the recommended way per platform.** On Linux that is the one-line
  installer, with the deb, rpm and Arch packages from the release page and building from source as
  alternatives; on macOS it is Homebrew, with the DMG (and its Gatekeeper steps) and building from
  source as alternatives.

## [0.4.0] - 2026-09-24

The first public release. Headroom now runs on macOS too, ships a ready-made macOS download, keeps
itself up to date and has its own brand.

### Added

- **Native macOS menu-bar app** (macOS 14 Sonoma or newer), built with SwiftUI and AppKit. It shows
  the same popup as on Linux: the spend donut, every limit with its pace forecast, usage trends and
  provider notices. The menu-bar item shows the limit that needs attention most, or one you pin, as
  a percentage or with the provider and limit name.
- **A download for macOS.** Every release carries `Headroom-<version>-universal.dmg` (Apple silicon
  and Intel) with a `.sha256`. The app is ad-hoc signed and not notarized yet; the
  [macOS guide](docs/macos.md#install-from-a-release-dmg) explains the one-time Gatekeeper step.
- **Automatic updates on macOS** with Sparkle 2: a daily check, **Check for Updates…** in the menu,
  and Settings → Service → App updates. Updates are verified with an EdDSA signature.
- **Update notices on Linux.** The daemon asks GitHub once a day whether a newer release exists, and
  the GNOME and Plasma popups show "Headroom X is available". Script installs update with one click
  or `headroom update`, which verifies the download against `SHA256SUMS`; package installs show how
  to get the new package. The checks can be turned off in the settings (`updates.check`) or with
  `headroom daemon --no-update-check`.
- **Brand.** The Headroom logo and mark by Asteru Studio replace the placeholder icons in the panel,
  the Plasma widget, the Linux app icon, notifications and the macOS app icon. The brand assets are
  not covered by the MIT license; see [NOTICE.md](NOTICE.md).
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

- Headroom uses identifiers in its own GitHub namespace: the D-Bus name
  `io.github.daniarjabagin.Headroom`, the GNOME extension `headroom@daniarjabagin.github.io`, the
  Plasma widget `io.github.daniarjabagin.headroom` and the macOS bundle id
  `io.github.daniarjabagin.headroom`. Development installs from before this release are cleaned up
  by `install.sh` and `uninstall.sh`; add API keys again and enable the new GNOME extension after
  logging back in.
- Retry on a signed-out account now shows progress and re-reads the CLI credentials, so signing in
  again in a terminal and pressing Retry is enough.
- The spend donut is shown even when only one provider has spend.
- Hover highlights are smoother and follow the pointer, in both light and dark themes.
- Account discovery logs are quieter.
- Informational provider notes (for example Grok's extra-usage cap) are a quiet line instead of a
  warning banner, and provider notes are translated to Russian in GNOME.
- Grok no longer shows "Extra usage off"; an extra-usage cap is shown in dollars.

### Fixed

- `headroom update --progress json` keeps installing when the window that started it closes.
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

## 0.3.0 - 2026-09-23

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

## 0.2.0 - 2026-09-23

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

## 0.1.0 - 2026-09-23

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

[Unreleased]: https://github.com/daniarjabagin/headroom/compare/v0.4.1...HEAD
[0.4.1]: https://github.com/daniarjabagin/headroom/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/daniarjabagin/headroom/releases/tag/v0.4.0
