# Changelog

All notable changes to Headroom are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Headroom follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

A forecast that follows how you actually work, usage history for accounts added through Headroom,
one-click CLI sign-in, and redesigned account settings.

### Added

- **Forecast from recent work.** Session-length windows (24 hours or less) are now projected from
  your pace over the latest stretch of active work, measured in active time only, instead of the
  average since the window started. When nothing is running, a card says *"Paused · lasts ≈3 h of
  work"* (how long the rest lasts at your last active pace) instead of counting down to a run-out
  that will not happen; idleness is judged between refreshes, so stale data never looks paused.
  Only accounts whose CLI writes local logs can pause. Weekly and longer windows keep the window
  average. `headroom status` shows the same forecast.
- **Live forecast between refreshes.** While a Claude or Codex account is working, the forecast
  follows the token spend in its local logs within seconds, calibrated against the provider's own
  percent steps. The shown percentages are always the provider's; the estimate only moves the
  forecast, and a later refresh that shows no rise, or usage of models without a price, falls back
  to the percent steps.
- **Usage history for accounts added through Headroom.** A Claude or Codex account added with
  `headroom accounts add` now gets usage history, spend and live refresh from the CLI's own logs
  (`~/.claude`, `~/.codex`) when that CLI is signed in to the same account and is not shown as an
  account of its own. Spend still counts every log once.
- **Sign in from the card.** When a CLI's sign-in expires, the card's **Sign in** opens a terminal
  running the CLI's own login (`claude auth login --claudeai`, `codex login`, `gh auth login`, …)
  through your login shell, so the CLI is found on your usual `PATH` (Terminal on macOS). Headroom
  picks up the new sign-in by itself; **Retry** and copying the command stay. `headroom accounts
  login <id>` now works for CLI accounts too.
- **Show models and projects** (`display.show_breakdown`, on by default) hides or shows the
  breakdown list under the spend ring.
- **Headroom mark in the popup header** when the spend section is hidden.
- **Limit bars shimmer** with a soft sheen in their own color, only while the popup is open and not
  with reduced motion.
- **D-Bus and socket API:** `pace.basis` and `pace.active_left_seconds`; `spend.<period>.models`
  and `models_other` with its `cost_per_mtok_usd_micros`; `display.show_breakdown`;
  `recovery.account_id` for `cli_login`; `refresh.last_attempt_at`. All additive, `version` stays
  `1`. See [docs/dbus-api.md](docs/dbus-api.md).

### Changed

- **Account settings are a list and a details pane**, as on macOS, in GNOME, Plasma and the tray:
  providers and accounts on the left, and the selected account's visibility, star, label, single
  limits, links, order, Sign in again… and Remove… on the right.
- **Collapse unstarred folds every account without a star**; with no starred accounts at all,
  every account folds. Starred accounts never fold. The "N more" row shows a warning mark or a
  tone dot when a folded account needs attention.
