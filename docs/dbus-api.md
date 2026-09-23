# Headroom D-Bus API

The daemon (`headroom daemon`, crate `headroom-daemon`) is the only process that talks to providers.
Shells (GNOME extension, Plasma plasmoid, tray, TUI, CLI) read state and send commands over the
session bus. Payloads are JSON strings so every toolkit can parse them the same way.

| item | value |
| --- | --- |
| bus | session bus |
| well-known name | `io.github.headroom.Daemon` |
| object path | `/io/github/headroom/Daemon` |
| interface | `io.github.headroom.Daemon1` |

The daemon requests the name with `DO_NOT_QUEUE` and without `ALLOW_REPLACEMENT`. If another daemon
already owns it, the new one exits with "another Headroom daemon already owns the bus name".

## Methods

| method | signature | description |
| --- | --- | --- |
| `GetState` | `() → s` | Current state payload (see [State](#state-payload)). Assembled on every call. |
| `ListProviders` | `() → s` | The providers compiled into this build and how to add their accounts (see [Providers](#providers)). Does not change while the daemon runs. |
| `Refresh` | `(s account_id) → ()` | `""`: refresh every visible or hidden active account whose last attempt is older than 60 s, that is not refreshing and not inside a rate-limit or `no_subscription` hold. An account id: force a refresh of that account now. |
| `Rescan` | `() → ()` | Run account discovery now instead of waiting for the next 10-minute pass, then refresh newly found accounts at once. Returns when the discovered accounts are stored and listed in the state; the refreshes it starts finish later. |
| `GetSettings` | `() → s` | Current settings JSON (see [Settings](#settings)). |
| `SetSettings` | `(s json) → ()` | Replace the settings document. Missing fields take their defaults, unknown fields are rejected. Validated before it is stored; emits `StateChanged`. Kept for compatibility; shells should use `UpdateSettings`. |
| `UpdateSettings` | `(s patch) → ()` | Apply a JSON Merge Patch (RFC 7386) to the current settings, validate the result like `SetSettings`, store it and emit `StateChanged`. See [Updating settings](#updating-settings). |
| `SetAccountLabel` | `(s account_id, s label) → ()` | Set a user label. Surrounding whitespace is trimmed; an empty label clears it. At most 64 characters. |
| `SetAccountOrder` | `(as ids) → ()` | Move the given accounts to the front, in that order. Accounts not listed keep their relative order after them. |
| `SetAccountHidden` | `(s account_id, b hidden) → ()` | Hide or show an account. Hidden accounts stay in the payload with `"hidden": true` but are ignored by the headline and by notifications. |

Refresh semantics:

- A forced refresh while the same account is already refreshing does not start a second request; it
  queues exactly one follow-up refresh that starts when the current one finishes. Further requests in
  the meantime are coalesced into that follow-up.
- Scheduled refreshes run every `refresh_interval_secs` ± 10 %. Failures back off 60 s, 120 s, 240 s, …
  up to 30 min (± 10 %). A provider rate limit waits `retry_after`, or 5 min when none is given.
  `no_subscription` is not transient: the account is checked again after 1 h (± 10 %).
- Rate-limited and `no_subscription` accounts are skipped by `Refresh("")` until their next scheduled
  check; `Refresh(account_id)` still forces a check.
- Each provider call has a 30 s timeout.

Rescan semantics:

- Discovery runs every 10 minutes and on `Rescan`. A rescan also re-syncs the usage homes and resets
  the 10-minute timer.
- Rescans are coalesced: requests that arrive while a discovery is running wait for one follow-up
  discovery that starts after the current one, and all of them return when it finishes.
- Accounts found by a rescan (new ones and ones that come back) refresh immediately; accounts that
  disappeared stop being refreshed and leave `accounts[]`.
- `headroom accounts add` and `headroom accounts remove` call `Rescan`, so the change shows up at once.

### Errors

| D-Bus error | when |
| --- | --- |
| `org.freedesktop.DBus.Error.InvalidArgs` | unknown account id, duplicate id in `SetAccountOrder`, label longer than 64 characters, malformed or invalid settings JSON, a settings patch that is not a JSON object or whose result is invalid |
| `org.freedesktop.DBus.Error.Failed` | storage or encoding failure inside the daemon, or `Rescan` while the daemon is shutting down |

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
| `generated_at` | RFC 3339 timestamp | When the payload was assembled. Use it as "now" for countdowns. |
| `next_refresh_at` | timestamp \| null | Earliest scheduled refresh among visible accounts. Accounts that are refreshing have no schedule until they finish; `null` when nothing is scheduled. |
| `last_success_at` | timestamp \| null | `fetched_at` of the newest live snapshot of a visible account (cached snapshots from an earlier daemon run count; data read from local logs does not). `null` when there is none. |
| `offline` | bool | `true` when every listed account (hidden ones included) failed its most recent refresh with a `network` error. Any success, any other error or an account not tried yet makes it `false`. `false` without accounts. |
| `display` | Display | A copy of `settings.display` (see [Settings](#settings)), so shells get their display options with every `StateChanged`. |
| `headline` | Headline \| null | The one window a panel should show. `null` when no visible account has a visible window. |
| `accounts` | Account[] | Known accounts in user order (`SetAccountOrder`). Accounts that disappeared from discovery are left out; their data is kept and returns if they come back. |
| `usage` | Usage[] | Local token usage, one entry per usage home the daemon reads (see [Usage](#usage)), whether or not an account belongs to it. |
| `spend` | Spend | `usage` summed across usage homes, per period and per provider. Shells show these totals as they are and never add up `usage` themselves. |

All timestamps are RFC 3339 strings in UTC (`2026-09-23T10:00:00Z`, fractional seconds when present).
Percentages are JSON numbers (floating point, unrounded). Token counts and money are integers; money
is always micro-USD (`12500000` = $12.50).

### Headline

| field | type | description |
| --- | --- | --- |
| `account_id` | string | Account the window belongs to. |
| `provider` | string | Provider id of that account (see [Providers](#providers)). |
| `provider_name` | string | Display name of that provider from the registry. |
| `account_label` | string | The account's user label, else its email, else `provider_name`. |
| `window` | string | Window id, see [Window ids](#window-ids). |
| `window_label` | string | Same as the window's `label`. |
| `used_percent` | number | Same as the window's `used_percent`. |
| `remaining_percent` | number | Same as the window's `remaining_percent`. |
| `tone` | Tone | Same as the window's `tone`. |

Selection: hidden accounts, accounts with status `no_subscription` and hidden windows
(`display.hidden_windows`) are never chosen. With
`headline.mode = "pinned"` in settings the pinned window is used when that account is listed, not
hidden and has that window, and the window is not hidden. Otherwise (`"auto"`, or the pin is not
available) the most critical visible window wins: highest `tone`, then lowest `remaining_percent`,
then account order.

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
| `kind` | string | `usd` or `count`. |
| `usd_micros` | integer | Only for `kind = "usd"`. |
| `value` | integer | Only for `kind = "count"`. |
| `unit` | string | Only for `kind = "count"`, e.g. `requests`. |

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
  "generated_at": "2026-09-23T10:00:00Z",
  "next_refresh_at": "2026-09-23T10:03:00Z",
  "last_success_at": "2026-09-23T09:58:00Z",
  "offline": false,
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
    "hidden_windows": { "codex:work": ["weekly"] }
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
    "tone": "critical"
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
| `notifications.almost_out` | bool | `true` | Notify when a window drops under 10 % remaining. |
| `notifications.cutting_it_close` | bool | `true` | Notify when pace rises to `close`. |
| `notifications.will_run_out` | bool | `true` | Notify when pace rises to `running_out` or `spent`. |
| `notifications.reset` | bool | `false` | Notify when a window that was `warning` or worse resets. |
| `headline` | object | `{"mode":"auto"}` | `{"mode":"auto"}` or `{"mode":"pinned","account_id":"codex:…","window":"session"}`. A pin needs a non-empty account id and window. |
| `reduced_motion` | bool | `false` | Shells disable animations. |
| `display.theme` | string | `"system"` | `system`, `light` or `dark`. Shells follow the system theme unless forced. |
| `display.language` | string | `"system"` | `system`, `en` or `ru`. Language of the daemon's notifications (and of shells that are translated). `system` resolves from `LC_ALL`, then `LC_MESSAGES`, then `LANG` of the daemon: a value starting with `ru` means Russian, anything else English. |
| `display.value_mode` | string | `"left"` | `left` shows remaining percent, `used` shows used percent. |
| `display.reset_format` | string | `"countdown"` | `countdown` (`resets in 2h 5m`) or `exact` (reset time of day / date). |
| `display.panel_label` | string | `"percent"` | What the panel shows next to the icon: `percent` or `window` (the headline's window label). |
| `display.show_spend` | bool | `true` | Show the spend section. |
| `display.show_account_spend` | bool | `true` | Show local spend under each account card. |
| `display.show_trend` | bool | `true` | Show the 30-day trend. |
| `display.show_forecast` | bool | `true` | Show pace forecasts (`~8% spare`, `limit in 23m`). |
| `display.translucent` | bool | `false` | Shells render the popup with a translucent (blurred where supported) background instead of an opaque one. |
| `display.hidden_windows` | object | `{}` | Map of account id → array of window ids to hide, e.g. `{"codex:1a2b3c4d5e6f":["weekly","model:spark"]}`. Ids must be non-empty; duplicates in a list are dropped (first occurrence kept). Account ids that are not currently listed are allowed and kept. |

```json
{
  "refresh_interval_secs": 300,
  "notifications": { "almost_out": true, "cutting_it_close": true, "will_run_out": true, "reset": false },
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
    "hidden_windows": {}
  }
}
```

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
- Arrays are replaced whole, e.g. one account's hidden window list.
- `headline` is a tagged union and is replaced whole when the patch gives it an object:
  `{"headline":{"mode":"auto"}}` unpins even though the current value has `account_id` and `window`.
  `{"headline":null}` also resets it to auto.
- The result is validated like `SetSettings` (unknown fields, unknown enum values, ranges). Deleting a
  key that does not exist is a no-op.

The read, merge, validation and store run inside the daemon under one lock that `SetSettings` shares,
so concurrent patches from several shells never lose each other's changes. An invalid patch or an
invalid result fails with `InvalidArgs` and leaves the settings unchanged; a successful patch emits
`StateChanged` even if nothing changed.

```
UpdateSettings('{"display":{"translucent":true,"hidden_windows":{"claude:9f8e":null}}}')
```

The former top-level `show_usage` is gone. Settings stored by an older daemon are migrated on load
(`show_usage` becomes `display.show_spend` unless that is set), but `SetSettings` with `show_usage` is
invalid.

## Notifications

The daemon sends desktop notifications itself through `org.freedesktop.Notifications` with app name
`Headroom`, icon `headroom` and a default action. Clicking a notification emits `OpenRequested`.

Rules per `(account, window)`:

- The first observation only records the current state, so start-up never alerts.
- `AlmostOut`, `CuttingItClose` and `WillRunOut` fire once on the rising edge. Jumping straight to
  `running_out` sends only `WillRunOut`.
- `AlmostOut` re-arms when remaining climbs back to 15 % or more; `CuttingItClose` and `WillRunOut`
  re-arm when severity drops to `healthy` or below.
- When `resets_at` moves forward by more than a second the window has reset: milestones re-arm, and
  `Reset` fires if the window's last tone was `warning` or `critical`.
- The state is stored in the daemon's database, so restarts do not repeat alerts. A failed delivery is
  rolled back and retried at the next refresh. Disabled milestones advance silently.
- Hidden accounts and hidden windows (`display.hidden_windows`) are not evaluated.

Subscription lapse, per account:

- When a refresh first ends in `no_subscription`, one notification is sent. It is not repeated while the
  account stays in that state, including across daemon restarts. A successful refresh ends the lapse,
  so a later lapse notifies again. A failed delivery is retried at the next check. Hidden accounts are
  not notified. There is no setting for it.

Texts follow `display.language`:

| milestone | English | Russian |
| --- | --- | --- |
| title | `Codex · Work — Session` | `Codex · Work — Сессия` |
| `AlmostOut` | `Under 10% left · resets in 42m` | `Осталось меньше 10% · сброс через 42 мин` |
| `CuttingItClose` | `Projected to finish close to the limit · resets in 2h` | `По прогнозу лимита едва хватит до сброса · сброс через 2 ч` |
| `WillRunOut` | `Projected to run out in 20m · resets in 42m` (`… before the reset` without a run-out time) | `По прогнозу лимит закончится через 20 мин · сброс через 42 мин` (`… до сброса`) |
| `WillRunOut` when spent | `Limit reached · resets in 42m` | `Лимит исчерпан · сброс через 42 мин` |
| `Reset` | `Limit reset · 100% left` | `Лимит сброшен · осталось 100%` |
| lapse title | `Codex · Work — subscription inactive` | `Codex · Work — подписка неактивна` |
| lapse body | `Limits are unavailable until the plan is renewed.` | `Данные о лимитах недоступны, пока подписка не продлена.` |

Session and weekly window labels are translated; other window labels come from the provider as is.
Titles start with the provider's `display_name` from the registry.

## Adding and removing accounts from a shell

Shells never run provider CLIs themselves; they run `headroom` and read its progress. Both commands work
without a terminal when `--progress json` is given and then print exactly one JSON object per line on
stdout (human hints go to stderr). The exit code is `0` only when the last event is `done`.

```
headroom accounts add <PROVIDER-ID> [--label NAME] [--api-key-stdin] --progress json
headroom accounts remove <ID> --yes --progress json
headroom providers [--json]
```

`add` does what the provider's first `add_account` entry in [`ListProviders`](#providers) says:
`cli_login` runs the login CLI; `api_key` needs `--api-key-stdin`; `auto_detect` fails with the
provider's reason. With `--api-key-stdin` it uses the provider's `api_key` entry, whatever its
position, and fails for providers without one. An unknown id fails with an `error` event that lists
the known ids.

API keys: `add <id> --api-key-stdin` reads the first line of stdin (surrounding whitespace is
trimmed; a key is never accepted from arguments or the environment), checks it with the provider,
stores it in the Secret Service (attributes `application=io.github.headroom`, `provider`, `account`)
or, without an unlocked keyring, in `$XDG_DATA_HOME/headroom/secrets/<account id>` (mode `0600`),
and writes the account into a new Headroom-owned home. A rejected key fails with `error` before any
`started`; nothing is stored. Adding a key for an account that is already added replaces its key.
The key never appears in events, messages or logs. `remove` also deletes the stored key.

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
