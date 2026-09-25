# Headroom D-Bus API

The daemon (`headroom daemon`, crate `headroom-daemon`) is the only process that talks to providers.
Shells (GNOME extension, Plasma plasmoid, tray, TUI, CLI) read state and send commands over the
session bus. Payloads are JSON strings so every toolkit can parse them the same way.

| item | value |
| --- | --- |
| bus | session bus |
| well-known name | `io.github.daniarjabagin.Headroom` |
| object path | `/io/github/daniarjabagin/Headroom` |
| interface | `io.github.daniarjabagin.Headroom1` |

The daemon requests the name with `DO_NOT_QUEUE` and without `ALLOW_REPLACEMENT`. If another daemon
already owns it, the new one exits with "another Headroom daemon already owns the bus name".

The same methods, payloads and events are served over a Unix socket as JSON-RPC 2.0, see
[Socket API](ipc.md). That is the transport on macOS; on Linux `headroom daemon --socket` adds it.

## Methods

| method | signature | description |
| --- | --- | --- |
| `GetState` | `() → s` | Current state payload (see [State](#state-payload)). Assembled on every call. |
| `ListProviders` | `() → s` | The providers compiled into this build and how to add their accounts (see [Providers](#providers)). Does not change while the daemon runs. |
| `Refresh` | `(s account_id) → ()` | `""`: refresh every visible or hidden active account whose last attempt is older than 60 s, that is not refreshing and not inside a rate-limit or `no_subscription` hold. An account id: force a refresh of that account now, ignoring the 60 s rule, unless it is inside a provider rate-limit hold (`retry_after`). Returns as soon as the work is scheduled; the account already has status `refreshing` in `GetState` and a `StateChanged` follows at once. Every refresh reads the account's credentials from disk again, so retrying a `signed_out` account picks up a new CLI sign-in. This is what a card's Retry button calls. |
| `RefreshNow` | `() → ()` | Force a refresh of every visible or hidden active account now, ignoring the 60 s rule, and read the local usage logs of every usage home at once. Accounts inside a provider rate-limit hold (`retry_after`) keep their hold and are skipped; `no_subscription` accounts are checked again. Returns as soon as the work is scheduled; the refreshed accounts already have status `refreshing` in `GetState` and in the next `StateChanged`. This is what a shell's refresh button calls (`headroom refresh --now`). |
| `Rescan` | `() → ()` | Run account discovery now instead of waiting for the next 10-minute pass, then refresh newly found accounts at once. Returns when the discovered accounts are stored and listed in the state; the refreshes it starts finish later. |
| `CheckForUpdates` | `() → s` | Ask GitHub for the latest Headroom release now and return the result as JSON (see [Checking on demand](#checking-on-demand)). Returns when the check is done; concurrent calls share one request, and a call within 60 s of the last check returns that result without a request. Fails with `NotSupported` on a daemon started with `--no-update-check`. |
| `GetSettings` | `() → s` | Current settings JSON (see [Settings](#settings)). |
| `SetSettings` | `(s json) → ()` | Replace the settings document. Missing fields take their defaults, unknown fields are rejected; the retired `dismissed_accounts` key is ignored (dismissals are managed only by `DismissAccount` and `RestoreAccounts`). Validated before it is stored; emits `StateChanged`. Kept for compatibility; shells should use `UpdateSettings`. |
| `UpdateSettings` | `(s patch) → ()` | Apply a JSON Merge Patch (RFC 7386) to the current settings, validate the result like `SetSettings`, store it and emit `StateChanged`. See [Updating settings](#updating-settings). |
| `ResetSettings` | `() → ()` | Restore every setting to its default except `onboarding.completed`, which keeps its value. Accounts, labels, order, hidden and dismissed accounts are not settings and stay as they are. Runs under the same lock as `SetSettings`/`UpdateSettings`, stores the result and emits `StateChanged`. Since 0.6.0. |
| `GetSpend` | `(s query) → s` | Spend and token breakdown for any period, grouped by model, project, provider or day. Implemented in 0.6, see [GetSpend](#getspend). |
| `GetDiagnostics` | `() → s` | A diagnostics report without secrets or emails, for "Copy diagnostics". Implemented in 0.6, see [GetDiagnostics](#getdiagnostics). |
| `SetAccountLabel` | `(s account_id, s label) → ()` | Set a user label. Surrounding whitespace is trimmed; an empty label clears it. At most 64 characters; control characters (C0, DEL, C1) are rejected. |
| `SetAccountOrder` | `(as ids) → ()` | Move the given accounts to the front, in that order. Accounts not listed keep their relative order after them. |
| `SetAccountHidden` | `(s account_id, b hidden) → ()` | Hide or show an account. Hidden accounts stay in the payload with `"hidden": true` but are ignored by the headline and by notifications. |
| `DismissAccount` | `(s account_id) → ()` | Stop showing a CLI-owned account (`"owner": "cli"`). The daemon records the account's current CLI home (provider, account id, home path) as dismissed, then rescans; that record leaves `accounts[]` at once, is no longer refreshed and is ignored by the headline and notifications. The CLI home and its credentials are never touched. Only that CLI record is dismissed: the same person signed in through a Headroom-owned home still shows (see [Rescan semantics](#rescan-semantics)). Dismissing an already dismissed account succeeds. Headroom-owned accounts are removed by deleting their home (`headroom accounts remove`), so dismissing one fails with `InvalidArgs`. Emits `StateChanged`. |
| `RestoreAccounts` | `(s provider) → ()` | Clear the dismissed CLI homes of one provider (`"grok"`), or of every provider with `""`, then rescan so the accounts come back. An unknown provider id fails with `InvalidArgs`. Emits `StateChanged`. |

Refresh semantics:

- A forced refresh (`Refresh(account_id)` or `RefreshNow`) while the same account is already
  refreshing does not start a second request; it
  queues exactly one follow-up refresh that starts when the current one finishes. Further requests in
  the meantime are coalesced into that follow-up.
- Scheduled refreshes run every `refresh_interval_secs` ± 10 %. Failures back off 60 s, 120 s, 240 s, …
  up to 30 min (± 10 %). A provider rate limit waits `retry_after`, or 5 min when none is given.
  `no_subscription` is not transient: the account is checked again after 1 h (± 10 %).
- Rate-limited and `no_subscription` accounts are skipped by `Refresh("")` until their next scheduled
  check; `Refresh(account_id)` still forces a check of `no_subscription` accounts and of rate-limited
  accounts whose `retry_after` has passed. `RefreshNow` skips only rate-limited accounts
  whose `retry_after` has not passed yet; it rechecks `no_subscription` accounts.
- `Refresh("")` suits automatic calls such as opening a popup; a user's explicit refresh should call
  `RefreshNow`, which also re-reads local usage logs instead of waiting for the file watcher or the
  60 s usage poll.
- Each provider call has a 30 s timeout.

### Rescan semantics

- Discovery runs every 10 minutes and on `Rescan`. A rescan also re-syncs the usage homes and resets
  the 10-minute timer.
- Rescans are coalesced: requests that arrive while a discovery is running wait for one follow-up
  discovery that starts after the current one, and all of them return when it finishes.
- Accounts found by a rescan (new ones and ones that come back) refresh immediately; accounts that
  disappeared stop being refreshed and leave `accounts[]`.
- `headroom accounts add` and `headroom accounts remove` call `Rescan`, so the change shows up at once.
- One account id can be signed in at several homes (the CLI's own home and a Headroom-owned home for
  the same person). Discovery lists every home; the daemon first drops dismissed CLI homes, then keeps
  one record per id, preferring the provider's order (usually the CLI home). So after the CLI home of
  an account is dismissed, signing the same person in through Headroom (`headroom accounts add`)
  shows the account again with `"owner": "headroom"`. `RestoreAccounts` brings the CLI home back,
  and it wins again where the provider prefers it.
- Dismissed CLI homes are kept in the daemon's database, not in the settings. Usage homes are
  independent of accounts: local usage and spend of a dismissed account's home are still read.

### Errors

| D-Bus error | when |
| --- | --- |
| `org.freedesktop.DBus.Error.InvalidArgs` | unknown account id, unknown provider id in `RestoreAccounts`, `DismissAccount` for a Headroom-owned account, duplicate id in `SetAccountOrder`, label longer than 64 characters or containing control characters, malformed or invalid settings JSON, a settings patch that is not a JSON object or whose result is invalid, a malformed or invalid `GetSpend` query |
| `org.freedesktop.DBus.Error.NotSupported` | `CheckForUpdates` on a daemon started with `--no-update-check` |
| `org.freedesktop.DBus.Error.Failed` | storage or encoding failure inside the daemon, or `Rescan` or `CheckForUpdates` while the daemon is shutting down |

The error message is human readable and safe to show.

## Signals

| signal | signature | description |
| --- | --- | --- |
| `StateChanged` | `(s state)` | Full state payload, same JSON as `GetState`. Emitted whenever the state changes, debounced by 250 ms so a burst of changes produces one signal. |
| `OpenRequested` | `()` | The user clicked a Headroom desktop notification (default action). Shells should open their popup. |

Shells should call `GetState` once on start-up and then follow `StateChanged`.

## Providers

`ListProviders` returns the provider registry so shells can build their "Add account" menu, name
providers and pick icons without special-casing any of them. `headroom providers --json` prints the
same document without a daemon.

```json
{
  "version": 1,
  "providers": [
    {
      "id": "codex",
      "display_name": "Codex",
      "add_account": [{ "kind": "cli_login", "program": "codex" }],
      "multi_account": true,
      "local_usage": true
    },
    {
      "id": "claude",
      "display_name": "Claude",
      "add_account": [{ "kind": "cli_login", "program": "claude" }],
      "multi_account": true,
      "local_usage": true
    }
  ]
}
```

| field | type | description |
| --- | --- | --- |
| `version` | integer | Schema version, currently `1`. New fields may be added without a bump. |
| `providers[].id` | string | Provider id, lowercase `[a-z0-9_-]+`. The same value as `accounts[].provider`, the prefix of account ids, and the argument of `headroom accounts add`. |
| `providers[].display_name` | string | Name to show, e.g. `Claude`. |
| `providers[].add_account` | AddAccount[] | Ways to add an account, most preferred first; the first is what `headroom accounts add <id>` does without `--api-key-stdin`. Never empty. |
| `providers[].multi_account` | bool | Several accounts of this provider can be tracked at once. |
| `providers[].local_usage` | bool | The provider reads local token logs, so it can appear in `usage[]` and `spend`. |

AddAccount, tagged by `kind`:

| kind | fields | shell action |
| --- | --- | --- |
| `cli_login` | `program` | Run `headroom accounts add <id> --progress json` (see [Adding accounts](#adding-and-removing-accounts-from-a-shell)); `program` is the CLI the login runs, useful in help text. |
| `api_key` | `label`, `console_url`, `hint` | Ask for the key (`label` names it, `console_url` is where the user creates one, `hint` describes its format), then run `headroom accounts add <id> --api-key-stdin --progress json` and write the key and a newline to its stdin. Keys never travel over D-Bus. |
| `auto_detect` | `reason` | Nothing to run; the daemon finds the account itself. Show `reason`. |

Icons: shells look up `icons/<id>.svg` in their own directory and fall back to a generic provider
icon when it is missing.

## State payload

Top level:

| field | type | description |
| --- | --- | --- |
| `version` | integer | Schema version, currently `1`. Incompatible changes bump it; new fields may be added without a bump, so ignore unknown fields. |
| `app_version` | string | Release of the daemon that assembled the payload, e.g. `"0.4.0"` (its `CARGO_PKG_VERSION`). Daemons older than this field omit it, so treat it as optional. It describes the release, not the schema; compatibility is decided by `version`. |
| `generated_at` | RFC 3339 timestamp | When the payload was assembled. Use it as "now" for countdowns. |
| `next_refresh_at` | timestamp \| null | Earliest scheduled refresh among visible accounts. Accounts that are refreshing have no schedule until they finish; `null` when nothing is scheduled. |
| `last_success_at` | timestamp \| null | `fetched_at` of the newest live snapshot of a visible account (cached snapshots from an earlier daemon run count; data read from local logs does not). `null` when there is none. |
| `offline` | bool | `true` when every listed account (hidden ones included) failed its most recent refresh with a `network` error. Any success, any other error or an account not tried yet makes it `false`. `false` without accounts. |
| `update` | Update \| null | A newer Headroom release, see [Update](#update). `null` when no newer release is known, when `updates.check` is off, when the daemon runs with `--no-update-check`, and in cached payloads read without a daemon. Daemons older than this field omit it. |
| `update_check` | UpdateCheck \| null | The last update check, see [Update check](#update-check). `null` when `updates.check` is off, when the daemon runs with `--no-update-check`, and in cached payloads read without a daemon. Daemons older than this field omit it. |
| `display` | Display | A copy of `settings.display` (see [Settings](#settings)), so shells get their display options with every `StateChanged`. |
| `headline` | Headline \| null | The one window a panel should show. `null` when no visible account has a visible window. |
| `accounts` | Account[] | Known accounts in user order (`SetAccountOrder`). Accounts that disappeared from discovery are left out; their data is kept and returns if they come back. |
| `combined` | Combined[] | Accounts of the same provider shown as one card, see [Combined accounts](#combined-accounts). Always present; empty when `display.combine_accounts` is off. Daemons older than this field omit it, so treat a missing field as `[]`. |
| `usage` | Usage[] | Local token usage, one entry per usage home the daemon reads (see [Usage](#usage)), whether or not an account belongs to it. |
| `spend` | Spend | `usage` summed across usage homes, per period and per provider. Shells show these totals as they are and never add up `usage` themselves. |

Headroom 0.6 adds `panel_items`, `panel_tone` and `provider_status` at the top level,
`collapsed` and `refresh` per account, `spend.last_7_days`, project breakdowns and
`cost_per_mtok_usd_micros`; see [0.6 payload additions](#06-payload-additions).

All timestamps are RFC 3339 strings in UTC (`2026-09-23T10:00:00Z`, fractional seconds when present).
Percentages are JSON numbers (floating point, unrounded). Token counts and money are integers; money
is always micro-USD (`12500000` = $12.50).

### Update

Present only when the latest stable release on GitHub is newer than the running daemon (semantic
version precedence; drafts and prereleases are ignored).

| field | type | description |
| --- | --- | --- |
| `version` | string | The newer release, e.g. `"0.5.0"`. |
| `url` | string | Its GitHub release page, e.g. `"https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0"`. |
| `published_at` | timestamp | When the release was published. |
| `install` | string | How this daemon's binary was installed: `self`, `package` or `unknown` (below). |
| `command` | string | One copyable line or a short sentence telling the user how to update. Shells show it as is. |

| `install` | detected when | `command` |
| --- | --- | --- |
| `self` | the binary is `<prefix>/bin/headroom` of an install receipt with `"method": "script"` (written by the release tarball's `install.sh`, which `get-headroom.sh` runs) | `headroom update` |
| `package` | the binary lives under `/usr`; the packager comes from `/usr/share/headroom/installed-by-package` (`deb`, `rpm` or `archlinux`, shipped by the release packages) | deb: `Download the new .deb package from <url>`; rpm: `Download the new .rpm package from <url>`; Arch: `Download the new Arch package from <url> and install it with sudo pacman -U`; no or another marker: `Update headroom with your system package manager` |
| `unknown` | anything else: a source build (`packaging/install.sh` writes a receipt with `"method": "source"`), `cargo run`, macOS | the release `url` |

The release packages are plain files attached to the GitHub release; there is no apt, dnf or pacman
repository, which is why package commands point at the release page.

```json
"update": {
  "version": "0.5.0",
  "url": "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0",
  "published_at": "2026-10-01T09:20:02Z",
  "install": "self",
  "command": "headroom update"
}
```

### Update check

| field | type | description |
| --- | --- | --- |
| `checked_at` | timestamp \| null | When GitHub last answered a check (a `304 Not Modified` counts), scheduled or on demand. Kept across daemon restarts. `null` before the first successful check. Failed and rate-limited checks do not change it. |

```json
"update_check": { "checked_at": "2026-09-23T04:00:00Z" }
```

A shell's "Check for updates" row shows `checked_at` ("checked 5 minutes ago") next to the running
`app_version`; its button calls `CheckForUpdates`. The row is hidden while `update_check` is `null`.

### Headline

| field | type | description |
| --- | --- | --- |
| `account_id` | string | Account the window belongs to. |
| `provider` | string | Provider id of that account (see [Providers](#providers)). |
| `provider_name` | string | Display name of that provider from the registry. |
| `account_label` | string \| null | The account's user label, else its email, else `provider_name`. `null` for a combined headline. |
| `window` | string | Window id, see [Window ids](#window-ids). |
| `window_label` | string | Same as the window's `label`. |
| `used_percent` | number | Same as the window's `used_percent`. |
| `remaining_percent` | number | Same as the window's `remaining_percent`. |
| `tone` | Tone | Same as the window's `tone`. |
| `combined` | bool | `true` when the headline is a combined window (see [Combined accounts](#combined-accounts)). Missing from older daemons: treat as `false`. |
| `account_count` | integer | Accounts behind the value: `1` for an account window, the number of `segments` for a combined window. Missing from older daemons: treat as `1`. |

For a combined headline `account_id` is the first segment's account, `account_label` is `null`, and
`used_percent` / `remaining_percent` are shares of the combined capacity on a 0–100 scale
(`remaining_percent / capacity_percent × 100` of the combined window), so a panel shows them exactly
like an account window.

Selection: hidden accounts, accounts with status `no_subscription` and hidden windows
(`display.hidden_windows`) are never chosen. With
`headline.mode = "pinned"` in settings the pinned window is used when that account is listed, not
hidden and has that window, and the window is not hidden. Otherwise (`"auto"`, or the pin is not
available) the most critical visible window wins: highest `tone`, then lowest `remaining_percent`,
then account order.

With `display.combine_accounts` on, the accounts listed in a `combined` group are not candidates on
their own: the group's combined windows take their place (at the position of the group's first
account) and compete by the same rule, using the 0–100 share. A pin that names an account inside a
group resolves to that group's combined window with the pinned window id; if the group has no such
window, auto selection applies. With the setting off, selection is exactly as above.

### Combined accounts

With `display.combine_accounts` on, the daemon adds one entry to `combined` for each provider that has
**two or more** accounts which are all of: not hidden, not dismissed, not `signed_out`, not
`no_subscription`, and have at least one window that is not hidden. An `error` or `stale` account that
still carries windows from its last snapshot is included; an account without windows is not. Accounts
that are left out keep their own card. Grouped accounts stay in `accounts[]` unchanged: a shell that
honours the setting renders one card per group instead of the accounts listed in `account_ids`, and
renders every other account as usual. Groups follow the order of their first account in `accounts[]`.

| field | type | description |
| --- | --- | --- |
| `provider` | string | Provider id shared by the accounts. |
| `provider_name` | string | Display name of that provider. |
| `account_ids` | string[] | Grouped account ids in account order. |
| `accounts` | object[] | `{account_id, label, plan}` per grouped account; `label` is the user label, else the email, else `provider_name`; `plan` may be `null`. |
| `windows` | CombinedWindow[] | One entry per window id found in the group. |

Windows are matched by `id`. A window that is hidden for an account (`display.hidden_windows`) does
not count for that account. A window id that only some accounts have still forms a combined window,
with a capacity of just those accounts. Windows are ordered as in the first account, followed by ids
that only later accounts have, in the order they are met.

CombinedWindow:

| field | type | description |
| --- | --- | --- |
| `id` | string | Window id. |
| `label` | string | Label of the window in the first account that has it. |
| `capacity_percent` | integer | `100 ×` the number of segments. |
| `used_percent` | number | Sum of the segments' `used_percent`, on the `0 … capacity_percent` scale. |
| `remaining_percent` | number | Sum of the segments' `remaining_percent`, on the same scale. |
| `resets_at` | timestamp \| null | Earliest `resets_at` among the segments. |
| `tone` | Tone | Combined tone, below. |
| `pace` | Pace | Combined pace, below. Its percents are on the `0 … capacity_percent` scale. |
| `segments` | Segment[] | One per account that has the window, in account order. |

Segment: `account_id`, `label` (as in `accounts`), and the account window's own `remaining_percent`,
`used_percent`, `resets_at` and `tone`, copied unchanged.

To draw a meter, divide by `capacity_percent` (e.g. `remaining_percent / capacity_percent × 100` for a
0–100 value and `even_pace_percent / capacity_percent` for the tick position).

Combined pace, with `C = capacity_percent` and per segment `u` = `used_percent` and
`p` = `pace.projected_percent`, or `u` when the segment has no projection (`untracked` or `spent`):

- `used = Σ u`, `projected = Σ p`, `even_pace_percent = Σ even_pace_percent` (`null` if any segment
  has none).
- `spent` when `Σ remaining_percent` rounds to 0. Otherwise `untracked` when no segment has a
  projection. Otherwise, with `P = projected / C × 100` and `U = used / C × 100`: `healthy` when
  `P ≤ 90`; `untracked` when `U < 5`; `close` when `P ≤ 100`; `running_out` above that.
- `projected_percent = projected` for `healthy`, `close` and `running_out`, else `null`.
  `spare_percent = max(0, C − projected)` for `healthy` and `close`, else `null`. `runs_out_at` is
  always `null`: the accounts do not run out together.
- Tone: `spent` → `critical`; `running_out` → `critical` when `U ≥ 90`, else `warning`; `close` →
  `warning`; `healthy` → `good`; `untracked` → by level: `critical` when `U ≥ 90`, `warning` when
  `U ≥ 80`, else `good`.
- A window only one account has keeps that account's `pace` and `tone` unchanged.

Notifications stay per account; combining never changes what is notified.

```json
"combined": [
  {
    "provider": "codex",
    "provider_name": "Codex",
    "account_ids": ["codex:work", "codex:personal"],
    "accounts": [
      { "account_id": "codex:work", "label": "Work", "plan": "Pro" },
      { "account_id": "codex:personal", "label": "Personal", "plan": "Plus" }
    ],
    "windows": [
      {
        "id": "session",
        "label": "Session",
        "capacity_percent": 200,
        "remaining_percent": 125.0,
        "used_percent": 75.0,
        "resets_at": "2026-09-23T12:00:00Z",
        "tone": "good",
        "pace": {
          "severity": "healthy",
          "even_pace_percent": 110.0,
          "projected_percent": 131.66666666666669,
          "spare_percent": 68.33333333333331,
          "runs_out_at": null
        },
        "segments": [
          { "account_id": "codex:work", "label": "Work", "remaining_percent": 45.0, "used_percent": 55.0,
            "resets_at": "2026-09-23T12:00:00Z", "tone": "warning" },
          { "account_id": "codex:personal", "label": "Personal", "remaining_percent": 80.0, "used_percent": 20.0,
            "resets_at": "2026-09-23T12:30:00Z", "tone": "good" }
        ]
      },
      {
        "id": "weekly",
        "label": "Weekly",
        "capacity_percent": 100,
        "remaining_percent": 40.0,
        "used_percent": 60.0,
        "resets_at": "2026-09-25T10:00:00Z",
        "tone": "good",
        "pace": { "severity": "healthy", "even_pace_percent": 71.42857142857143, "projected_percent": 84.0,
                  "spare_percent": 16.0, "runs_out_at": null },
        "segments": [
          { "account_id": "codex:personal", "label": "Personal", "remaining_percent": 40.0, "used_percent": 60.0,
            "resets_at": "2026-09-25T10:00:00Z", "tone": "good" }
        ]
      }
    ]
  }
]
```

Here `codex:work` hides its weekly window, so the combined weekly window has one segment. The full
payload is the snapshot test `crates/headroom-daemon/src/state/snapshots/state_combined.json`.

### Account

| field | type | description |
| --- | --- | --- |
| `id` | string | Stable id, `"{provider}:{12 hex}"`. |
| `provider` | string | Provider id, e.g. `"codex"` or `"claude"`; any id from `ListProviders`. An account stored by a build with more providers keeps its id and shows the `no_provider` error. |
| `provider_name` | string | Display name from the registry (`Codex`, `Claude`); the id itself for a provider this build lacks. Shells show it instead of hardcoding names. |
| `label` | string \| null | User label. Shells fall back to `email`, then to `provider_name`. |
| `email` | string \| null | From the last snapshot, else from storage. |
| `plan` | string \| null | Plan name as reported by the provider. `null` while the status is `no_subscription`. |
| `hidden` | bool | Hidden by the user. |
| `owner` | string | `cli` when the credentials belong to the provider's CLI home (read-only for Headroom, sign out with the CLI), `headroom` when the account was added with `headroom accounts add` and can be removed with `headroom accounts remove`. |
| `status` | Status | See below. |
| `error` | Error \| null | Last refresh error. Kept while the last good data is shown. |
| `updated_at` | timestamp \| null | Time of the data: `fetched_at` for live data, the observation time for data read from local logs. `null` when no data exists yet. |
| `source` | Source \| null | Where the shown data came from. `null` when no data exists yet. |
| `windows` | Window[] | Quota windows of the last good snapshot, in provider order. |
| `balances` | Balance[] | Credits and similar balances. |
| `notices` | Notice[] | Provider notices to show under the account. |
| `usage_home` | string | The account's own home, `~`-relative when under the user's home. Links the account to the `usage[]` entry with the same `usage_home` and provider; when that home has no logs there is no such entry. |

Status, evaluated in this order:

| value | meaning |
| --- | --- |
| `refreshing` | A refresh is in flight. |
| `no_subscription` | The account is signed in, but the provider reports no active paid plan (for example a free ChatGPT plan without Codex limits, or a Claude account without Pro/Max). `error.kind` is `no_subscription` and `error.message` says what the provider reported. The last good snapshot is dropped: `windows`, `balances` and `notices` are empty, `updated_at`, `source` and `plan` are `null`. The account never drives the headline or window notifications. The state survives daemon restarts until a refresh succeeds. |
| `signed_out` | The last refresh failed with `not_signed_in` or `sign_in_expired`. Ask the user to sign in with the CLI. |
| `error` | The last refresh failed for another reason. Any previous data is still shown. |
| `fresh` | Data is at most 10 minutes old. |
| `stale` | Data is older than 10 minutes, or there is no data yet. |

Source:

| value | meaning |
| --- | --- |
| `live` | Fetched from the provider API by this daemon run. |
| `local_log` | Read from the tool's local logs by this daemon run (the API was unavailable). |
| `cache` | Loaded from the daemon's database, not yet refreshed since start-up. |

Error:

| field | type | description |
| --- | --- | --- |
| `kind` | string | `not_signed_in`, `sign_in_expired`, `api_key_only`, `no_subscription`, `rate_limited`, `network`, `invalid_response`, `local_data`, `unsupported`, `timeout`, `no_provider` |
| `message` | string | Safe, human-readable message. Never contains tokens. |

An account without an active subscription:

```json
{
  "id": "codex:work",
  "provider": "codex",
  "provider_name": "Codex",
  "label": "Work",
  "email": "ada@example.com",
  "plan": null,
  "hidden": false,
  "owner": "cli",
  "status": "no_subscription",
  "error": {
    "kind": "no_subscription",
    "message": "No active ChatGPT subscription (Free plan)."
  },
  "updated_at": null,
  "source": null,
  "windows": [],
  "balances": [],
  "notices": [],
  "usage_home": "~/.codex"
}
```

### Window

| field | type | description |
| --- | --- | --- |
| `id` | string | See [Window ids](#window-ids). |
| `label` | string | Display label from the provider mapping (`"Session"`, `"Weekly"`, `"Opus"`, …). |
| `used_percent` | number | Percent used. Can exceed 100 on boosted plans. |
| `remaining_percent` | number | `max(0, 100 − used_percent)`. |
| `resets_at` | timestamp \| null | When the window resets. |
| `period_seconds` | integer \| null | Window length. |
| `tone` | Tone | Colour to use. Shells never compute tone themselves. |
| `pace` | Pace | Burn-rate projection. |
| `hidden` | bool | Listed in `display.hidden_windows` for this account. Hidden windows stay in the payload so a settings UI can list them, but shells should not show them, and they never drive the headline or notifications. |

Pace:

| field | type | description |
| --- | --- | --- |
| `severity` | Severity | `untracked`, `healthy`, `close`, `running_out`, `spent` |
| `even_pace_percent` | number \| null | Where an even burn would be now (position of the pace tick). Present whenever reset and period are known. |
| `projected_percent` | number \| null | Projected use at reset. `null` for `untracked` and `spent`. |
| `spare_percent` | number \| null | `100 − projected_percent`, the headroom left at reset (`~8% spare`). Only for `healthy` and `close`; `null` otherwise. |
| `runs_out_at` | timestamp \| null | Projected run-out time when it falls before the reset. |

Tone: `neutral`, `good`, `warning`, `critical` (blue accent, amber, red; neutral is grey).

#### Window ids

| id | meaning |
| --- | --- |
| `session` | 5-hour window |
| `weekly` | 7-day window |
| `model:{name}` | model-specific window, e.g. `model:opus` |
| `other:{name}` | any other provider window |

### Balance

| field | type | description |
| --- | --- | --- |
| `id` | string | Stable id, e.g. `credits`. |
| `label` | string | Display label. |
| `kind` | string | `usd`, `money` or `count`. Treat any other value as unknown and skip the row. |
| `usd_micros` | integer | Only for `kind = "usd"`: millionths of a US dollar, may be negative. |
| `currency` | string | Only for `kind = "money"`: ISO 4217 code, three upper-case letters (`CNY`, `USD`, `EUR`, …). |
| `micros` | integer | Only for `kind = "money"`: millionths of one unit of `currency`, may be negative (debt). |
| `value` | integer | Only for `kind = "count"`. |
| `unit` | string | Only for `kind = "count"`, e.g. `requests`. |

`money` is a balance in a currency the provider reports, e.g. DeepSeek and Moonshot yuan accounts:
`{ "id": "balance_cny", "label": "Balance", "kind": "money", "currency": "CNY", "micros": 12500000 }`
is ¥12.50. Providers that report US dollars keep `kind = "usd"`; `money` may also carry `USD`
(Moonshot's international platform, DeepSeek's dollar balance). Amounts in different currencies are
never summed or converted, and spend totals (`cost_usd_micros`) stay USD only. Shells format a
`money` balance with the currency's symbol when they know it (`$` USD, `¥` CNY, `€` EUR: `¥12.50`,
`-¥3.00`) and otherwise as the number followed by the code (`12.50 GBP`), rounded half away from
zero to two decimals; this is what `headroom status` prints.

### Notice

| field | type | description |
| --- | --- | --- |
| `tone` | Tone | Colour of the notice. |
| `text` | string | Text to show. |

### Usage

Local token usage belongs to a usage home (a CLI config directory with the tool's logs), not to an
account. Usage homes are discovered independently of accounts, so the list also covers homes without
an OAuth account (API-key or signed-out users) and extra config dirs signed into an account that is
already listed from another dir. Accounts that share a home share usage.

- Codex: `$CODEX_HOME` or `~/.codex` when it has `sessions/` or `archived_sessions/`, and
  Headroom-owned homes with those directories.
- Claude: `$CLAUDE_CONFIG_DIR`, `~/.claude`, config dirs found by the account scan (hidden
  directories in `~` and directories in `$XDG_CONFIG_HOME` holding `.claude.json` or
  `.credentials.json`) and Headroom-owned dirs — each only when it has a `projects/` directory.
- Paths that resolve to the same directory are listed once. The set is refreshed with every account
  discovery (every 10 minutes and on `Rescan`).
- Entries are ordered by the provider's position in `ListProviders`, then by home path.

| field | type | description |
| --- | --- | --- |
| `provider` | string | Provider id of the tool that wrote the logs. |
| `provider_name` | string | Display name of that provider. |
| `usage_home` | string | Same format as `accounts[].usage_home`. |
| `today` | Totals | Today in the daemon's local time zone. |
| `yesterday` | Totals | Yesterday. |
| `last_30_days` | Totals | Today and the 29 previous days. |
| `daily` | Daily[] | Exactly 30 entries, oldest first, ending today; days without usage are zero. |

Totals:

| field | type | description |
| --- | --- | --- |
| `tokens.input` | integer | Uncached input tokens. |
| `tokens.cache_read` | integer | Cache reads. |
| `tokens.cache_write` | integer | Cache writes (all TTLs). |
| `tokens.output` | integer | Output tokens, reasoning included. |
| `tokens.reasoning` | integer | Informational subset of `output`, not added to `total`. |
| `tokens.total` | integer | `input + cache_read + cache_write + output`. |
| `cost_usd_micros` | integer | Cost of the priced events. |
| `partial` | bool | Some events had no known price; `cost_usd_micros` excludes them. |
| `unpriced_tokens` | integer | Tokens of unpriced events. |
| `unpriced_models` | string[] | Models without a price, sorted. |
| `models` | ModelUsage[] | The top 5 models of this period, sorted as below. Empty when the period has no usage. |
| `models_other` | OtherModels \| null | The models after the top 5 added together; `null` when the period has 5 models or fewer. |

Daily: `date` (`YYYY-MM-DD`), `total_tokens`, `cost_usd_micros`, `partial`.

ModelUsage: `model` (as logged), `total_tokens`, `cost_usd_micros`, `partial` (the model has no known
price; its cost is excluded). Sorted by `cost_usd_micros` descending, then `total_tokens` descending,
then `model` ascending. `models` plus `models_other` add up exactly to the period's totals.

OtherModels: `count` (number of models folded in, at least 1), `total_tokens`, `cost_usd_micros`
(integer sums of those models), `partial` (any of them is `partial`). Shells show it as one
"N other models" row and never compute it themselves.

### Spend

| field | type | description |
| --- | --- | --- |
| `today` | PeriodSpend | Sum of `usage[].today` over every usage home, including homes without an account. Events are stored per home and a session is logged in one home only, so nothing is counted twice. |
| `yesterday` | PeriodSpend | Sum of `usage[].yesterday`. |
| `last_30_days` | PeriodSpend | Sum of `usage[].last_30_days`. |

PeriodSpend:

| field | type | description |
| --- | --- | --- |
| `cost_usd_micros` | integer | Sum of `by_provider[].cost_usd_micros`. |
| `total_tokens` | integer | Sum of `by_provider[].total_tokens`. |
| `partial` | bool | Any provider in the period is `partial`. |
| `by_provider` | ProviderSpend[] | One entry per provider with tokens or cost in the period (homes of one provider are added together), highest cost first, then by provider name. Empty when the period has no usage. |

ProviderSpend: `provider`, `provider_name`, `cost_usd_micros`, `total_tokens` (`tokens.total` summed), `partial` (any of its homes is partial), `models` (ModelUsage[]: the period's models of all that provider's homes merged by model name, summed, sorted as above and cut to the top 5), `models_other` (OtherModels \| null: the merged models after the top 5; `null` when there are 5 or fewer). Merging uses every model of every home, not the homes' own top 5, so a model that is small in each home but large in total is ranked correctly.

### Example

```json
{
  "version": 1,
  "app_version": "0.4.0",
  "generated_at": "2026-09-23T10:00:00Z",
  "next_refresh_at": "2026-09-23T10:03:00Z",
  "last_success_at": "2026-09-23T09:58:00Z",
  "offline": false,
  "update": null,
  "update_check": { "checked_at": "2026-09-23T04:00:00Z" },
  "display": {
    "theme": "system",
    "language": "system",
    "value_mode": "left",
    "reset_format": "countdown",
    "panel_label": "percent",
    "show_spend": true,
    "show_account_spend": true,
    "show_trend": true,
    "show_forecast": true,
    "translucent": false,
    "combine_accounts": false,
    "hidden_windows": { "codex:work": ["weekly"] },
    "density": "normal",
    "time_format": "auto",
    "panel_mode": "headline",
    "panel_indicator": "ring",
    "panel_limits": [],
    "panel_position": { "box": "right", "index": 0 },
    "spend_period": "30d",
    "spend_unit": "cost",
    "spend_breakdown": "models",
    "starred_accounts": [],
    "collapse_unstarred": false,
    "hide_on_screen_share": true
  },
  "headline": {
    "account_id": "claude:main",
    "provider": "claude",
    "provider_name": "Claude",
    "account_label": "ada@claude.example",
    "window": "session",
    "window_label": "Session",
    "used_percent": 92.0,
    "remaining_percent": 8.0,
    "tone": "critical",
    "combined": false,
    "account_count": 1
  },
  "accounts": [
    {
      "id": "codex:work",
      "provider": "codex",
      "provider_name": "Codex",
      "label": "Work",
      "email": "ada@example.com",
      "plan": "Pro",
      "hidden": false,
      "owner": "cli",
      "status": "fresh",
      "error": null,
      "updated_at": "2026-09-23T09:58:00Z",
      "source": "live",
      "windows": [
        {
          "id": "session",
          "label": "Session",
          "used_percent": 55.0,
          "remaining_percent": 45.0,
          "resets_at": "2026-09-23T12:00:00Z",
          "period_seconds": 18000,
          "tone": "warning",
          "pace": {
            "severity": "close",
            "even_pace_percent": 60.0,
            "projected_percent": 91.66666666666667,
            "spare_percent": 8.333333333333329,
            "runs_out_at": null
          },
          "hidden": false
        },
        {
          "id": "weekly",
          "label": "Weekly",
          "…": "same shape as the session window",
          "hidden": true
        }
      ],
      "balances": [
        { "id": "credits", "label": "Credits", "kind": "usd", "usd_micros": 12500000 }
      ],
      "notices": [],
      "usage_home": "~/.codex"
    },
    {
      "id": "claude:main",
      "provider": "claude",
      "provider_name": "Claude",
      "label": null,
      "email": "ada@claude.example",
      "plan": "Pro",
      "hidden": false,
      "owner": "cli",
      "status": "signed_out",
      "error": {
        "kind": "sign_in_expired",
        "message": "sign-in expired, open the CLI to sign in again"
      },
      "updated_at": "2026-09-23T09:00:00Z",
      "source": "cache",
      "windows": [
        {
          "id": "session",
          "label": "Session",
          "used_percent": 92.0,
          "remaining_percent": 8.0,
          "resets_at": "2026-09-23T10:30:00Z",
          "period_seconds": 18000,
          "tone": "critical",
          "pace": {
            "severity": "running_out",
            "even_pace_percent": 90.0,
            "projected_percent": 102.22222222222221,
            "spare_percent": null,
            "runs_out_at": "2026-09-23T10:23:28.695652174Z"
          },
          "hidden": false
        }
      ],
      "balances": [],
      "notices": [],
      "usage_home": "~/.claude"
    }
  ],
  "combined": [],
  "usage": [
    {
      "provider": "codex",
      "provider_name": "Codex",
      "usage_home": "~/.codex",
      "today": {
        "tokens": { "input": 1000, "cache_read": 0, "cache_write": 0, "output": 200, "reasoning": 0, "total": 1200 },
        "cost_usd_micros": 2400,
        "partial": false,
        "unpriced_tokens": 0,
        "unpriced_models": [],
        "models": [
          { "model": "gpt-5.5", "total_tokens": 1200, "cost_usd_micros": 2400, "partial": false }
        ],
        "models_other": null
      },
      "yesterday": { "…": "same shape as today" },
      "last_30_days": { "…": "same shape as today" },
      "daily": [
        { "date": "2026-08-25", "total_tokens": 0, "cost_usd_micros": 0, "partial": false },
        { "date": "2026-09-23", "total_tokens": 1200, "cost_usd_micros": 2400, "partial": false }
      ]
    }
  ],
  "spend": {
    "today": {
      "cost_usd_micros": 12400,
      "total_tokens": 6200,
      "partial": false,
      "by_provider": [
        {
          "provider": "claude", "provider_name": "Claude", "cost_usd_micros": 10000, "total_tokens": 5000, "partial": false,
          "models": [
            { "model": "claude-opus", "total_tokens": 5000, "cost_usd_micros": 10000, "partial": false }
          ],
          "models_other": null
        },
        {
          "provider": "codex", "provider_name": "Codex", "cost_usd_micros": 2400, "total_tokens": 1200, "partial": false,
          "models": [
            { "model": "gpt-5.5", "total_tokens": 1200, "cost_usd_micros": 2400, "partial": false }
          ],
          "models_other": null
        }
      ]
    },
    "yesterday": { "…": "same shape as today" },
    "last_30_days": { "…": "same shape as today" }
  }
}
```

The complete payload this example is cut from is the snapshot test
`crates/headroom-daemon/src/state/snapshots/state_full.json`.

## Settings

`GetSettings` returns and `SetSettings` accepts one JSON document. Missing fields (at any level) take
their defaults, so `SetSettings` always replaces the whole document. Unknown fields at any level and
unknown enum values are rejected with `InvalidArgs`. A successful `SetSettings` emits `StateChanged`
(the state carries `display` and the windows' `hidden` flags).

| field | type | default | description |
| --- | --- | --- | --- |
| `refresh_interval_secs` | integer | `300` | Scheduled refresh interval, `60`–`3600`. Applies from each account's next scheduled refresh. |
| `adaptive_refresh` | bool | `true` | Since 0.6.0. While a provider's CLI is writing local logs, refresh that provider's accounts every 60 s (same backoff and rate-limit holds); after 10 minutes without log activity return to `refresh_interval_secs`. Reported per account as `accounts[].refresh` (see [0.6 payload additions](#06-payload-additions)). |
| `notifications.almost_out` | bool | `true` | Notify when a window drops under the threshold (`notifications.threshold_percent`, or the provider's own threshold). |
| `notifications.cutting_it_close` | bool | `true` | Notify when pace rises to `close`. |
| `notifications.will_run_out` | bool | `true` | Notify when pace rises to `running_out` or `spent`. |
| `notifications.reset` | bool | `false` | Notify when a window that was `warning` or worse resets. |
| `notifications.threshold_percent` | integer | `10` | Since 0.6.0. Remaining percent under which `almost_out` fires, `1`–`50` (settings UIs offer 5, 10, 20, 30). It re-arms when remaining climbs back to the threshold + 5. |
| `notifications.provider_thresholds` | object | `{}` | Since 0.6.0. Provider id → threshold `0`–`50` that replaces `threshold_percent` for every account of that provider, e.g. `{"claude":20,"copilot":0}`. `0` turns window alerts off for the provider; a missing key uses `threshold_percent`. Provider ids must be non-empty; ids this build does not know are kept. Never `null` inside a stored document (in a patch `null` deletes the key). |
| `notifications.quiet_hours` | object | see below | Since 0.6.0. `{"enabled":false,"from":"22:00","to":"08:00","allow_critical":true}`. `from`/`to` are `HH:MM` in the daemon's local time zone (two digits each, `00:00`–`23:59`); a range with `from` later than `to` crosses midnight. With `enabled`, `from` and `to` must differ. Alerts that fall inside the range are held and delivered as one summary when it ends; held alerts whose window has reset meanwhile are dropped. With `allow_critical` alerts with `critical` urgency are delivered at once. Every field is optional in a patch. |
| `headline` | object | `{"mode":"auto"}` | `{"mode":"auto"}` or `{"mode":"pinned","account_id":"codex:…","window":"session"}`. A pin needs a non-empty account id and window. |
| `reduced_motion` | bool | `false` | Shells disable animations. |
| `display.theme` | string | `"system"` | `system`, `light` or `dark`. Shells follow the system theme unless forced. |
| `display.language` | string | `"system"` | `system`, `en` or `ru`. Language of the daemon's notifications (and of shells that are translated). `system` resolves from `LC_ALL`, then `LC_MESSAGES`, then `LANG` of the daemon: a value starting with `ru` means Russian, anything else English. |
| `display.value_mode` | string | `"left"` | `left` shows remaining percent, `used` shows used percent. |
| `display.reset_format` | string | `"countdown"` | `countdown` (`resets in 2h 5m`) or `exact` (reset time of day / date). |
| `display.panel_label` | string | `"percent"` | What the panel shows next to the indicator: `percent`, `window` (the item's window label) or, since 0.6.0, `none` (indicator only: `ring` + `none` is "ring only"). |
| `display.panel_mode` | string | `"headline"` | Since 0.6.0. `headline` (one limit, as before), `several` (up to 3 limits from `display.panel_limits`) or `icon` (only the Headroom mark, tinted with `panel_tone`). The daemon resolves it into `panel_items` (see [0.6 payload additions](#06-payload-additions)). |
| `display.panel_indicator` | string | `"ring"` | Since 0.6.0. How each panel item draws its level: `ring`, `bar` (26×5 mini meter with the even-pace tick) or `none`. |
| `display.panel_limits` | array | `[]` | Since 0.6.0. The limits shown in `several` mode, in order: 0–3 objects `{"account_id":"claude:…","window":"session"}`, both non-empty and required, no other fields. Duplicates are dropped (first kept); more than 3 distinct entries is invalid. Empty = the daemon picks the 2 most critical windows. Account ids that are not listed right now are kept. |
| `display.panel_position` | object | `{"box":"right","index":0}` | Since 0.6.0. Where the GNOME indicator sits: `box` is `left`, `center` or `right`, `index` an integer ≥ 0 (position inside the box). Both fields are required when the object is given. Other shells ignore it. |
| `display.density` | string | `"normal"` | Since 0.6.0. `normal` or `compact` popup layout. |
| `display.time_format` | string | `"auto"` | Since 0.6.0. `auto` (follow the desktop or locale), `12h` or `24h`, for every clock time a shell shows. |
| `display.spend_period` | string | `"30d"` | Since 0.6.0. The spend card's selected period: `today`, `yesterday`, `7d` (`spend.last_7_days`) or `30d` (`spend.last_30_days`). |
| `display.spend_unit` | string | `"cost"` | Since 0.6.0. The spend card's unit: `cost`, `tokens` or `cost_per_mtok` (`cost_per_mtok_usd_micros`). |
| `display.spend_breakdown` | string | `"models"` | Since 0.6.0. What the spend legend breaks down: `models` or `projects`. |
| `display.starred_accounts` | string[] | `[]` | Since 0.6.0. Account ids that are always expanded in the popup. Non-empty ids; duplicates dropped (first kept); ids not listed right now are kept. |
| `display.collapse_unstarred` | bool | `false` | Since 0.6.0. Collapse accounts that are not starred and not at `warning` or worse into one "N more" row. The daemon decides per account (`accounts[].collapsed`). |
| `display.hide_on_screen_share` | bool | `true` | Since 0.6.0. Shells that can detect screen sharing replace figures with `••` while the screen is shared. |
| `display.show_spend` | bool | `true` | Show the spend section. |
| `display.show_account_spend` | bool | `true` | Show local spend under each account card. |
| `display.show_trend` | bool | `true` | Show the 30-day trend. |
| `display.show_forecast` | bool | `true` | Show pace forecasts (`~8% spare`, `limit in 23m`). |
| `display.translucent` | bool | `false` | Shells render the popup with a translucent (blurred where supported) background instead of an opaque one. |
| `display.combine_accounts` | bool | `false` | Show accounts of the same provider as one card with summed windows. The daemon fills the state's `combined` list and picks the headline from combined windows (see [Combined accounts](#combined-accounts)); off leaves `combined` empty. |
| `updates.check` | bool | `true` | Check GitHub once a day for a newer Headroom release and report it as the state's `update` (see [Update checks](#update-checks)). `false` stops the requests and hides `update` at once. |
| `display.hidden_windows` | object | `{}` | Map of account id → array of window ids to hide, e.g. `{"codex:1a2b3c4d5e6f":["weekly","model:spark"]}`. Ids must be non-empty; duplicates in a list are dropped (first occurrence kept). Account ids that are not currently listed are allowed and kept. |
| `status_pages.enabled` | bool | `false` | Since 0.6.0. Poll the public status pages of the providers that have accounts and report incidents as `provider_status` (see [0.6 payload additions](#06-payload-additions)). Off by default because it sends new network requests: one `GET` of each provider's public Statuspage `summary.json` every 5–10 minutes, without identifiers, accounts or usage. |
| `shortcuts.open` | string | `""` | Since 0.6.0. Global shortcut that opens the popup, in GTK accelerator syntax (`<Super>u`, `<Control><Alt>h`): any number of `<Modifier>` groups (ASCII letters) followed by a key name of ASCII letters, digits and `_`. At most 64 characters. `""` disables it. Each shell registers it with its own platform API. |
| `logging.level` | string | `"info"` | Since 0.6.0. Daemon log level: `error`, `warn`, `info` or `debug`. Applied without a restart; `RUST_LOG` wins when set. |
| `onboarding.completed` | bool | `false` | Since 0.6.0. `false` until the user finishes (or skips for good) the first-run window. Settings stored by a daemon older than 0.6.0 load with `true`, so existing users never see onboarding; a fresh install starts with `false`. `ResetSettings` keeps it. |

Dismissed accounts are not a setting: `DismissAccount` and `RestoreAccounts` manage them in the
daemon's database. A `dismissed_accounts` key sent by older clients to `SetSettings` or
`UpdateSettings` is ignored, and a list stored by an older daemon is migrated once: each id that
names a CLI-owned account becomes that account's dismissed CLI home; other ids are dropped.

```json
{
  "refresh_interval_secs": 300,
  "adaptive_refresh": true,
  "notifications": {
    "almost_out": true,
    "cutting_it_close": true,
    "will_run_out": true,
    "reset": false,
    "threshold_percent": 10,
    "provider_thresholds": {},
    "quiet_hours": { "enabled": false, "from": "22:00", "to": "08:00", "allow_critical": true }
  },
  "headline": { "mode": "auto" },
  "reduced_motion": false,
  "display": {
    "theme": "system",
    "language": "system",
    "value_mode": "left",
    "reset_format": "countdown",
    "panel_label": "percent",
    "show_spend": true,
    "show_account_spend": true,
    "show_trend": true,
    "show_forecast": true,
    "translucent": false,
    "combine_accounts": false,
    "hidden_windows": {},
    "density": "normal",
    "time_format": "auto",
    "panel_mode": "headline",
    "panel_indicator": "ring",
    "panel_limits": [],
    "panel_position": { "box": "right", "index": 0 },
    "spend_period": "30d",
    "spend_unit": "cost",
    "spend_breakdown": "models",
    "starred_accounts": [],
    "collapse_unstarred": false,
    "hide_on_screen_share": true
  },
  "updates": { "check": true },
  "status_pages": { "enabled": false },
  "shortcuts": { "open": "" },
  "logging": { "level": "info" },
  "onboarding": { "completed": false }
}
```

Compatibility: every settings object rejects unknown fields, so a daemon older than 0.6.0 rejects a
patch that names any key marked "Since 0.6.0" (and `"none"` for `display.panel_label`). Clients send
these keys only when the state's `app_version` is `0.6.0` or later and hide the matching controls
otherwise. Reading is always safe: older daemons simply omit the keys, so treat a missing key as its
default. "Off" is never encoded as `null` (a `null` in a patch means "back to the default"), which is
why a provider threshold of `0` turns alerts off.

`SetSettings` replaces the whole document, `onboarding.completed` included: a client that uses it must
send the current `onboarding` object back, or the first-run window shows again.

### Updating settings

`UpdateSettings(s patch)` changes only the fields named in `patch`, following JSON Merge Patch
(RFC 7386):

- `patch` must be a JSON object.
- Objects merge recursively: `{"display":{"theme":"dark"}}` changes the theme and keeps every other
  field.
- `null` deletes a key, so the field falls back to its default: `{"refresh_interval_secs":null}` →
  `300`, `{"display":null}` → all display defaults.
- `display.hidden_windows` merges per account id: `{"display":{"hidden_windows":{"codex:1a2b":["weekly"]}}}`
  sets that account's list (other accounts keep theirs) and `{"display":{"hidden_windows":{"codex:1a2b":null}}}`
  removes that account's entry.
- `notifications.provider_thresholds` merges per provider id the same way:
  `{"notifications":{"provider_thresholds":{"claude":20}}}` sets Claude's threshold and keeps the
  others, `{"notifications":{"provider_thresholds":{"claude":null}}}` removes Claude's entry (back to
  `threshold_percent`), and `{"notifications":{"provider_thresholds":null}}` removes them all. To turn
  a provider's alerts off send `0`, never `null`.
- `notifications.quiet_hours` is an object and merges field by field:
  `{"notifications":{"quiet_hours":{"enabled":true}}}` keeps the stored `from`, `to` and
  `allow_critical`; `{"notifications":{"quiet_hours":{"from":null}}}` resets only `from` to `22:00`.
- Arrays are replaced whole, e.g. one account's hidden window list, `display.panel_limits` and
  `display.starred_accounts`: send the complete new list (`[]` empties it, `null` resets it to `[]`).
- `headline` is a tagged union and is replaced whole when the patch gives it an object:
  `{"headline":{"mode":"auto"}}` unpins even though the current value has `account_id` and `window`.
  `{"headline":null}` also resets it to auto.
- `display.panel_position` is replaced whole when the patch gives it an object, so the object must
  carry both `box` and `index`: `{"display":{"panel_position":{"box":"left","index":2}}}`.
  `{"display":{"panel_position":null}}` resets it to `{"box":"right","index":0}`.
- The result is validated like `SetSettings` (unknown fields, unknown enum values, ranges). Deleting a
  key that does not exist is a no-op.

The read, merge, validation and store run inside the daemon under one lock that `SetSettings` shares,
so concurrent patches from several shells never lose each other's changes. An invalid patch or an
invalid result fails with `InvalidArgs` and leaves the settings unchanged; a successful patch emits
`StateChanged` even if nothing changed.

```
UpdateSettings('{"display":{"translucent":true,"hidden_windows":{"claude:9f8e":null}}}')
UpdateSettings('{"display":{"panel_mode":"several","panel_limits":[{"account_id":"claude:9f8e","window":"session"},{"account_id":"codex:1a2b","window":"weekly"}]}}')
UpdateSettings('{"notifications":{"threshold_percent":20,"provider_thresholds":{"copilot":0},"quiet_hours":{"enabled":true}}}')
```

`ResetSettings()` is the "Reset all settings…" action: it is equivalent to `SetSettings` with a
document that holds only the current `onboarding` object, and it never touches accounts.

The former top-level `show_usage` is gone. Settings stored by an older daemon are migrated on load
(`show_usage` becomes `display.show_spend` unless that is set), but `SetSettings` with `show_usage` is
invalid.

## 0.6 payload additions

The fields and methods below are the contract for Headroom 0.6. Each is marked with its status:
**implemented in 0.6** means the daemon of the 0.6.0 release serves it exactly as written here, even
where the current development build does not yet. All of them are additive: `version` stays `1`,
older daemons omit them, and clients must treat a missing field as described in its row. A client
checks `app_version` ≥ `0.6.0` before calling a new method; older daemons answer `UnknownMethod`
(`-32601` on the socket).

Summary:

| where | field or method | status |
| --- | --- | --- |
| state, top level | `panel_items`, `panel_tone` | implemented in 0.6 |
| state, top level | `provider_status` | implemented in 0.6 |
| state, top level | `update_check` | defined by the update-check work, see [Update checks](#update-checks) |
| `accounts[]` | `collapsed`, `refresh` | implemented in 0.6 |
| `accounts[]` | `recovery` | defined by the account-recovery work, see [Account](#account) |
| `spend` | `last_7_days` | implemented in 0.6 |
| PeriodSpend | `projects`, `projects_other` | implemented in 0.6 |
| PeriodSpend, ProviderSpend, ModelUsage | `cost_per_mtok_usd_micros` | implemented in 0.6 |
| `ListProviders` | `providers[].links` | implemented in 0.6 |
| methods | `GetSpend`, `GetDiagnostics` | implemented in 0.6 |
| methods | `ResetSettings` | implemented (see [Methods](#methods)) |
| methods | `CheckForUpdates` | implemented (see [Checking on demand](#checking-on-demand)) |

### Panel items

`panel_items` (PanelItem[], always present) is what a panel indicator draws, already resolved from
`display.panel_mode` and `display.panel_limits`; shells never pick limits themselves.
`panel_tone` (Tone | null) is the worst tone among the visible windows of visible accounts
(`critical` > `warning` > `good` > `neutral`), used to tint the mark in `icon` mode; `null` when no
visible account has a visible window. Both are computed in every mode.

| `panel_mode` | `panel_items` |
| --- | --- |
| `headline` | The headline as one item, or `[]` when `headline` is `null`. |
| `several` | The entries of `display.panel_limits` in order, each resolved like a pinned headline (skipped when the account is not listed, hidden, `no_subscription`, lacks the window or the window is hidden; a limit inside a combined group resolves to the combined window). An empty `panel_limits` yields the 2 most critical windows by the headline's auto rule. At most 3 items; `[]` when nothing resolves. |
| `icon` | `[]`. The shell draws only the Headroom mark tinted with `panel_tone`. |

PanelItem has the [Headline](#headline) fields plus:

| field | type | description |
| --- | --- | --- |
| `provider`, `provider_name`, `account_id`, `account_label`, `window`, `window_label`, `used_percent`, `remaining_percent`, `tone`, `combined`, `account_count` | | Exactly as in [Headline](#headline). |
| `value_percent` | number | `remaining_percent` when `display.value_mode` is `left`, `used_percent` when `used`; the number a panel prints. |
| `even_pace_percent` | number \| null | The window's `pace.even_pace_percent` (for a combined window divided by `capacity_percent` × 100, so always 0–100), for the `bar` indicator's tick. |
| `logo` | string | Icon key: shells load `icons/<logo>.svg` and fall back to the generic provider icon. Currently the provider id. |

```json
"panel_tone": "warning",
"panel_items": [
  {
    "account_id": "claude:9f8e7d6c5b4a", "provider": "claude", "provider_name": "Claude",
    "account_label": "ada@claude.example", "window": "weekly", "window_label": "Weekly",
    "used_percent": 28.0, "remaining_percent": 72.0, "value_percent": 72.0, "tone": "good",
    "even_pace_percent": 41.5, "logo": "claude", "combined": false, "account_count": 1
  },
  {
    "account_id": "codex:1a2b3c4d5e6f", "provider": "codex", "provider_name": "Codex",
    "account_label": "Work", "window": "weekly", "window_label": "Weekly",
    "used_percent": 60.0, "remaining_percent": 40.0, "value_percent": 40.0, "tone": "warning",
    "even_pace_percent": 52.0, "logo": "codex", "combined": false, "account_count": 1
  }
]
```

Older daemons: no `panel_items`; build one item from `headline`.

### Account additions

| field | type | description |
| --- | --- | --- |
| `collapsed` | bool | `true` when `display.collapse_unstarred` is on, the account is not in `display.starred_accounts`, and none of its visible windows has tone `warning` or `critical`. Always `false` when the setting is off. Shells fold collapsed accounts into one "N more · Copilot, Grok ›" row in account order. Missing: `false`. |
| `refresh` | Refresh | How the daemon is polling this account now. Missing: show `next_refresh_at` only. |
| `recovery` | object \| null | What a card's primary button does after an error (`retry`, `sign_in`, `cli_login`); defined in [Account](#account) by the account-recovery work, not redefined here. |

Refresh:

| field | type | description |
| --- | --- | --- |
| `mode` | string | `live` while `adaptive_refresh` is on and the provider's CLI wrote local logs in the last 10 minutes; `idle` otherwise. |
| `interval_secs` | integer | The interval in effect: `60` in `live` mode, `refresh_interval_secs` in `idle` mode. |
| `next_at` | timestamp \| null | This account's next scheduled refresh; `null` while it is refreshing or has no schedule. |
| `reason` | string | Why `next_at` is what it is: `activity` (live mode), `schedule` (the normal interval), `backoff` (after failures) or `hold` (provider rate limit or `no_subscription` check). |

```json
"refresh": { "mode": "live", "interval_secs": 60, "next_at": "2026-09-23T10:01:00Z", "reason": "activity" }
```

### Provider status

`provider_status` (ProviderStatus[], always present) lists public status-page state for providers
that have at least one listed account. Empty when `status_pages.enabled` is off or no status page has
been read yet, and for providers without a known status page. Ordered like `ListProviders`. Missing:
`[]`.

| field | type | description |
| --- | --- | --- |
| `provider` | string | Provider id. |
| `indicator` | string | Statuspage indicator of the components Headroom follows: `none`, `minor`, `major`, `critical` or `maintenance`. Shells show nothing for `none`. |
| `tone` | Tone | `neutral` for `none`, `warning` for `minor` and `maintenance`, `critical` for `major` and `critical`. |
| `title` | string \| null | Name of the current incident or maintenance; `null` for `none`. |
| `stage` | string \| null | Its latest stage as reported (`investigating`, `identified`, `monitoring`, `scheduled`, `in_progress`, `verifying`); `null` for `none`. |
| `started_at` | timestamp \| null | When the incident or maintenance started. |
| `url` | string | `https` link to the incident, or to the status page for `none`. |

```json
"provider_status": [
  { "provider": "claude", "indicator": "minor", "tone": "warning", "title": "Elevated errors on Claude Opus",
    "stage": "identified", "started_at": "2026-09-23T09:12:00Z", "url": "https://status.anthropic.com/incidents/abc123" }
]
```

### Spend additions

- `spend.last_7_days` (PeriodSpend): today and the 6 previous days in the daemon's local time zone,
  summed over every usage home exactly like the other periods. Missing: hide the `7d` choice.
- `projects` (ProjectSpend[]) and `projects_other` (OtherProjects | null) on every PeriodSpend
  (`spend.today`, `yesterday`, `last_7_days`, `last_30_days`). A project is the working directory an
  event was logged in (Claude and Codex logs), `~`-relative when under the user's home; events without
  one form the project `null` ("No project"). Projects are sorted like models (cost descending, then
  tokens descending, then name). Listed are the projects whose share is at least 50 ‰, at most 5; the
  rest are added together in `projects_other` (`null` when nothing was folded). `projects` plus
  `projects_other` add up exactly to the period's totals. Missing: hide the projects breakdown.
- `cost_per_mtok_usd_micros` (integer | null) on PeriodSpend, ProviderSpend and ModelUsage: the priced
  cost per million priced tokens, `cost_usd_micros × 1 000 000 / priced total tokens`, rounded half up
  to a whole micro-USD. Unpriced tokens are never in the denominator. `null` when there are no priced
  tokens.

ProjectSpend:

| field | type | description |
| --- | --- | --- |
| `project` | string \| null | `~/code/headroom`; `null` for events without a working directory. |
| `cost_usd_micros` | integer | Cost of the project's priced events. |
| `total_tokens` | integer | `tokens.total` of its events. |
| `partial` | bool | Some of its events had no known price. |
| `share_permille` | integer | Share of the period's `cost_usd_micros`, `cost × 1000 / period cost` rounded down; by `total_tokens` instead when the period's cost is `0`. |
| `by_provider` | object[] | `{provider, provider_name, cost_usd_micros, total_tokens}` per provider that logged in this project, sorted like `by_provider`, for the provider-coloured bar. |

OtherProjects: `count` (projects folded in, at least 1), `cost_usd_micros`, `total_tokens`,
`partial`, `share_permille` (computed like ProjectSpend).

```json
"projects": [
  { "project": "~/code/headroom", "cost_usd_micros": 9100000, "total_tokens": 48100000, "partial": false,
    "share_permille": 733,
    "by_provider": [
      { "provider": "claude", "provider_name": "Claude", "cost_usd_micros": 8000000, "total_tokens": 40000000 },
      { "provider": "codex", "provider_name": "Codex", "cost_usd_micros": 1100000, "total_tokens": 8100000 }
    ] }
],
"projects_other": { "count": 4, "cost_usd_micros": 3300000, "total_tokens": 9000000, "partial": false, "share_permille": 266 },
"cost_per_mtok_usd_micros": 217163
```

### Provider links

`ListProviders` adds `providers[].links` (object, always present in 0.6): `status`, `dashboard` and
`usage`, each an absolute `https://` URL or `null` when the provider has none. Shells open them with
the desktop's URL handler and never build URLs themselves. Missing: no link actions.

```json
"links": {
  "status": "https://status.anthropic.com",
  "dashboard": "https://claude.ai/settings",
  "usage": "https://claude.ai/settings/usage"
}
```

### GetSpend

`GetSpend(s query) → s` returns a spend breakdown for `headroom spend` and similar clients. It reads
the same stored usage events as `spend`, in the daemon's local time zone; dates are inclusive.

Query (JSON object, unknown fields rejected):

| field | type | description |
| --- | --- | --- |
| `period` | string | `today`, `yesterday`, `7d` or `30d`. Either `period` or `since` is required, not both. |
| `since` | string | `YYYY-MM-DD`, first day. |
| `until` | string | `YYYY-MM-DD`, last day, only with `since`; default today. Must not be before `since`. |
| `by` | string | `model`, `project`, `provider` or `day`. |
| `provider` | string | Optional provider id filter; an unknown id is invalid. |

Result:

| field | type | description |
| --- | --- | --- |
| `since`, `until` | string | The resolved inclusive date range. |
| `by` | string | As requested. |
| `rows` | SpendRow[] | One row per group, sorted by cost descending, then tokens descending, then key; `day` rows are sorted by date ascending and include days without usage. Not cut to a top N. |
| `total` | SpendRow | The sum of all rows, with `key` `null`. |

SpendRow: `key` (model name, project path or `null`, provider id, or date), `provider` (the
provider id for `model` and `provider` rows, else `null`), `tokens` (the Totals `tokens` object:
`input`, `cache_read`, `cache_write`, `output`, `reasoning`, `total`), `cost_usd_micros`, `partial`,
`unpriced_tokens`, `cost_per_mtok_usd_micros` (as above), `sessions` (distinct logged sessions),
`share_permille` (of the result's total cost, as for projects).

```
GetSpend('{"since":"2026-09-17","by":"model","provider":"claude"}')
```

```json
{
  "since": "2026-09-17", "until": "2026-09-23", "by": "model",
  "rows": [
    { "key": "claude-opus-4-5", "provider": "claude",
      "tokens": { "input": 1200000, "cache_read": 180000000, "cache_write": 9000000, "output": 2100000, "reasoning": 0, "total": 192300000 },
      "cost_usd_micros": 151200000, "partial": false, "unpriced_tokens": 0, "cost_per_mtok_usd_micros": 786271,
      "sessions": 41, "share_permille": 767 }
  ],
  "total": { "key": null, "provider": null, "…": "same fields, summed" }
}
```

### GetDiagnostics

`GetDiagnostics() → s` returns a report for bug reports. It never contains tokens, API keys, emails,
account labels or home paths outside `~`-relative form.

| field | type | description |
| --- | --- | --- |
| `app_version` | string | Daemon release. |
| `os` | string | OS name and version (`/etc/os-release` `PRETTY_NAME`, or the macOS version). |
| `desktop` | string \| null | `XDG_CURRENT_DESKTOP` and session type, e.g. `GNOME (wayland)`. |
| `transports` | string[] | `dbus`, `socket`. |
| `log_level` | string | Effective level (`error`, `warn`, `info`, `debug`). |
| `log_level_source` | string | `settings` (`logging.level`) or `env` (`RUST_LOG` is set and wins). |
| `log_file` | string | Path of the daemon's log file, `~`-relative. |
| `providers` | object[] | `{provider, accounts, usage_homes}` counts per provider. |
| `accounts` | object[] | `{provider, status, error_kind, source, updated_at, refresh_mode}` per account, in account order, without ids. |
| `text` | string | The same report as plain text, ready for "Copy diagnostics". Shells copy it as is. |

### Other 0.6 methods

- `ResetSettings() → ()`: see [Methods](#methods).
- `CheckForUpdates() → s` and the state's `update_check`: defined in [Update checks](#update-checks)
  by the update-check work; this section does not redefine them.

## Update checks

The daemon asks GitHub whether a newer Headroom release exists and reports it as `update` in the
state payload. Privacy: this is one `GET https://api.github.com/repos/daniarjabagin/headroom/releases/latest`
a day, plus one per `CheckForUpdates` call (at most one a minute), sent with
`User-Agent: headroom/<version>`, `Accept: application/vnd.github+json` and the previous response's
`ETag` as `If-None-Match`. Nothing else is sent: no identifiers, accounts, usage or settings. GitHub sees the request's IP address like any web request.

- The first check runs 2 minutes after start-up, or 24 h (± 10 %) after the last successful check if
  that is later; the last check (time, `ETag`, latest release) is stored in the daemon's database, so
  restarts do not check again. After a check the next one follows 24 h (± 10 %) later.
- `304 Not Modified` keeps the stored release. `403` and `429` are treated as GitHub's rate limit:
  the next attempt waits for `Retry-After` or `X-RateLimit-Reset`, at least 1 h and at most 24 h.
  Network errors, other statuses and unusable answers are logged at `debug` only and retried after
  1 h (± 10 %); they never show in the UI.
- `updates.check: false` stops the requests and hides `update`; turning it on again checks at once
  if a check is due. `headroom daemon --no-update-check` never checks (the macOS app starts its daemon
  this way because it updates itself with Sparkle).
- A change of `update` or `update_check` emits `StateChanged`.

### Checking on demand

`CheckForUpdates()` (socket: `CheckForUpdates`) runs a check now for a shell's "Check for updates"
button. It sends the same request as the daily check (with the stored `ETag`, so an unchanged
release costs a `304`) and returns:

```json
{"status":"available","checked_at":"2026-09-23T10:00:00Z","version":"0.6.0"}
{"status":"rate_limited","checked_at":"2026-09-22T09:14:00Z","version":"0.5.1","until":"2026-09-23T11:30:00Z"}
```

| field | type | description |
| --- | --- | --- |
| `status` | string | `up_to_date`: the latest stable release is not newer than the running daemon. `available`: it is newer; the state's `update` describes it. `failed`: the request failed (network, unexpected status, unusable answer). `rate_limited`: GitHub's rate limit holds checks until `until`. `disabled`: `updates.check` is `false`; nothing was sent. |
| `checked_at` | timestamp \| null | The last successful check, the same value as the state's `update_check.checked_at` (so for `failed` and `rate_limited` it is the previous success). `null` for `disabled` and before the first success. |
| `version` | string \| null | The latest stable release known from that check, newer or not. `null` for `disabled` and when none is known. |
| `until` | timestamp | Present only for `rate_limited`: when the next request may be sent. |

- Calls that arrive together or while a check is running share one request and get the same result.
- A call within 60 s of the last check (scheduled or on demand) returns that check's result without a
  request.
- While a rate-limit hold (`403`/`429`) is active, calls return `rate_limited` without a request.
- A check that reaches GitHub moves the daily schedule: the next scheduled check follows 24 h
  (± 10 %) later, or 1 h (± 10 %) after a failure, or at the end of a rate-limit hold.
- The request may take up to 30 s plus the connection time; D-Bus clients should call it
  asynchronously with a timeout of at least 60 s (`gdbus` and `QDBus` default to 25 s).
- A daemon started with `--no-update-check` answers with `org.freedesktop.DBus.Error.NotSupported`
  (socket: `-32000`); `headroom update --check` still works there because it asks GitHub itself.

## Updating Headroom

```
headroom update --check
headroom update [--yes] [--progress json]
```

`--check` fetches the latest release directly (no daemon needed) and prints the running and the
latest version with the command that updates this install.

Without `--check`, `headroom update` fetches the latest release and, when it is newer:

- For a `self` install it asks for confirmation (`--yes` skips it; without a terminal or with
  `--progress json` it fails unless `--yes` is given), downloads `SHA256SUMS` and
  `SHA256SUMS.sig` from the release and checks the Ed25519 signature against the release key built
  into the binary (a missing or invalid signature fails the update), then downloads
  `headroom-<version>-<arch>-linux-musl.tar.gz`, checks its SHA-256,
  unpacks it into a temporary directory and runs its `install.sh` with the options recorded in the
  install receipt. That replaces the binary, icons, unit, D-Bus file, GNOME extension and Plasma
  widget and restarts `headroom.service`. When the receipt records a Headroom tray
  (`"tray": "linux-gnu"` or `"linux-gnu-layershell"`, written by `install.sh --tray`), it also
  downloads `headroom-tray-<version>-<arch>-<variant>.tar.gz`, checks it against the same signed
  `SHA256SUMS`, unpacks it into the bundle's `tray/` directory and the recorded `--tray` option
  reinstalls `~/.local/bin/headroom-tray` and restarts a running tray. A release without that tray
  tarball fails the update before anything is downloaded; an unknown variant name is read as
  `linux-gnu`. On Wayland GNOME Shell loads the new extension only after
  logging out and back in; Plasma reloads widgets when `plasmashell` restarts.
- For `package` and `unknown` installs it prints what to do instead (the same text as `command`) and
  exits non-zero.

An install that is already current ends with `done` and the running version. With
`--progress json` stdout carries one JSON object per line (the lines of `install.sh` that start with
`==> ` become `step` events, its other output goes to stderr); the exit code is `0` only when the last
event is `done`. When the reader closes stdout or stderr mid-update (a settings window closed), the
remaining output is dropped and the installation still runs to the end.

| event | fields | meaning |
| --- | --- | --- |
| `step` | `text` | Human-readable progress, e.g. `"Downloading Headroom 0.5.0…"`. |
| `done` | `version`, `relogin` | Success. `version` is the installed release (the running one when it was already current). `relogin` is `true` when a GNOME Shell extension or Plasma widget was updated, so the user should log out and back in. |
| `error` | `message` | Failure (missing or invalid signature, checksum mismatch, download error, a package install, declined without `--yes`, …); the process exits non-zero. |

```
{"event":"step","text":"Checking for a new release…"}
{"event":"step","text":"Downloading Headroom 0.5.0…"}
{"event":"step","text":"Verifying the signature…"}
{"event":"step","text":"Verifying the checksum…"}
{"event":"step","text":"Unpacking…"}
{"event":"step","text":"Installing /home/ada/.local/bin/headroom"}
{"event":"done","version":"0.5.0","relogin":true}
```

Install receipt: the release tarball's `install.sh` writes `$XDG_DATA_HOME/headroom/install.json`
(default `~/.local/share/headroom/install.json`) and `uninstall.sh` removes it:

```json
{"method":"script","version":"0.4.0","options":["--no-plasma"],"prefix":"/home/ada/.local"}
```

`options` are the flags the user passed (`--no-service`, `--no-gnome`, `--no-plasma`), `prefix` is
the absolute install prefix (the binary is `<prefix>/bin/headroom`). `packaging/install.sh` (a source
build) writes the same file with `"method":"source"`, which `headroom update` does not replace.

## Notifications

The daemon sends desktop notifications itself through `org.freedesktop.Notifications` with app name
`Headroom`, icon `headroom` and a default action. Clicking a notification emits `OpenRequested`.

Rules per `(account, window)`:

- The first observation only records the current state, so start-up never alerts.
- `AlmostOut`, `CuttingItClose` and `WillRunOut` fire once on the rising edge. Jumping straight to
  `running_out` sends only `WillRunOut`.
- `AlmostOut` fires when remaining drops under the threshold: `notifications.provider_thresholds[provider]`
  when the provider has a key, otherwise `notifications.threshold_percent` (default 10). A threshold of
  `0` turns `AlmostOut` off for that provider; its other milestones are unaffected.
- `AlmostOut` re-arms when remaining climbs back to the threshold + 5 % or more (15 % with the default);
  `CuttingItClose` and `WillRunOut` re-arm when severity drops to `healthy` or below.
- When `resets_at` moves forward by more than a second the window has reset: milestones re-arm, and
  `Reset` fires if the window's last tone was `warning` or `critical`.
- The state is stored in the daemon's database, so restarts do not repeat alerts. A failed delivery is
  rolled back and retried at the next refresh. Disabled milestones advance silently.
- Hidden accounts, dismissed accounts and hidden windows (`display.hidden_windows`) are not evaluated.

Subscription lapse, per account:

- When a refresh first ends in `no_subscription`, one notification is sent. It is not repeated while the
  account stays in that state, including across daemon restarts. A successful refresh ends the lapse,
  so a later lapse notifies again. A failed delivery is retried at the next check. Hidden accounts are
  not notified. There is no setting for it.

Quiet hours (`notifications.quiet_hours`, `from`/`to` in the daemon's local time zone, so they follow
daylight saving changes; a range with `from` later than `to` crosses midnight, `from` is inside the
range and `to` is not):

- While the range is active, window alerts and subscription lapses are held instead of delivered.
  With `allow_critical`, alerts with `critical` urgency (`WillRunOut` when the limit is reached) are
  delivered at once. A held alert counts as delivered for the rules above, so it is not repeated.
- Held alerts are stored in the daemon's database (table `held_alerts`) and survive restarts. They are
  keyed by the alert `id`; a newer alert with the same id replaces the older one.
- When the range ends the daemon releases them. It checks at start-up, whenever settings change, when
  the range ends and at least once a minute (so a missed wake-up after suspend is caught). Turning
  `enabled` off, or moving the range so that now is outside it, releases held alerts at once.
- On release, items that no longer apply are dropped: the window has reset (its `resets_at` moved
  forward or has passed), the condition has cleared (the milestone re-armed), the milestone was
  switched off or its threshold set to `0`, or the account's lapse ended.
- One remaining item is delivered as its original notification (same `id` and title; the body is
  recomposed so countdowns are current). Two or more are delivered as one summary with `id`
  `summary/<unix seconds>`, an empty `account_id`, the highest urgency of its items, and one line per
  item (`<provider> · <account> · <window> — <state>`, oldest first), at most 4 lines plus a
  `+N more` line. Nothing is sent when every item was dropped. A failed delivery keeps the items and
  is retried at the next check.
- On macOS the summary is an ordinary `Alert` (see [`docs/ipc.md`](ipc.md)) with the same fields.

Texts follow `display.language` (`N` is the threshold that applied):

| milestone | English | Russian |
| --- | --- | --- |
| title | `Codex · Work — Session` | `Codex · Work — Сессия` |
| `AlmostOut` | `Under N% left · resets in 42m` | `Осталось меньше N% · сброс через 42 мин` |
| `CuttingItClose` | `Projected to finish close to the limit · resets in 2h` | `По прогнозу лимита едва хватит до сброса · сброс через 2 ч` |
| `WillRunOut` | `Projected to run out in 20m · resets in 42m` (`… before the reset` without a run-out time) | `По прогнозу лимит закончится через 20 мин · сброс через 42 мин` (`… до сброса`) |
| `WillRunOut` when spent | `Limit reached · resets in 42m` | `Лимит исчерпан · сброс через 42 мин` |
| `Reset` | `Limit reset · 100% left` | `Лимит сброшен · осталось 100%` |
| lapse title | `Codex · Work — subscription inactive` | `Codex · Work — подписка неактивна` |
| lapse body | `Limits are unavailable until the plan is renewed.` | `Данные о лимитах недоступны, пока подписка не продлена.` |
| summary title | `Headroom — while you were away` | `Headroom — пока вас не было` |
| summary line, `AlmostOut` | `Codex · Work · Weekly — under N% left` | `Codex · Work · Неделя — осталось меньше N%` |
| summary line, `CuttingItClose` | `… — close to the limit` | `… — лимита едва хватит` |
| summary line, `WillRunOut` | `… — projected to run out` | `… — лимит скоро закончится` |
| summary line, `WillRunOut` when spent | `… — limit reached` | `… — лимит исчерпан` |
| summary line, `Reset` | `… — limit reset` | `… — лимит сброшен` |
| summary line, lapse | `Codex · Work — subscription inactive` | `Codex · Work — подписка неактивна` |
| summary overflow | `+2 more` | `и ещё 2` |

Summary lines leave out `· <account>` when the account has neither a label nor an email.

Session and weekly window labels are translated; other window labels come from the provider as is.
Titles start with the provider's `display_name` from the registry.

## Adding and removing accounts from a shell

Shells never run provider CLIs themselves; they run `headroom` and read its progress. Both commands work
without a terminal when `--progress json` is given and then print exactly one JSON object per line on
stdout (human hints go to stderr). The exit code is `0` only when the last event is `done`.

```
headroom accounts add <PROVIDER-ID> [--label NAME] [--api-key-stdin] --progress json
headroom accounts remove <ID> --yes --progress json
headroom accounts restore [<PROVIDER-ID>]
headroom providers [--json]
```

`add` does what the provider's first `add_account` entry in [`ListProviders`](#providers) says:
`cli_login` runs the login CLI; `api_key` needs `--api-key-stdin`; `auto_detect` fails with the
provider's reason. With `--api-key-stdin` it uses the provider's `api_key` entry, whatever its
position, and fails for providers without one. An unknown id fails with an `error` event that lists
the known ids.

API keys: `add <id> --api-key-stdin` reads the first line of stdin (surrounding whitespace is
trimmed; a key is never accepted from arguments or the environment), checks it with the provider,
stores it in the Secret Service (attributes `application=io.github.daniarjabagin.headroom`, `provider`, `account`)
or, without an unlocked keyring, in `$XDG_DATA_HOME/headroom/secrets/<account id>` (mode `0600`),
and writes the account into a new Headroom-owned home. A rejected key fails with `error` before any
`started`; nothing is stored. Adding a key for an account that is already added replaces its key.
The key never appears in events, messages or logs. `remove` also deletes the stored key.

`remove` depends on who owns the account. A Headroom-owned account (`"owner": "headroom"`) is signed
out: its home under `$XDG_DATA_HOME/headroom/accounts/` is deleted. Headroom never deletes or changes
a CLI home, so for a CLI-owned account (`"owner": "cli"`) `remove` calls `DismissAccount` instead:
Headroom stops showing the account and the provider's CLI stays signed in. This needs a running
daemon; without one `remove` fails with a message saying so. Both cases end with the same `done`
event. A dismissed account can then be added again through Headroom's own sign-in
(`accounts add`, a Headroom-owned home): it shows with `"owner": "headroom"` while its CLI home stays
dismissed. When the daemon shows one id from two homes, `remove` acts on the record the daemon
shows. `headroom accounts restore [<PROVIDER-ID>]` calls `RestoreAccounts` to show dismissed CLI
accounts again.

| event | fields | meaning |
| --- | --- | --- |
| `started` | `provider`, `home` | The login CLI starts with its config dir set to the new Headroom-owned `home`; for an API key, the key was accepted and stored for the account in `home`. `add` only. |
| `url` | `url` | The first `http(s)` URL of an output line, reported once per distinct URL. Loopback URLs (`localhost`, `127.0.0.1`, `[::1]`: the CLI's own callback server) are skipped. Also taken from OSC 8 terminal hyperlinks. `add` only. |
| `output` | `line` | Every stdout and stderr line of the login CLI, ANSI escapes removed. A prompt without a trailing newline is reported after 100 ms of silence. `add` only. |
| `done` | `account_id`, `label` | Success. For `add`, `label` is the label that was applied, or `null` when none was requested or the daemon was not running or did not list the account yet. For `remove`, `label` is always `null`. |
| `error` | `message` | Failure; the process exits with a non-zero code. A failed `add` deletes the new home. A cancelled `add` reports `"cancelled"`. |

```
{"event":"started","provider":"codex","home":"/home/ada/.local/share/headroom/accounts/codex/2f0c…"}
{"event":"output","line":"Starting local login server on http://localhost:1455."}
{"event":"output","line":"If your browser did not open, navigate to this URL to authenticate:"}
{"event":"output","line":"https://auth.openai.com/oauth/authorize?…"}
{"event":"url","url":"https://auth.openai.com/oauth/authorize?…"}
{"event":"done","account_id":"codex:1a2b3c4d5e6f","label":"Work"}
```

Lines written to the command's stdin are forwarded to the login CLI line by line, so a shell can paste
a code when the CLI asks for one. `remove` without `--yes` fails with an `error` event instead of
prompting. Without `--progress` both commands keep their interactive terminal behaviour.

Cancelling `add`: a shell stops a running `add` by sending `SIGTERM` (or `SIGINT`) to `headroom`, or,
with `--progress json`, by closing the read end of its stdout pipe. `headroom` then kills the login
CLI's whole process group with `SIGKILL` (the CLI is started as the leader of its own process group,
so helpers it spawned go too), waits for it, deletes the new home, prints
`{"event":"error","message":"cancelled"}` if stdout is still open and exits non-zero. A signal that
arrives after the login finished but before `done` also deletes the new home. Without `--progress`
the CLI shares the terminal's process group so it can read the keyboard; there `SIGTERM` stops only
the CLI process itself (Ctrl+C reaches the whole foreground group anyway). After a
successful `add` or `remove` the command asks a running daemon to `Rescan`, so `StateChanged` follows.
