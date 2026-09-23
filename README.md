# Headroom

See how much of your AI coding limits is left — right in your Linux top panel.

Headroom tracks Codex, Claude Code and other AI coding tools: session and weekly limits, reset times,
exact token usage and estimated spend, across multiple accounts. Native on GNOME, KDE Plasma and any
desktop with a system tray, on Wayland and X11.

Status: early development.

## Install

Requirements: a Rust toolchain (`cargo`), a systemd user session and a D-Bus session bus. The GNOME
extension additionally needs `gnome-extensions`, `glib-compile-schemas` and `python3`.

```sh
packaging/install.sh              # binary, systemd user service, D-Bus activation, GNOME extension
packaging/install.sh --no-gnome   # skip the GNOME Shell extension
packaging/install.sh --no-service # install ~/.local/bin/headroom only
```

The script is idempotent: run it again to upgrade. It installs `~/.local/bin/headroom`, enables and
restarts `headroom.service` (a user unit) and registers D-Bus activation, so panels start the daemon
on demand. On GNOME, enable the extension afterwards with
`gnome-extensions enable headroom@headroom.github.io` (on Wayland, log out and back in first).

`packaging/uninstall.sh` removes everything again and keeps your data.

### Environment for the service

The daemon finds Codex in `$CODEX_HOME` (default `~/.codex`) and Claude Code in `$CLAUDE_CONFIG_DIR`
(default `~/.claude`, plus other Claude config directories it discovers). If you set these variables in
your shell, set them for the user service too, for example in `~/.config/environment.d/headroom.conf`:

```sh
CODEX_HOME=$HOME/work/codex
CLAUDE_CONFIG_DIR=$HOME/.claude-work
```

Then log out and back in, or run `systemctl --user daemon-reload && systemctl --user restart headroom`.
Logs go to the journal: `journalctl --user -u headroom -f` (set `RUST_LOG=debug` in the same file for
more detail).

## Usage

```sh
headroom status             # limits, pace and spend for every account
headroom status --json      # the raw state payload (see docs/dbus-api.md)
headroom refresh [ID]       # refresh all due accounts, or one account now
headroom accounts           # list accounts with their ids
headroom accounts label ID "Work"
headroom accounts hide ID   # and: show ID, order ID1 ID2 …
```

`status` talks to the running daemon; when it is not running it shows the last cached data.

Headroom reads the accounts you are signed in to with the `codex` and `claude` CLIs. To track more
accounts, let Headroom keep a separate sign-in for each:

```sh
headroom accounts add codex --label work   # runs `codex login` with its own CODEX_HOME
headroom accounts add claude               # runs `claude auth login` with its own CLAUDE_CONFIG_DIR
headroom accounts remove ID                # deletes such a sign-in (never ~/.codex or ~/.claude)
```

These homes live in `~/.local/share/headroom/accounts/`. The daemon picks up a new account within
10 minutes, or immediately after `systemctl --user restart headroom`.

## Waybar

`headroom waybar` prints one JSON line per change for a `custom` module: the headline percentage as
text, a tooltip with every account and your spend, and a `good`, `warning`, `critical` or `neutral`
class. It waits for the daemon when it is not running.

```jsonc
"custom/headroom": {
    "exec": "headroom waybar",
    "return-type": "json",
    "format": "{text}",
    "tooltip": true
}
```

```css
#custom-headroom.warning { color: #ffd60a; }
#custom-headroom.critical { color: #ff453a; }
#custom-headroom.neutral { opacity: 0.6; }
```

## Acknowledgements

Provider data collection is informed by [OpenQuota](https://github.com/deviffyy/OpenQuota) (MIT).

## License

MIT
