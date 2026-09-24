# Headroom on macOS

Headroom on macOS is a native menu-bar app (`shell/macos`, SwiftUI + AppKit) that bundles the same
`headroom` binary the Linux build uses. The app starts `headroom daemon --socket <path>` as a child
process, talks to it over the [socket API](ipc.md) and only renders what the daemon sends. All
numbers, tones and pace come from the daemon, exactly as on Linux. Background: [research/macos.md](research/macos.md).

Requirements: macOS 14 (Sonoma) or newer. The popup is an opaque surface by default; Settings →
General → Translucent background switches it to a blurred system material (ignored under Reduce
Transparency). Popup and Settings follow `display.theme`.

## Install from a release (DMG)

Every GitHub release has `Headroom-<version>-universal.dmg` (Apple silicon and Intel) and its
`.sha256`. The rest of this page (sections 1–3) is only needed to build from source.

1. Optionally check the download: `shasum -a 256 -c Headroom-<version>-universal.dmg.sha256` in the
   download folder prints `OK`.
2. Open the DMG and drag **Headroom** onto **Applications**. Eject the DMG.
3. Allow the first launch. The app is ad-hoc signed and not notarized, so Gatekeeper blocks it the
   first time. On macOS 15 (Sequoia) and newer, right-click → Open no longer bypasses this:
   1. Open Headroom from Applications. macOS says it "cannot verify that Headroom is free of
      malware"; click **Done** (not Move to Trash).
   2. System Settings → **Privacy & Security**, scroll to Security: "Headroom was blocked to protect
      your Mac" → **Open Anyway** (shown for about an hour after the blocked launch).
   3. Confirm with **Open Anyway** again and your password or Touch ID. Headroom starts and later
      launches open without asking.

   Alternatively, remove the quarantine flag in Terminal, then open the app normally:

   ```sh
   xattr -dr com.apple.quarantine /Applications/Headroom.app
   ```

