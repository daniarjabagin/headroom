# Headroom on macOS

Headroom on macOS is a native menu-bar app (`shell/macos`, SwiftUI + AppKit) that bundles the same
`headroom` binary the Linux build uses. The app starts `headroom daemon --socket <path>` as a child
process, talks to it over the [socket API](ipc.md) and only renders what the daemon sends. All
numbers, tones and pace come from the daemon, exactly as on Linux. Background: [research/macos.md](research/macos.md).

Requirements: macOS 14 (Sonoma) or newer. On macOS 26 and later the popup uses Liquid Glass; older
systems get the opaque tray or a system material.

## 1. One-time setup

1. **Xcode** from the App Store (full Xcode, not only the Command Line Tools: `swift test` and
   SwiftUI previews need it). Open it once to accept the license, then:

   ```sh
   sudo xcode-select -s /Applications/Xcode.app
   ```

2. **Rust** through rustup. The repository pins the toolchain in `rust-toolchain.toml`; rustup
   installs it on the first `cargo` call inside the repository.

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **A free signing certificate** (recommended). With ad-hoc signing macOS asks again for Keychain
   access after every rebuild, because the signature changes. A free "Apple Development" certificate
   keeps the grants:

   1. Xcode → Settings → Accounts → **+** → Apple ID, sign in.
   2. Select the team "(Personal Team)" → **Manage Certificates…** → **+** → **Apple Development**.
   3. Find the identity name:

      ```sh
      security find-identity -v -p codesigning
      #   1) 0123ABCD… "Apple Development: you@example.com (TEAMID1234)"
      ```

   Personal-team certificates expire after a while; when signing starts failing, create a new one
   the same way.

## 2. Build and install

From the repository root:

```sh
CODESIGN_IDENTITY="Apple Development: you@example.com (TEAMID1234)" \
    shell/macos/script/bundle.sh --install --open
```

`bundle.sh`:

- builds `cargo build --release -p headroom` and `swift build -c release` (native arm64 on Apple
  silicon; `--universal` builds arm64 + x86_64 and needs
  `rustup target add aarch64-apple-darwin x86_64-apple-darwin`),
- assembles `shell/macos/dist/Headroom.app`:
  `Contents/MacOS/Headroom`, `Contents/Helpers/headroom`, `Contents/Resources/AppIcon.icns`
  (a placeholder generated from the symbolic mark; skipped with a note if `qlmanage`/`sips`/`iconutil`
  fail) and `Info.plist` (`LSUIElement`, `io.github.headroom`, version from the workspace
  `Cargo.toml`, minimum macOS 14.0),
- signs the helper (`io.github.headroom.helper`) and then the app with `CODESIGN_IDENTITY`
  (default `-`, ad-hoc) and verifies the signature,
- `--install` quits a running Headroom and copies the app to `/Applications`; `--open` launches it.

Without `--install` run it from `shell/macos/dist/Headroom.app`. There is no Dock icon: Headroom
lives in the menu bar. Click the item for the popup, right-click for Refresh and Quit.

Launch at login: System Settings → General → Login Items → **+** → Headroom (an in-app switch comes
later).

## 3. First run

- **Keychain prompts.** Claude Code keeps its sign-in in the Keychain, so the daemon asks
  "headroom wants to use your confidential information stored in "Claude Code-credentials"".
  Choose **Always Allow**. With an Apple Development identity the grant survives rebuilds; with
  ad-hoc signing it is asked again after each rebuild.
- **Notifications.** macOS asks once whether Headroom may send notifications; limit alerts
  ("Under 10% left", "Limit reset") come from the daemon and are posted by the app.
- **Environment.** Apps started from Finder do not see your shell's `PATH` or `CODEX_HOME`. Headroom
  runs your login shell once (`$SHELL -i -l -c env`, 5 s timeout) and passes `PATH`, `CODEX_HOME`,
  `CLAUDE_CONFIG_DIR`, `GROK_HOME`, `CLINE_DIR`, `GH_CONFIG_DIR`, `XDG_*`, `LANG` and `LC_*` to the
  daemon. When `LANG` is missing it is derived from the macOS preferred language, which the daemon
  uses for notification texts while `display.language` is `system`.

## 4. Files and logs

| what | where |
| --- | --- |
| daemon socket | `~/Library/Application Support/Headroom/daemon.sock` (`$TMPDIR/headroom-<uid>.sock` when that path is longer than 103 bytes) |
| daemon database | `~/Library/Application Support/Headroom/` (see the daemon's config) |
| daemon log (stdout + stderr of the helper) | `~/Library/Logs/Headroom/daemon.log`, rotated to `daemon.log.1` above 5 MiB at start |
| app log | unified log, subsystem `io.github.headroom` |

```sh
tail -f ~/Library/Logs/Headroom/daemon.log
log stream --level info --predicate 'subsystem == "io.github.headroom"'
```

The CLI works against the same daemon: `HEADROOM_SOCKET` selects another socket.

```sh
/Applications/Headroom.app/Contents/Helpers/headroom status
```

## 5. Development

```sh
cd shell/macos
swift build                       # app, HeadroomUI and HeadroomKit
swift test                        # HeadroomKit unit tests (fixtures, client, supervisor, formatting)
open Package.swift                # Xcode, SwiftUI previews
script/sync-fixtures.sh           # refresh test fixtures from the Rust snapshot tests
```

Package layout:

| target | contents |
| --- | --- |
| `HeadroomKit` | platform-independent: Codable payload models, JSON-RPC line codec, `DaemonClient` actor over a Unix socket, `DaemonSupervisor` with restart backoff, login-shell environment, display formatting (en/ru), `AppModel`. Builds and tests on Linux too. |
| `HeadroomUI` | SwiftUI views (popup, menu-bar label, Liquid Glass / material surfaces). macOS only. |
| `Headroom` | the AppKit app: `NSStatusItem`, key-capable non-activating `NSPanel`, notifications, wiring. macOS only. |

Running the app outside a bundle (`swift run Headroom`) does not start a helper; it connects to a
daemon that is already listening, e.g. `cargo run -p headroom -- daemon --socket /tmp/h.sock` with
`HEADROOM_SOCKET=/tmp/h.sock swift run Headroom`. Notifications need the bundled app.

On Linux, `HeadroomKit` can be built and tested with the swift.org toolchain (`swift build --target
HeadroomKit && swift test`); the AppKit targets compile to nothing there.

## 6. Troubleshooting

| symptom | what to do |
| --- | --- |
| Popup says "Service not running" | Read `~/Library/Logs/Headroom/daemon.log`. The app restarts the helper with backoff (1 s … 60 s). "another Headroom daemon is already listening" means a second daemon (for example one started from a terminal) owns the socket; stop it. |
| Popup says the bundled service does not match | `Contents/Helpers/headroom --version` differs from the app version. Rebuild with `bundle.sh` so both come from the same checkout. |
| Popup says the service is a different version | The daemon on the socket speaks another state schema. Quit other Headroom daemons and restart the app. |
| Keychain asks after every rebuild | Sign with an Apple Development identity (§1.3). |
| Codex/Claude accounts missing | Check that `PATH`/`CODEX_HOME`/`CLAUDE_CONFIG_DIR` are exported by your login shell (`$SHELL -i -l -c env`). Then Refresh. |
| "Headroom is damaged" / Gatekeeper on a copied build | Only builds from another machine are quarantined: `xattr -d com.apple.quarantine /Applications/Headroom.app`. |
| Menu bar shows only the gauge glyph | No visible account has a visible window yet, or the daemon is not connected. |
