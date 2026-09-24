# Security policy

Headroom reads the sign-ins of your AI coding tools, so security problems matter even in a small
project. Thank you for reporting them responsibly.

## Supported versions

Only the latest minor release gets security fixes. Please check that the problem still exists in the
[latest release](https://github.com/daniarjabagin/headroom/releases/latest) before reporting it.

## Reporting a vulnerability

**Do not open a public issue.** Report privately through GitHub:

1. Open the [Security tab](https://github.com/daniarjabagin/headroom/security) of the repository.
2. Click **Report a vulnerability** and describe the problem, the affected version and platform, and
   how to reproduce it.

Do not include real tokens, API keys or personal data; placeholders are enough.

Headroom is maintained by one person in their spare time. You can expect an acknowledgement within
a week and an honest assessment after that. Confirmed problems are fixed in a new release as soon as
possible and credited in the [changelog](CHANGELOG.md) and the advisory, unless you prefer to stay
anonymous. Please give the fix a reasonable time to ship before disclosing details.

## In scope

- **Credential handling:** reading CLI sign-ins, storing API keys and Headroom's own sign-ins, and
  anything that could leak a secret into logs, errors, process arguments or IPC payloads.
- **IPC exposure:** the D-Bus service on the session bus and the Unix socket, including anything
  that lets another user or process read state or trigger actions it should not.
- **Updates:** the Linux updater and its `SHA256SUMS` verification, and the macOS Sparkle updates
  with their EdDSA signatures.
- **Install scripts:** `get-headroom.sh`, `packaging/install.sh`, the packages and the Homebrew cask.

Out of scope: vulnerabilities in the providers' own services and CLIs, and attacks that require
control of your user account (someone who can run code as you can already read your credentials).

## Security model

- **Local only.** Headroom talks to the providers you use and fetches public price lists. It has no
  telemetry and no server of its own.
- **CLI credentials are read-only.** Files such as `~/.codex/auth.json` and
  `~/.claude/.credentials.json`, and the matching Keychain items on macOS, are never written or
  refreshed.
- **Secrets stay in the system keyring.** API keys live in the Secret Service on Linux and in the
  Keychain on macOS. Only when no keyring is available do they go to a `0600` file. Headroom's own
  data directory is `0700`.
- **Only your user can talk to the daemon.** D-Bus uses your session bus; the socket is `0600` inside
  a `0700` directory. Panels and the menu-bar app never see secrets.
- **One request a day to GitHub** checks for updates. Nothing is sent besides the request itself,
  and the check can be turned off. Downloads are verified with SHA-256 on Linux and with Sparkle's
  EdDSA signature on macOS before they are installed.

The [Privacy](README.md#privacy) section of the README and [docs/architecture.md](docs/architecture.md)
describe the model in more detail.