4. Headroom appears in the menu bar (there is no Dock icon). Continue with [First run](#5-first-run).

Upgrading from a 0.4.0 development build: the bundle id changed from `io.github.headroom` to
`io.github.daniarjabagin.headroom`, so macOS treats the new app as a different one. Quit and delete the
old app, allow notifications and launch at login again, and add API keys again (the old ones stay in
the Keychain under `io.github.headroom` until you delete them in Keychain Access).

Why this is needed: notarization requires a paid Apple Developer account, which the project does
not have yet. The DMG is built by the release workflow from the tagged source, with the same
`bundle.sh --universal --dmg` described below, and the app signature is only an ad-hoc one. Because
an ad-hoc signature changes with every build, macOS forgets Keychain "Always Allow" grants after each
update: expect the "Claude Code-credentials" prompt again once per new version and choose **Always
Allow** again. Later versions arrive through the built-in updater ([Updates](#updates)); the
Gatekeeper step is only needed for the first install.

## Updates

Headroom updates itself with [Sparkle 2](https://sparkle-project.org) (2.10.0, pulled in by
SwiftPM and embedded as `Contents/Frameworks/Sparkle.framework`). Sparkle owns updates on macOS:
the app starts its daemon with `--no-update-check`, so the daemon never polls GitHub on its own there,
and the popup has no update notice.

- **Feed.** The app reads
  `https://github.com/daniarjabagin/headroom/releases/latest/download/appcast.xml` (`SUFeedURL`).
  Every release (not pre-releases) carries an `appcast.xml` with one item: the version, the build
  number, the universal DMG with its size and EdDSA signature, minimum macOS 14.0, the release notes
  and a link to the GitHub release.
- **Checks.** Automatic checks are on by default (`SUEnableAutomaticChecks`), once a day
  (`SUScheduledCheckInterval` = 86400). Because the default is set in `Info.plist`, Sparkle does not
  show its "Check for updates automatically?" prompt on the second launch. Settings → Service →
  App updates turns automatic checks off or on, has **Check Now** and shows when the last check ran;
  the right-click menu has **Check for Updates…**.
- **Install.** When a newer build number is found, Sparkle shows the release notes and offers
  **Install Update**; it downloads the DMG, checks it, replaces `/Applications/Headroom.app` and
  relaunches. The downloaded update is not quarantined, so the Gatekeeper steps of the first install
  are not repeated.
- **Trust without a Developer ID.** The app is ad-hoc signed, so Sparkle cannot compare Apple code
  signatures between versions. It accepts an app update when either the archive's EdDSA signature
  matches the `SUPublicEDKey` of the installed app, or both apps carry matching Developer ID
  signatures (`SUUpdateValidator` in Sparkle). The EdDSA check is what protects Headroom's updates;
  the ad-hoc signature only has to be valid. Never change `script/sparkle-public-key.txt` without
  a key rotation plan: apps with the old key reject updates signed with a new one.
- **Keychain after an update.** Every release has a new ad-hoc signature, so macOS asks again for
  "Claude Code-credentials" after each update; choose **Always Allow** once per version.
- **Build numbers.** Sparkle compares `CFBundleVersion`, which `bundle.sh` derives from the version
  (`major × 10000 + minor × 100 + patch`), so it always grows with the version.

### Publishing (maintainers)

The EdDSA key pair was generated for Headroom; only the public key is in the repository
(`shell/macos/script/sparkle-public-key.txt`, written into `SUPublicEDKey`). The private key is kept
outside the repository at `~/.config/headroom-release/sparkle_ed25519_private.txt` on the
maintainer's machine (directory `0700`, file `0600`). Its format is Sparkle's exported key: one line,
the base64 of the 32-byte Ed25519 seed, the same as `generate_keys -x` writes.

When the repository is published, add that line as the GitHub Actions secret
**`SPARKLE_ED_PRIVATE_KEY`** (Settings → Secrets and variables → Actions). The macOS release job
then downloads Sparkle's `sign_update` (pinned 2.10.0, SHA-256 checked), signs the DMG with the key
on stdin, verifies the signature against the committed public key
(`script/verify-ed-signature.swift`) and writes `appcast.xml` with `script/write-appcast.sh`; the
release job attaches it next to the DMG. Without the secret the job only warns and the release has
no appcast, so installed apps are not offered it. Back the private key up: if it is lost, installed
apps can only be updated by hand.

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

## 2. Get the code onto the Mac

The repository has no remote. Copy it from the Linux machine either as a git bundle (keeps the
history, and later updates are a `git pull` away):

```sh
# on Linux, in the repository
git bundle create headroom.bundle --all
# AirDrop / scp / USB headroom.bundle to the Mac, then on the Mac:
git clone headroom.bundle headroom
cd headroom
# later: make a new bundle on Linux, copy it over the old file, then
git pull
```

or as a plain copy of the working tree, run from the repository root:

```sh
rsync -a --delete --exclude target --exclude shell/macos/.build --exclude shell/macos/dist \
    ./ mac.local:headroom/
```

## 3. Build and install

### First build checklist

The AppKit and SwiftUI targets are only compiled on a Mac, so the first build there is also their
first compile. Go through these steps in order and stop at the first one that fails:

1. `xcode-select -p` prints `/Applications/Xcode.app/Contents/Developer` and `swift --version` shows
   Swift 6.0 or newer.
2. `cd shell/macos && swift build 2>&1 | tee /tmp/headroom-swift-build.log` compiles all four
   targets (debug).
3. `swift test 2>&1 | tee /tmp/headroom-swift-test.log` passes (it finishes in a few seconds; a
   run longer than a minute is a hang worth reporting).
4. From the repository root: `shell/macos/script/bundle.sh --open 2>&1 | tee /tmp/headroom-bundle.log`
   (add `CODESIGN_IDENTITY=…` once you have one). The log lists `resources:
   HeadroomMac_HeadroomUI.bundle` and ends with `built …/Headroom.app`.
5. The menu-bar item appears; the popup opens under it, shows provider logos, grows and shrinks with
   its content (expand an account, switch the spend period), stops growing at 600 pt (or 40 pt short
   of the screen's visible height) and scrolls beyond that with a thin indicator, and starts at the
   top when reopened. With Translucent background off it is fully opaque in both themes.

If a step fails, send back:

- the complete log of the failing step (`/tmp/headroom-swift-build.log`, `-test.log` or
  `-bundle.log`), not only the last lines: the first `error:` is usually the one that matters;
- the output of `swift --version` and `sw_vers`;
- for a crash or wrong behaviour after launch: `log show --last 5m --predicate 'subsystem ==
  "io.github.daniarjabagin.headroom"'` and `~/Library/Logs/Headroom/daemon.log`, plus a screenshot.

The logs contain file paths but no credentials; still skim them for account emails before sending.

### Build

From the repository root, with your identity from §1.3:

```sh
CODESIGN_IDENTITY="Apple Development: you@example.com (TEAMID1234)" \
    shell/macos/script/bundle.sh --install --open
```

`bundle.sh`:

- builds `cargo build --release -p headroom` and `swift build -c release --product Headroom`, which
  links the `HeadroomKit`, `HeadroomUI` and `HeadroomSettings` targets into the app (native arm64 on
  Apple silicon; `--universal` builds arm64 + x86_64 and needs
  `rustup target add aarch64-apple-darwin x86_64-apple-darwin`),
- `--no-cargo` skips the Rust build for UI-only changes and reuses the helper of the previous
  `dist/Headroom.app` (or the last `cargo build --release`); the helper's `--version` must still match
  the workspace version, otherwise the app refuses to start it,
- stages the helper as `dist/headroom-daemon` (macOS file systems are case-insensitive, so
  `dist/headroom` would overwrite the app binary `dist/Headroom`) and assembles
  `shell/macos/dist/Headroom.app`:
  `Contents/MacOS/Headroom`, `Contents/Helpers/headroom`, every SwiftPM resource bundle from
  `swift build --show-bin-path` in `Contents/Resources/` (`HeadroomMac_HeadroomUI.bundle` with the
  provider logos; the build fails when none is found), `Contents/Frameworks/Sparkle.framework`
  (from the SwiftPM build, without its XPC services because the app is not sandboxed; the app binary
  gets the `@executable_path/../Frameworks` rpath), `Contents/Resources/AppIcon.icns` (the app icon,
  converted with `iconutil` from the committed `shell/macos/Icon/AppIcon.iconset`; the build fails
  if that fails) and `Info.plist` (`LSUIElement`, `io.github.daniarjabagin.headroom`, `CFBundleShortVersionString`
  from the workspace `Cargo.toml`, `CFBundleVersion` as the build number
  `major × 10000 + minor × 100 + patch` (0.4.0 → 400; minor and patch must stay below 100),
  minimum macOS 14.0, and the Sparkle keys `SUFeedURL`, `SUPublicEDKey` from
  `script/sparkle-public-key.txt`, `SUEnableAutomaticChecks`, `SUScheduledCheckInterval` = 86400),
- signs the helper (`io.github.daniarjabagin.headroom.helper`), then Sparkle's `Autoupdate`, `Updater.app` and
  the framework, then the app with `CODESIGN_IDENTITY` (default `-`, ad-hoc) and verifies the
  signature,
- `--dmg` then creates `shell/macos/dist/Headroom-<version>-<arch>.dmg` (`arm64` natively,
  `universal` with `--universal`): volume "Headroom <version>" with the app and an `/Applications`
  symlink for drag-to-install, compressed UDZO, checked with `hdiutil verify`, signed only when
  `CODESIGN_IDENTITY` is a real identity, plus `Headroom-<version>-<arch>.dmg.sha256` (the hash is
  printed). The release workflow runs `bundle.sh --universal --dmg` on `macos-15` after `swift test`
  and attaches both files to the GitHub release, together with the signed `appcast.xml`
  ([Publishing](#publishing-maintainers)),
- `--install` quits a running Headroom and copies the app to `/Applications`; `--open` launches it.

Without `--install` run it from `shell/macos/dist/Headroom.app`. There is no Dock icon: Headroom
lives in the menu bar. Click the item for the popup, right-click for **Refresh Now** (⌘R),
**Settings…** (⌘,), **Check for Updates…** (bundled app only) and **Quit Headroom** (⌘Q).

Launch at login is off by default; turn it on in Settings → Service (it uses `SMAppService.mainApp`,
so install the app to `/Applications` first). If macOS asks for approval, the Service tab links to
System Settings → General → Login Items.

## 4. Settings

Settings… opens a preferences window with toolbar tabs (General, Accounts, Notifications, Service; the
window title follows the tab, the app comes to the front even though it has no Dock icon). All
values live in the daemon; every change is sent at once as an `UpdateSettings` merge patch, in order,
and the window re-reads the settings when the writes are done. The app also loads them every time it
(re)connects to the daemon, so app-side options such as reduce motion apply to the popup before
Settings is ever opened. Texts follow `display.language`
(`system` uses the macOS preferred language).

| tab | contents |
| --- | --- |
| General | theme, language, translucent background, reduce motion; values left/used, reset countdown/exact; menu bar limit (auto or a pinned account + window) and label (percent or provider + limit); popup sections (total spend, per-account spend, trend, forecast); refresh interval |
| Accounts | the daemon's accounts in their order: drag to reorder, show/hide, rename, hide single limits, remove. Remove runs `headroom accounts remove <id> --yes --progress json`: a CLI-owned account is only hidden ("The <provider> CLI stays signed in"), a Headroom-owned one is signed out |
| Accounts → Add Account… | the providers from `ListProviders`. `cli_login` runs `headroom accounts add <provider> --progress json` and shows the progress (Open Sign-In Page, a device code with Copy, a field to paste a code, the CLI output, Cancel); `api_key` sends the key on stdin with `--api-key-stdin`, never on the command line; `auto_detect` explains where Headroom looks and offers Detect Again (`RestoreAccounts`) |
| Notifications | the four milestones of the settings schema. macOS asks for permission on the first alert or when a milestone is switched on; when notifications are denied the tab links to System Settings |
| Service | app version, service status, the daemon log (Open Log, Show in Finder), Launch at login, app updates (Automatically check for updates, Check Now, last check) |

The account commands run the bundled `Contents/Helpers/headroom` with the same environment as the
daemon (login-shell `PATH`, `CODEX_HOME`, …) plus `HEADROOM_SOCKET` set to the app's socket, so the
CLI and the app always talk to the same daemon.

## 5. First run

- **Keychain prompts.** Claude Code keeps its sign-in in the Keychain, so the daemon asks
  "headroom wants to use your confidential information stored in "Claude Code-credentials"".
  Choose **Always Allow**. With an Apple Development identity the grant survives rebuilds; with
  ad-hoc signing it is asked again after each rebuild.
- **Notifications.** macOS asks whether Headroom may send notifications when the first alert
  arrives (or when you switch on a milestone in Settings → Notifications); limit alerts ("Under 10%
  left", "Limit reset") come from the daemon and are posted by the app. Clicking one opens the popup.
- **Environment.** Apps started from Finder do not see your shell's `PATH` or `CODEX_HOME`. Headroom
  runs your login shell once (`$SHELL -i -l -c env`, 5 s timeout) and passes `PATH`, `CODEX_HOME`,
  `CLAUDE_CONFIG_DIR`, `GROK_HOME`, `CLINE_DIR`, `GH_CONFIG_DIR`, `XDG_*`, `LANG` and `LC_*` to the
  daemon. When `LANG` is missing it is derived from the macOS preferred language, which the daemon
  uses for notification texts while `display.language` is `system`.

## 6. Files and logs

| what | where |
| --- | --- |
| daemon socket | `~/Library/Application Support/Headroom/daemon.sock`; when that path is longer than 103 bytes, `$TMPDIR/headroom-<uid>/daemon.sock` in a private `0700` directory that the daemon creates (the app never creates it) |
| daemon database | `~/Library/Application Support/Headroom/` (see the daemon's config) |
| daemon log (stdout + stderr of the helper) | `~/Library/Logs/Headroom/daemon.log`, rotated to `daemon.log.1` above 5 MiB at start |
| app log | unified log, subsystem `io.github.daniarjabagin.headroom` |

```sh
tail -f ~/Library/Logs/Headroom/daemon.log
log stream --level info --predicate 'subsystem == "io.github.daniarjabagin.headroom"'
```

The CLI works against the same daemon: `HEADROOM_SOCKET` selects another socket.

```sh
/Applications/Headroom.app/Contents/Helpers/headroom status
```

## 7. Development

```sh
cd shell/macos
swift build                       # app, HeadroomSettings, HeadroomUI and HeadroomKit
swift test                        # HeadroomKit unit tests (fixtures, client, supervisor, formatting)
open Package.swift                # Xcode, SwiftUI previews
script/sync-fixtures.sh           # refresh test fixtures from the Rust snapshot tests
```

Package layout:

| target | contents |
| --- | --- |
| `HeadroomKit` | platform-independent: Codable payload models, JSON-RPC line codec, `DaemonClient` actor over a Unix socket, `DaemonSupervisor` with restart backoff, login-shell environment, the `headroom accounts` progress runner, display formatting and every app string (en/ru), `AppModel`, `SettingsStore`, the add-account session. Builds and tests on Linux too. |
| `HeadroomUI` | SwiftUI views (popup, menu-bar label, opaque and blurred surfaces, thin scroll indicator). macOS only. |
| `HeadroomSettings` | the Settings window (toolbar-style `NSTabViewController`, one SwiftUI form per tab), add-account flows, notification permission, launch at login. macOS only. |
| `Headroom` | the AppKit app: `NSStatusItem`, key-capable non-activating `NSPanel`, menus, notifications, theme, Sparkle updater (`SparkleUpdates`, feeding `UpdatesModel` in HeadroomKit), wiring. macOS only; Sparkle is a macOS-only dependency of this target and is imported behind `#if canImport(Sparkle)`. |

Running the app outside a bundle (`swift run Headroom`) does not start a helper; it connects to a
daemon that is already listening, e.g. `cargo run -p headroom -- daemon --socket /tmp/h.sock` with
`HEADROOM_SOCKET=/tmp/h.sock swift run Headroom`. Notifications need the bundled app. Provider logos
are looked up in `HeadroomMac_HeadroomUI.bundle` under the app's `Contents/Resources`, then next to
the executable (so `swift run` finds the bundle in the build directory); both the flat SwiftPM layout
and a `Contents/Resources` layout inside the bundle work.

On Linux, `HeadroomKit` can be built and tested with the swift.org toolchain (`swift build --target
HeadroomKit && swift test`); the AppKit targets compile to nothing there.

## 8. Troubleshooting

| symptom | what to do |
| --- | --- |
| Popup says "Service not running" | Read `~/Library/Logs/Headroom/daemon.log`. The app restarts the helper with backoff (1 s … 60 s). "another Headroom daemon is already listening" means a second daemon (for example one started from a terminal) owns the socket; stop it. |
| Popup says the bundled service does not match | `Contents/Helpers/headroom --version` differs from the app version. Rebuild with `bundle.sh` so both come from the same checkout. |
| Popup says the service is a different version | The daemon on the socket speaks another state schema. Quit other Headroom daemons and restart the app. |
| "A different Headroom service is running" | The daemon on the socket reports another `app_version` than the app (for example a `cargo run` daemon or an older install). Stop it (`pkill -f 'headroom daemon'`); the app starts its own helper and recovers. |
| Add account says the helper is missing | The app was started outside the bundle (`swift run`). Account commands need `Contents/Helpers/headroom`; build with `bundle.sh`. |
| Keychain asks after every rebuild | Sign with an Apple Development identity (§1.3). |
| Codex/Claude accounts missing | Check that `PATH`/`CODEX_HOME`/`CLAUDE_CONFIG_DIR` are exported by your login shell (`$SHELL -i -l -c env`). Then Refresh. |
| "Headroom is damaged" / "cannot verify" on a downloaded or copied build | Downloaded and copied builds are quarantined: use Privacy & Security → Open Anyway ([Install from a release](#install-from-a-release-dmg)) or `xattr -dr com.apple.quarantine /Applications/Headroom.app`. |
| Keychain asks again after an update | Expected: release builds are ad-hoc signed, so each version has a new signature. Choose Always Allow once per version. |
| Check for Updates says the update is improperly signed | The release's `appcast.xml` was signed with a key that does not match `SUPublicEDKey` of the installed app. Install the new DMG by hand and report it. |
| Check for Updates is missing | The app runs outside a bundle (`swift run`) or its `Info.plist` has no `SUFeedURL`/`SUPublicEDKey`; build with `bundle.sh`. |
| Menu bar shows only the Headroom mark | No visible account has a visible window yet, or the daemon is not connected. |
