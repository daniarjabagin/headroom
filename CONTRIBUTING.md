# Contributing to Headroom

Thanks for helping. Headroom is small on purpose: one Rust core and a few thin native shells. This
page explains how the pieces fit, the rules the code follows and how to run and test each part.

Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## Issues and discussions

- **Bugs, feature requests and new providers:** pick a form on
  [New issue](https://github.com/daniarjabagin/headroom/issues/new/choose). The forms ask for
  exactly what is needed to act on a report, such as `headroom --version`, `headroom status --json`
  and the daemon log; remove emails and never paste tokens.
- **Questions and ideas:** [Discussions](https://github.com/daniarjabagin/headroom/discussions).
- **Vulnerabilities:** privately, as described in [SECURITY.md](SECURITY.md).

For anything larger than a fix, open an issue first so we can agree on the approach before you write
the code. Issues labelled
[`good first issue`](https://github.com/daniarjabagin/headroom/labels/good%20first%20issue) are
small, well-described tasks that need little knowledge of the codebase, a good way in. Issues
labelled [`help wanted`](https://github.com/daniarjabagin/headroom/labels/help%20wanted) are ones
where a pull request is especially welcome. Leave a comment before you start so no one else works on
the same thing.

## Architecture

One core, many thin shells. All logic lives in the Rust daemon; frontends only render its state and
send commands.

```
┌──────────────────────── headroom (Rust) ─────────────────────────┐
│ providers ─► collector ─► state store ─► IPC ─────────► shells   │
│ (codex, claude, …)  scheduler, backoff,   cache on disk,         │
│ auth, local logs,   pricing, pacing       notifications          │
└──────────────────────────────────────────────────────────────────┘
        ▲ D-Bus (Linux session bus)                ▲ Unix socket (macOS)
┌───────┴────────┐ ┌──────────────┐ ┌──────────────┐ ┌─────┴────────┐
│ GNOME Shell    │ │ KDE Plasma 6 │ │ CLI /        │ │ macOS app    │
│ extension (GJS)│ │ plasmoid(QML)│ │ Waybar JSON  │ │ (SwiftUI)    │
└────────────────┘ └──────────────┘ └──────────────┘ └──────────────┘
```

- `headroom daemon` runs as a systemd user service on Linux and as a child process of the macOS app
  (bundled in `Headroom.app/Contents/Helpers`). It owns all network access, credentials and files.
- The IPC is D-Bus on Linux ([docs/dbus-api.md](docs/dbus-api.md)) and line-delimited JSON-RPC over
  a Unix socket on macOS ([docs/ipc.md](docs/ipc.md), also available on Linux with
  `headroom daemon --socket`). Both carry the same JSON.
- Shells never read provider credentials or call provider APIs. They talk only to the daemon.
- Every number shown to the user comes from exactly one place: the daemon's state model. Shells
  never recompute pace, tone or totals.

[docs/architecture.md](docs/architecture.md) is the detailed contract: domain model, pacing formula,
providers, storage and transports.

## Repository layout

```
Cargo.toml                 workspace root, shared lints and dependency versions
crates/
  headroom-core/           domain model, provider trait, pacing, errors — no I/O
  headroom-pricing/        price catalog, model aliases, cost math
  headroom-providers/      one module per provider: auth, client, local logs, mapper
  headroom-daemon/         scheduler, storage, transports (D-Bus, socket), notifications, updates
  headroom/                the single `headroom` binary: CLI subcommands, daemon, waybar
shell/
  gnome/                   GNOME Shell extension (ESM, GNOME 46+)
  plasma/                  Plasma 6 widget (QML)
  macos/                   SwiftPM package: HeadroomKit (models, socket client), menu-bar app
packaging/                 systemd unit, D-Bus activation, icons, nfpm (deb, rpm, Arch), install scripts
assets/                    brand and provider logos
docs/                      architecture, D-Bus and socket API, macOS guide, screenshots
```

Every provider module has the same shape:

```
crates/headroom-providers/src/codex/
  mod.rs          public entry: impl Provider
  auth.rs         credential discovery and token refresh
  client.rs       HTTP calls, raw response types
  local_usage.rs  local log parsing (if the tool writes logs)
  mapper.rs       raw data → headroom-core model
  fixtures/       real, anonymized responses used by tests
```

[Adding a provider](docs/architecture.md#adding-a-provider) walks through a new one step by step.

## Engineering rules

### General

- **No comments in code.** Names, types and small functions carry the intent. The only exceptions
  are license headers where legally required, `// SAFETY:` on `unsafe` blocks and tool directives
  (`#[allow(…, reason = "…")]`, eslint pragmas). Public API of library crates may have one-line
  `///` docs when the name cannot carry the meaning.
- No dead code, commented-out code, TODO/FIXME or placeholder implementations.
- No speculative abstraction: add a trait, generic or config option only when there are two real
  users.
- Small units: functions around 40 lines at most, files around 400 lines. Split by responsibility,
  not by size alone.
- One responsibility per module. Pure logic (parsing, mapping, pacing, pricing) is separate from I/O
  so it can be unit-tested with fixtures.
- **Correctness of numbers is the product.** Never estimate, round early or silently drop data. Keep
  raw integer token counts and convert to display units only at the edge.
- Fail loudly inside, degrade gracefully outside: a broken provider shows an error state in the UI;
  it never crashes the daemon or hides other providers.
- Match the style of the surrounding code.

### Rust

- Edition 2024, stable toolchain, pinned in `rust-toolchain.toml` (rustup picks it up by itself).
- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass. Workspace lints:
  `clippy::pedantic` warns, `unsafe_code` is forbidden, `unwrap_used` and `expect_used` are denied
  outside tests.
- Errors: `thiserror` enums in library crates, `anyhow` only in the binary. No `panic!`, `unwrap`,
  `expect` or `unreachable!` on runtime data; use `?` and typed errors with context.
- Libraries: `tokio`, `reqwest` with `rustls`, `zbus`, `serde`, `jiff` for time (everywhere),
  `rusqlite`. D-Bus and the Secret Service are Linux-only (`cfg(target_os = "linux")`). Secrets go
  to the Secret Service over `zbus` on Linux and to the Keychain through `/usr/bin/security` with
  the secret on stdin on macOS, with a `0600` file as fallback. No extra crates for either.
- Parse, don't validate: raw API types (`client.rs`) are separate from domain types
  (`headroom-core`). Unknown fields are ignored, missing required fields are errors, optional fields
  are `Option`.
- Newtypes for units that can be confused: tokens (`u64`), percent, money in micro-USD (`i64`),
  timestamps. Never use `f64` for money or token counts.
- No global mutable state. Pass dependencies explicitly; clocks and HTTP are injectable for tests.
- Files are written atomically (temp file, fsync, rename). Credential files are never modified
  unless Headroom owns them: CLI credentials such as `~/.codex/auth.json` or
  `~/.claude/.credentials.json` are read-only.
- Deduplicate log records by stable ids; never sum raw streaming chunks.
- Back off on 429 and 5xx with jitter; never poll providers in tight loops.
- Imports grouped std / external / crate, no glob imports except `use super::*` in tests.

### GNOME Shell extension (`shell/gnome`)

- ESM modules and the GNOME Shell 45+ extension API; the extension supports GNOME 46–50. `eslint`
  and `prettier` must pass.
- `St`, `Clutter`, `PanelMenu.Button` and `PopupMenu`; styles are generated from
  `styles/tokens.json` and `styles/template/` and use GNOME's variables, so light and dark themes
  and accent colors follow the system.
- All D-Bus access goes through one proxy module. `disable()` cleans up every signal, source and
  actor.
- No blocking calls on the shell main loop, no subprocesses except launching `headroom` for
  actions.

### Plasma widget (`shell/plasma`)

- Plasma 6.2+ (live signals need 6.4+, older versions poll), Qt 6, QML with `Kirigami` and
  `PlasmaComponents3`. Use the Qt 6 tools in `/usr/lib/qt6/bin` (`qmllint`, `qmlformat`); the
  ones in `/usr/bin` may be Qt 5.
- Compact representation in the panel, full representation as the popup. Use Kirigami units and
  theme colors; never hardcode pixel sizes or colors.

### macOS app (`shell/macos`)

- SwiftPM, Swift 6 language mode, deployment target macOS 14. An AppKit `NSStatusItem` and a
  key-capable non-activating `NSPanel` host SwiftUI; Liquid Glass only behind
  `if #available(macOS 26, *)` with a material fallback. No force unwraps, `try!` or `fatalError` on
  runtime data.
- `HeadroomKit` (models, socket client, supervisor policy, formatting, strings) is
  platform-independent and its tests also run on Linux with the swift.org toolchain. AppKit and
  SwiftUI code is guarded by `#if canImport(AppKit)`.
- The app renders the daemon's payload only; it bundles and supervises the `headroom` binary.

### Tests

- Every provider has fixture-based tests for its mapper and local log parser. Fixtures are real
  responses with all personal data removed.
- Token counting and pricing have table-driven tests covering deduplication, cache tokens, day
  boundaries and time zones.
- No network and no real home directory in tests: use temp dirs and injected clocks.
- A bug fix starts with a failing test.

### Design

Every surface (GNOME, Plasma, macOS) follows the same tokens and components, an OpenUsage-like
modern macOS look adapted to each toolkit:

- a 320 px popup with a 13 px radius, 12 px cards without borders, 5 px pill meters with an
  even-pace tick, a 4-pt grid with 2-pt half-steps, system fonts and tabular numerals for all
  figures;
- light and dark themes are both first-class and follow the system setting;
- color carries meaning only and comes from the daemon's `tone`: good → system accent, warning →
  amber, critical → red, neutral → gray. Tone reflects pace (burn rate); only when pace is untracked
  does it fall back to level (amber from 80 % used, red from 90 %). Shells never compute tone;
- motion is subtle: 150–250 ms ease-out, no bouncing, reduced-motion settings are honoured;
- every state is designed: loading, empty (tool not installed), signed out, error, stale data,
  offline;
- do not reuse OpenUsage's or OpenQuota's name, logo or icons, and use the Headroom brand only as
  described in [assets/brand/README.md](assets/brand/README.md).

## Build, run and test

### Rust

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cargo run -p headroom -- daemon                 # run the daemon from a checkout (stop the installed service first)
cargo run -p headroom -- status --json
cargo run -p headroom -- daemon --socket        # also serve the socket API
HEADROOM_SOCKET=<path> cargo run -p headroom -- status
```

Snapshot tests of the CLI output are refreshed with `UPDATE_SNAPSHOTS=1 cargo test -p headroom`;
review the diff before committing it.

The macOS code paths of the Rust crates can be checked from Linux without an Apple SDK:

```sh
rustup target add aarch64-apple-darwin
CC_aarch64_apple_darwin=/bin/true CXX_aarch64_apple_darwin=/bin/true AR_aarch64_apple_darwin=/bin/true \
  cargo clippy --workspace --all-targets --target aarch64-apple-darwin -- -D warnings
```

### GNOME Shell extension

```sh
cd shell/gnome
npm ci                   # eslint and prettier
make lint test           # eslint, prettier --check, gjs unit tests
make install             # pack and install into ~/.local/share/gnome-shell/extensions
```

To work on the extension without real accounts, run it against the mock daemon, which serves sample
states on the session bus instead of the real daemon:

```sh
make devkit SCENARIO=showcase                  # nested GNOME Shell in a window, sandboxed home, mock daemon
HEADLESS=1 make devkit                         # the same without a window (virtual monitor)
COLOR_SCHEME=prefer-dark make devkit           # dark theme inside the nested shell
make mock SCENARIO=critical                    # only the mock daemon, on your own session bus
```

Scenarios: `full`, `showcase`, `showcase-update`, `critical`, `empty`, `offline`, `single`,
`no_subscription`. Stop the real daemon (`systemctl --user stop headroom`) before `make mock`, since
both own the same bus name.

### Plasma widget

```sh
cd shell/plasma
make lint test           # qmllint, qmlformat check, message check, qmltestrunner
make install             # install or upgrade the widget with kpackagetool6
make mock NAME=full      # the mock daemon for a live widget
```

`make preview` renders the popup offscreen from a sample state into a PNG, without a running
Plasma session:

```sh
python3 ../gnome/dev/mock-daemon.py --dump --scenario showcase > /tmp/showcase.json
make preview STATE=/tmp/showcase.json          # OUT=… sets the output path
```

### macOS app

You need a Mac with Xcode (Swift 6) and Rust through rustup; the [macOS guide](docs/macos.md) covers
the one-time setup and signing.

```sh
cd shell/macos
swift build                        # every target
swift test                         # HeadroomKit unit tests
script/sync-fixtures.sh            # refresh test fixtures from the Rust snapshot tests
cd ../..
shell/macos/script/bundle.sh --install --open   # build, sign ad-hoc, copy to /Applications, launch
```

`HeadroomKit` also builds and tests on Linux with the swift.org toolchain:
`cd shell/macos && swift build --target HeadroomKit && swift test`.

## Commits and pull requests

- [Conventional Commits](https://www.conventionalcommits.org): `feat(codex): …`, `fix(daemon): …`,
  `refactor(core): …`, `docs: …`, `chore: …`. Mark breaking changes with `!`.
- One logical change per commit. The tree builds and the tests pass at every commit.
- Never commit secrets, real tokens, personal emails or unredacted logs, and check fixtures and
  screenshots for personal data.
- User-visible changes get a line under `[Unreleased]` in [CHANGELOG.md](CHANGELOG.md).
- Before opening a pull request, run the checks for every part you touched (see above). CI runs the
  same checks, and the pull request template has a short checklist.

By contributing you agree that your contribution is licensed under the [MIT license](LICENSE). The
Headroom brand assets are not part of that license; see [NOTICE.md](NOTICE.md).
