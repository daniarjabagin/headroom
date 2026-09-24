# Headroom socket API

On macOS the daemon has no session bus. It serves the same commands and events as the
[D-Bus API](dbus-api.md) over a Unix domain socket instead. The macOS app (`shell/macos`) and the
`headroom` CLI on macOS use it. On Linux D-Bus stays the primary transport; `headroom daemon --socket`
serves the socket in addition, which is how the socket transport is tested on Linux.

Every payload (state, settings, providers) is the same JSON document the D-Bus API returns. Only the
framing differs.

The state payload also carries `update` (a newer Headroom release, see
[Update](dbus-api.md#update)). The macOS app updates itself with Sparkle and starts its daemon with
`headroom daemon --no-update-check`, so `update` stays `null` there and the daemon sends no update
requests.

The state payload carries `app_version`, the release of the daemon that serves the socket. A client
that bundles its own daemon (the macOS app) compares it with its own release to notice that it is
connected to a foreign daemon, for example one started from another install or left running across an
upgrade. The field is missing from daemons that predate it; `version` alone decides whether the
payload can be read.

## Socket

| item | value |
| --- | --- |
| default path (macOS) | `~/Library/Application Support/Headroom/daemon.sock` |
| default path (Linux, `--socket` without a path) | `$XDG_RUNTIME_DIR/headroom/daemon.sock` |
| fallback when the path exceeds 103 bytes | `$TMPDIR/headroom-<uid>/daemon.sock` |
| override | `headroom daemon --socket <path>`, clients: `HEADROOM_SOCKET=<path>` |
| permissions | socket `0600`, parent directory created `0700` when missing (an existing directory is left as it is) |
| lock | `daemon.lock` next to the socket (the socket path with the extension `lock`), `0600`, held with an exclusive `flock` for the daemon's lifetime |
| framing | UTF-8 JSON-RPC 2.0, one JSON object per line terminated by `\n` |

The fallback directory `$TMPDIR/headroom-<uid>` is created `0700` when missing. The daemon and the
CLI both refuse to use it unless it is a real directory (not a symlink) owned by the current uid with
mode `0700` exactly, with an error such as "refusing to use /tmp/headroom-1000: its mode is 0755, not
0700", because another user could have created it first.

On start the daemon takes the lock without waiting, then removes a stale socket file. If the lock is
held or another daemon is listening on the path, the new one exits with "another Headroom daemon is
already listening on <path>"; two daemons started at once therefore never both clear and bind the
socket. The socket is removed on shutdown; the lock file stays and is simply unlocked.

## Requests

```json
{"jsonrpc":"2.0","id":7,"method":"Refresh","params":["codex:1f3a…"]}
```

- `params` is a positional array with the D-Bus arguments in order (strings, booleans, string
  arrays). Methods without arguments take `[]` or omit `params`.
- `id` is a number, a string or `null`. A request without `id` is a JSON-RPC notification: it is
  executed and gets no response, not even an error (invalid params included). Only a line that is not
  valid JSON or not a request object is answered with an `"id": null` error. Blank lines are ignored.
- A client may send several requests without waiting; responses carry the request `id` and may arrive
  in any order, interleaved with notifications.

| method | params | result |
| --- | --- | --- |
| `GetState` | `[]` | state payload as a JSON **object** (not a string) |
| `ListProviders` | `[]` | providers payload as a JSON object |
| `GetSettings` | `[]` | settings as a JSON object |
| `Refresh` | `[account_id]` | `null` |
| `RefreshNow` | `[]` | `null` |
| `Rescan` | `[]` | `null` |
| `SetSettings` | `[json_string]` | `null` |
| `UpdateSettings` | `[patch_string]` | `null` |
| `SetAccountLabel` | `[account_id, label]` | `null` |
| `SetAccountOrder` | `[[id, …]]` | `null` |
| `SetAccountHidden` | `[account_id, hidden_bool]` | `null` |
| `DismissAccount` | `[account_id]` | `null` |
| `RestoreAccounts` | `[provider]` | `null` |
| `Subscribe` | `[]` or `[[topic, …]]` | `null`; from now on this connection receives the notifications of those topics |

`Subscribe` topics are `state` (`StateChanged`), `alerts` (`Alert`) and `open` (`OpenRequested`).
No params, `[]` or an empty topic list subscribe to all three; an unknown topic is `-32602`. Calling
`Subscribe` again replaces the connection's topics. A client that does not show alerts (such as
`headroom waybar`, which sends `[["state"]]`) must leave out `alerts`, so it never counts as an
alert delivery.

Semantics, validation and side effects of every method are exactly those in
[dbus-api.md](dbus-api.md#methods). `SetSettings` and `UpdateSettings` take the settings JSON as a
string, as on D-Bus.

## Responses and errors

```json
{"jsonrpc":"2.0","id":7,"result":null}
{"jsonrpc":"2.0","id":8,"error":{"code":-32602,"message":"unknown account id codex:zz"}}
```

| code | meaning | D-Bus equivalent |
| --- | --- | --- |
| `-32700` | line is not valid JSON | — |
| `-32600` | not a JSON-RPC 2.0 request object | — |
| `-32601` | unknown method | `UnknownMethod` |
| `-32602` | invalid arguments: wrong param count/types, plus every `InvalidArgs` case in dbus-api.md | `InvalidArgs` |
| `-32000` | failure inside the daemon | `Failed` |

`message` is human readable and safe to show. `-32700` and `-32600` responses carry `"id": null`
when the id could not be read. A line longer than 1 MiB (1,048,576 bytes before the `\n`) closes the
connection without a response.

## Notifications

Sent only to connections that called `Subscribe` with the notification's topic. They have no `id`.

```json
{"jsonrpc":"2.0","method":"StateChanged","params":{"state":{…}}}
{"jsonrpc":"2.0","method":"OpenRequested","params":{}}
{"jsonrpc":"2.0","method":"Alert","params":{"id":"codex:1f3a…/session/almost_out","title":"Codex · Work — Session","body":"Under 10% left · resets in 42m","account_id":"codex:1f3a…","urgency":"normal"}}
```

| notification | when |
| --- | --- |
| `StateChanged` | same as the D-Bus signal: full state payload, debounced by 250 ms |
| `OpenRequested` | a client asked the daemon to open the popup (reserved; the macOS app handles its own notification clicks) |
| `Alert` | on macOS, instead of `org.freedesktop.Notifications`: the app posts it with `UNUserNotificationCenter`. `urgency` is `low`, `normal` or `critical`. Texts follow `display.language` exactly as in [Notifications](dbus-api.md#notifications). A delivery counts as successful when at least one connection subscribed to `alerts` received it; otherwise it is rolled back and retried at the next refresh, like a failed desktop notification on Linux. |

A subscriber should call `GetState` after `Subscribe` and then follow `StateChanged`. A subscriber
that stops reading loses notifications once 64 are queued for it.

`Alert` fields: `id` is `<account_id>/<window>/<milestone>` for window milestones (`window` as in the
state payload, `milestone` one of `almost_out`, `cutting_it_close`, `will_run_out`, `reset`) and
`<account_id>/subscription_inactive` for a subscription lapse; the same id may repeat after the
milestone re-arms, so it suits replacing a shown notification. `urgency` is `low` for `reset`,
`critical` for `will_run_out` when the limit is already reached, `normal` otherwise.