- **Rate limits are calm.** When a provider limits requests but Headroom has data, the card keeps
  the last numbers with a quiet "Provider is limiting requests · next try 18:05" line instead of an
  error. Repeated limits back off 5 → 10 → 20 → 40 → 60 minutes (or the provider's Retry-After),
  manual Retry is spaced at least a minute apart, and Claude's usage is polled at most every
  3 minutes because Claude Code polls the same endpoint.
- **Models breakdown and the "Other" row come from the daemon**, including the Other row's cost per
  million tokens, so every desktop shows the same numbers.
- "Will run out" and "cutting it close" no longer repeat after every burst of work: once sent, they
  come back only after the window resets or your pace has really calmed down. They are not sent
  while an account is paused.

### Fixed

- **`headroom waybar | head -1` no longer prints "Broken pipe".** Every printing command exits
  quietly when its output is closed.
- **GNOME: cards no longer shift on hover.** The status-page and other hover buttons keep the
  card's height.
- **The number in the Total tokens ring always fits**, in every shell.
- **Kilo Code and Devin CLI sign-ins are picked up automatically**: the credential watcher now
  looks where these CLIs write (under `$XDG_DATA_HOME`), and a CLI account in a custom directory is
  watched there.
- **Copilot sign-in in a terminal** always signs `gh` in to the account's own config dir, even when
  your login shell sets `XDG_CONFIG_HOME`.

## [0.6.0] - 2026-09-25

A panel indicator you can shape yourself, spend by model and project, provider status pages, quiet
hours, a Retry button that actually fixes things, and new `guard`, `spend` and `diagnostics`
commands.

### Fixed

- **GNOME preferences no longer freeze when an option changes.** Changing the refresh interval (or
  any other drop-down) swapped the row's model inside its own selection signal and locked up the
  window. Rows now swap their options only when the options really change, and settings are applied
  on idle. The Plasma option drop-down no longer rebuilds itself while an item is being picked.
- **Retry fixes what it can.** When an account changed or signed out, Retry rescans first, so a new
  CLI sign-in or a different account shows up at once instead of after the next 10-minute scan.
  Accounts signed in through Headroom refresh their own Codex and Claude (Linux) tokens (under a lock,
  re-reading the file first, written atomically); CLI credential files are still never written. A
  failing account watches its credential file, so running `claude` or `codex login` brings the card
  back without a click. The notice stays while the account refreshes, the button shows that it is
  busy and never loses a click, and its action follows the daemon's `recovery`: **Retry**, **Sign in
  again…** (into the same Headroom home, with the new `headroom accounts login`) or **Copy
  command** for the CLI's own login. In GNOME, Plasma, the tray and on macOS.
- **Check for updates on demand.** GNOME and Plasma preferences and the tray's About page show "You
  have the latest version · 0.6.0 · checked just now" with a **Check now** button. Concurrent clicks
  share one request, a check within 60 s reuses the last answer, and GitHub's rate limit and ETag
  are honoured. macOS keeps using Sparkle.

### Added

- **Shape the panel indicator.** Show the most critical limit (as before), **several limits** (up to
  three, pinned or the two most critical, each with its provider logo) or just the **Headroom mark**
  tinted by the worst tone. Draw each as a ring, as a mini bar with the even-pace tick or as text
  only, and label it with the percentage, the window name or nothing. The daemon resolves what to
  show (`panel_items`), so every desktop shows the same limits.
  - GNOME: put the indicator on the left, in the centre or on the right of the top bar, or press and
    hold it and drag it to a new place.
  - macOS: several limits become separate menu-bar items that keep their ⌘-drag positions.
  - Plasma: the widget follows the same modes, with a thin fallback on vertical panels and a tooltip
    that lists every limit; place it with Plasma's own panel editing.
  - Headroom tray: the icon draws one or two rings, bars or the mark; the tray host decides its
    position.
  - A critical limit pulses gently in every style (not with reduced motion).
- **Hide numbers while the screen is shared** (on by default). GNOME shows only the mark in the top
  bar and `••` in the popup, with a banner and **Show anyway** until the share ends; project paths
  are masked too. On macOS the popup and menu-bar items are left out of screenshots, recordings and
  screen sharing.
- **Global shortcut to open the popup** (`shortcuts.open`, off by default), set with a capture
  dialog: GNOME key grab, Plasma's widget shortcut, the GlobalShortcuts portal on Wayland or a key
  grab on X11 for the tray, and Carbon hot keys on macOS (no Accessibility permission needed).
- **A richer popup** in GNOME, Plasma, the Headroom tray and on macOS, with the features below.
- **Spend by model and by project.** Hover a legend row for the models behind a provider, or switch
  the breakdown to **Projects** (the working directory Claude Code and Codex logged, with a bar split
  by provider). Periods are Today, Yesterday, **7 days** and 30 days; units are cost, tokens or **cost
  per million tokens** (priced tokens only). Your choice is remembered. Existing Claude and Codex logs
  are read once more to fill in projects, without double counting.
- **Quick links** to each provider's status page, dashboard and usage page, as icons on the account
  header and in a new right-click menu (Refresh, Hide, Share as image…, Copy as text, …).
- **Provider status pages** (off by default, see Privacy in the README). When turned on, the daemon
  reads the public status pages of the providers you have accounts with every 5 minutes: Claude,
  Codex (OpenAI), Copilot (GitHub), Cursor, Devin, Kimi Code and Moonshot API, MiniMax, Kilo Code,
  Warp and Poe. It follows only the components that matter for each provider, so a GitHub Pages
  outage does not flag Copilot, and the account card shows an amber incident or a red outage.
- **Cards on demand.** Star the accounts you always want to see and fold the others into one
  "3 more · Copilot, Grok, Warp ›" row. An account that needs attention (warning, critical, signed
  out or failing) always stays open.
- **Compact density**: one-line metrics, thinner meters and a smaller donut for a popup about 40 %
  shorter.
- **Click to toggle**: click a percentage to switch between left and used, and a reset time to
  switch between the countdown and the exact time.
- **Share as image**: a branded 1200×630 card of an account's limits, without emails, labels or
  paths, copied to the clipboard and saved to `~/Pictures/Headroom`.
- **Adaptive refresh** (on by default): while Claude Code, Codex or Grok is writing usage logs, its
  accounts are checked every minute, and after 10 quiet minutes they return to the normal interval.
  Backoff and rate-limit holds still win, and no account is ever polled more than once a minute. The
  popup footer tells you when an account is live, when the next update is due, or how outdated the
  data is.
- **12- or 24-hour times**, following the desktop clock or locale unless you choose.
- **Notification thresholds per provider.** Choose when "almost out" fires (5, 10, 20 or 30 % left,
  10 % by default) and override it per provider; `0` turns a provider's alerts off.
- **Quiet hours.** Alerts that arrive at night are held (they survive restarts) and delivered as one
  summary when quiet hours end; anything that reset meanwhile is dropped, and critical alerts can
  still come through.
- **Advanced settings**: the daemon's log level (applied without a restart), the log file with Copy
  path and Open folder, **Copy diagnostics** and **Reset all settings…** (accounts are kept). In
  GNOME, Plasma, the tray and on macOS.
- **First run shows what Headroom found**: detected providers with switches, sign-in for tools that
  are installed but signed out, and a pointer to where Headroom lives: a window in GNOME and the
  tray, a banner in the Plasma popup, a new step of the welcome window on macOS. Existing installs never see it.
- **`headroom guard`** exits `0`, `1` or `2` depending on whether the visible limits stay above a
  minimum, for scripts, git hooks and agents (`headroom guard --min 20 --window weekly && …`).
- **`headroom spend`** breaks spend down by model, project, provider or day for 7 or 30 days or any
  date range, as a table or JSON; without a running daemon it reads the database directly.
- **`headroom diagnostics`** prints versions, platform, log settings and account health without
  tokens, emails, labels or account ids, ready to paste into a bug report.
- **`headroom accounts login <id>`** signs an account Headroom added in again, into the same home,
  keeping its id, label and order.
- **Waybar: several providers in one module.** `headroom waybar --providers claude,codex` shows one
  toned value per provider, `--window session|weekly|any` picks the window and `--labels none` drops
  the names; the tooltip lists every window and your spend. Without flags the output is unchanged.
- **D-Bus and socket API:** `CheckForUpdates`, `GetSpend`, `GetDiagnostics` and `ResetSettings`;
  `panel_items`, `panel_tone`, `provider_status`, `update_check`, `accounts[].recovery`,
  `accounts[].refresh`, `accounts[].collapsed`, `spend.last_7_days`, project breakdowns and
  `cost_per_mtok_usd_micros` in the state; `links` in `ListProviders`. All additive, `version` stays
  `1`. See [docs/dbus-api.md](docs/dbus-api.md#06-payload-additions).

### Changed

- **The daemon writes its own log file**, capped at about 4 MiB and private (`0600`):
  `~/.local/state/headroom/headroom.log` on Linux and `~/Library/Logs/Headroom/headroom.log` on
  macOS, next to the journal or the app's `daemon.log`. `RUST_LOG` still wins over the log level
  setting.
- **GNOME: the popup opens when the mouse button is released,** because pressing and holding the
  indicator now starts a drag.
- **The footer shows when data was updated** and when the next update is due, in your time format.
- **Settings are regrouped** in every shell: Appearance, Top panel (or Menu bar), Spend, Sections,
  Popup cards, Data refresh and Privacy on General; thresholds and quiet hours on Notifications; a
  new Advanced tab.
- The tray and GNOME update notices and account cards in place instead of rebuilding them on every
  state change, which keeps the popup smooth.
- Update checks now finish within the default D-Bus timeout (20 s at most).

## [0.5.1] - 2026-09-25

Polish for the Headroom tray.

### Fixed

- **Even-pace tick in the tray meters.** It was drawn as a faint dot; it is now a clear 2 px tick
  the full 9 px height of the meter, above the fill, in light and dark, as in GNOME. Combined
  (segmented) meters now show each account's tick too.
- **Provider logos in the tray** for Warp, Poe, DeepSeek and Moonshot; Kilo Code keeps the generic
  provider icon as in GNOME and Plasma. Their spend colors come from the shared design tokens.
- **Balance labels in the tray** are shown as the provider names them (Kilo Code's "Credit balance"
  appeared as "Credits"), and Russian uses the same translations as GNOME, including the notices of
  Kilo Code, Warp, DeepSeek and Moonshot.
- **The tray no longer rejects a state without a headline account**, and the GNOME development mock
  sends the combined headline with the first account's id like the daemon.

## [0.5.0] - 2026-09-24

A tray for every other Linux desktop, five new providers, combined accounts, balances in any
currency, a welcome window on macOS and a full security review.

### Added

- **Headroom tray for every other Linux desktop.** `headroom-tray` puts the usage ring in the system
  tray of Xfce, Cinnamon, MATE, Budgie, LXQt, Hyprland, Sway, niri and any other StatusNotifierItem
  host (XEmbed-only trays through `snixembed`) and opens the same popup as GNOME and Plasma, with a
  settings window for display options, accounts and notifications. The one-line installer adds it
  on those desktops (`--tray` / `--no-tray` to choose), with an autostart and a menu entry, and
  `headroom update` keeps it current. Releases ship tray tarballs and `headroom-tray` deb, rpm and
  Arch packages; the deb runs on Ubuntu 24.04 / Mint 22 and newer, the rpm, Arch package and
  `-layershell` tarball use gtk4-layer-shell to place the popup on wlroots compositors.
- **Welcome window on the first launch on macOS.** It shows where Headroom lives in the menu bar and
  what it shows, offers **Open at login** (on by default, applied when you continue; errors appear in
  place), explains that Claude Code and Codex CLI accounts are found automatically and others are
  added in Settings → Accounts, and leads to the popup or to Settings. It appears only once. The
  Homebrew cask's caveats now say to start the app with `open -a Headroom`.
- **Kilo Code, Warp and Poe providers.** Kilo Code shows the credit balance in exact USD (personal,
  or the organization chosen at sign-in) and warns when credits are used up; accounts come from
  `kilo auth login` (found automatically or signed in through `headroom accounts add kilo` into
  Headroom's own directory) or a pasted Kilo API key. Warp shows monthly credits used of the limit
  with the reset time, or "Unlimited", plus bonus credits; Poe shows the point balance. Both take a
  pasted API key. Test fixtures follow the documented and open-source response shapes; no real
  response has been captured yet.
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

[Unreleased]: https://github.com/daniarjabagin/headroom/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/daniarjabagin/headroom/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/daniarjabagin/headroom/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/daniarjabagin/headroom/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/daniarjabagin/headroom/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/daniarjabagin/headroom/releases/tag/v0.4.0
