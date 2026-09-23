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
| `Refresh` | `(s account_id) → ()` | `""`: refresh every visible or hidden active account whose last attempt is older than 60 s, that is not refreshing and not inside a rate-limit hold. An account id: force a refresh of that account now. |
| `Rescan` | `() → ()` | Run account discovery now instead of waiting for the next 10-minute pass, then refresh newly found accounts at once. Returns when the discovered accounts are stored and listed in the state; the refreshes it starts finish later. |
| `GetSettings` | `() → s` | Current settings JSON (see [Settings](#settings)). |
| `SetSettings` | `(s json) → ()` | Replace the settings document. Missing fields take their defaults. Validated before it is stored. |
| `SetAccountLabel` | `(s account_id, s label) → ()` | Set a user label. Surrounding whitespace is trimmed; an empty label clears it. At most 64 characters. |
| `SetAccountOrder` | `(as ids) → ()` | Move the given accounts to the front, in that order. Accounts not listed keep their relative order after them. |
| `SetAccountHidden` | `(s account_id, b hidden) → ()` | Hide or show an account. Hidden accounts stay in the payload with `"hidden": true` but are ignored by the headline and by notifications. |

Refresh semantics:

- A forced refresh while the same account is already refreshing does not start a second request; it
  queues exactly one follow-up refresh that starts when the current one finishes. Further requests in
  the meantime are coalesced into that follow-up.
- Scheduled refreshes run every `refresh_interval_secs` ± 10 %. Failures back off 60 s, 120 s, 240 s, …
  up to 30 min (± 10 %). A provider rate limit waits `retry_after`, or 5 min when none is given.
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
| `org.freedesktop.DBus.Error.InvalidArgs` | unknown account id, duplicate id in `SetAccountOrder`, label longer than 64 characters, malformed or invalid settings JSON |
| `org.freedesktop.DBus.Error.Failed` | storage or encoding failure inside the daemon, or `Rescan` while the daemon is shutting down |

The error message is human readable and safe to show.

## Signals

| signal | signature | description |
| --- | --- | --- |
| `StateChanged` | `(s state)` | Full state payload, same JSON as `GetState`. Emitted whenever the state changes, debounced by 250 ms so a burst of changes produces one signal. |
| `OpenRequested` | `()` | The user clicked a Headroom desktop notification (default action). Shells should open their popup. |

Shells should call `GetState` once on start-up and then follow `StateChanged`.

## State payload

Top level:

| field | type | description |
| --- | --- | --- |
| `version` | integer | Schema version, currently `1`. Incompatible changes bump it; new fields may be added without a bump, so ignore unknown fields. |
| `generated_at` | RFC 3339 timestamp | When the payload was assembled. Use it as "now" for countdowns. |
| `next_refresh_at` | timestamp \| null | Earliest scheduled refresh among visible accounts. Accounts that are refreshing have no schedule until they finish; `null` when nothing is scheduled. |
| `last_success_at` | timestamp \| null | `fetched_at` of the newest live snapshot of a visible account (cached snapshots from an earlier daemon run count; data read from local logs does not). `null` when there is none. |
| `offline` | bool | `true` when every listed account (hidden ones included) failed its most recent refresh with a `network` error. Any success, any other error or an account not tried yet makes it `false`. `false` without accounts. |
| `headline` | Headline \| null | The one window a panel should show. `null` when no visible account has a window. |
| `accounts` | Account[] | Known accounts in user order (`SetAccountOrder`). Accounts that disappeared from discovery are left out; their data is kept and returns if they come back. |
| `usage` | Usage[] | Local token usage, one entry per `(provider, usage_home)` of a listed account. |
| `spend` | Spend | `usage` summed across usage homes, per period and per provider. Shells show these totals as they are and never add up `usage` themselves. |

All timestamps are RFC 3339 strings in UTC (`2026-09-23T10:00:00Z`, fractional seconds when present).
Percentages are JSON numbers (floating point, unrounded). Token counts and money are integers; money
is always micro-USD (`12500000` = $12.50).

### Headline

| field | type | description |
| --- | --- | --- |
| `account_id` | string | Account the window belongs to. |
| `window` | string | Window id, see [Window ids](#window-ids). |
| `remaining_percent` | number | Same as the window's `remaining_percent`. |
| `tone` | Tone | Same as the window's `tone`. |

Selection: with `headline.mode = "pinned"` in settings the pinned window is used when that account is
listed, not hidden and has that window. Otherwise (`"auto"`, or the pin is not available) the most
critical window across visible accounts wins: highest `tone`, then lowest `remaining_percent`, then
account order.

### Account

| field | type | description |
| --- | --- | --- |
| `id` | string | Stable id, `"{provider}:{12 hex}"`. |
| `provider` | Provider | `"codex"` or `"claude"`. |
| `label` | string \| null | User label. Shells fall back to `email`, then to the provider name. |
| `email` | string \| null | From the last snapshot, else from storage. |
| `plan` | string \| null | Plan name as reported by the provider. |
| `hidden` | bool | Hidden by the user. |
| `status` | Status | See below. |
| `error` | Error \| null | Last refresh error. Kept while the last good data is shown. |
| `updated_at` | timestamp \| null | Time of the data: `fetched_at` for live data, the observation time for data read from local logs. `null` when no data exists yet. |
| `source` | Source \| null | Where the shown data came from. `null` when no data exists yet. |
| `windows` | Window[] | Quota windows of the last good snapshot, in provider order. |
| `balances` | Balance[] | Credits and similar balances. |
| `notices` | Notice[] | Provider notices to show under the account. |
| `usage_home` | string | Usage home of the account, `~`-relative when under the user's home. Matches `usage[].usage_home`. |

Status, evaluated in this order:

| value | meaning |
| --- | --- |
| `refreshing` | A refresh is in flight. |
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
| `kind` | string | `not_signed_in`, `sign_in_expired`, `api_key_only`, `rate_limited`, `network`, `invalid_response`, `local_data`, `timeout`, `no_provider` |
| `message` | string | Safe, human-readable message. Never contains tokens. |

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

Local token usage belongs to a usage home (a CLI config directory), not to an account; accounts that
share a home share usage.

| field | type | description |
| --- | --- | --- |
| `provider` | Provider | Tool that wrote the logs. |
| `usage_home` | string | Same format as `accounts[].usage_home`. |
| `today` | Totals | Today in the daemon's local time zone. |
| `yesterday` | Totals | Yesterday. |
| `last_30_days` | Totals | Today and the 29 previous days. |
| `daily` | Daily[] | Exactly 30 entries, oldest first, ending today; days without usage are zero. |
| `models` | ModelUsage[] | Per model over the 30 days, highest cost first, then most tokens. |

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

Daily: `date` (`YYYY-MM-DD`), `total_tokens`, `cost_usd_micros`, `partial`.

ModelUsage: `model`, `total_tokens`, `cost_usd_micros`, `partial`.

### Spend

| field | type | description |
| --- | --- | --- |
| `today` | PeriodSpend | Sum of `usage[].today`. |
| `yesterday` | PeriodSpend | Sum of `usage[].yesterday`. |
| `last_30_days` | PeriodSpend | Sum of `usage[].last_30_days`. |

PeriodSpend:

| field | type | description |
| --- | --- | --- |
| `cost_usd_micros` | integer | Sum of `by_provider[].cost_usd_micros`. |
| `total_tokens` | integer | Sum of `by_provider[].total_tokens`. |
| `partial` | bool | Any provider in the period is `partial`. |
| `by_provider` | ProviderSpend[] | One entry per provider with tokens or cost in the period (homes of one provider are added together), highest cost first, then by provider name. Empty when the period has no usage. |

ProviderSpend: `provider`, `cost_usd_micros`, `total_tokens` (`tokens.total` summed), `partial` (any of its homes is partial).

### Example

```json
{
  "version": 1,
  "generated_at": "2026-09-23T10:00:00Z",
  "next_refresh_at": "2026-09-23T10:03:00Z",
  "last_success_at": "2026-09-23T09:58:00Z",
  "offline": false,
  "headline": {
    "account_id": "claude:main",
    "window": "session",
    "remaining_percent": 8.0,
    "tone": "critical"
  },
  "accounts": [
    {
      "id": "codex:work",
      "provider": "codex",
      "label": "Work",
      "email": "ada@example.com",
      "plan": "Pro",
      "hidden": false,
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
          }
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
      "label": null,
      "email": "ada@claude.example",
      "plan": "Pro",
      "hidden": false,
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
          }
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
      "usage_home": "~/.codex",
      "today": {
        "tokens": { "input": 1000, "cache_read": 0, "cache_write": 0, "output": 200, "reasoning": 0, "total": 1200 },
        "cost_usd_micros": 2400,
        "partial": false,
        "unpriced_tokens": 0,
        "unpriced_models": []
      },
      "yesterday": { "…": "same shape as today" },
      "last_30_days": { "…": "same shape as today" },
      "daily": [
        { "date": "2026-08-25", "total_tokens": 0, "cost_usd_micros": 0, "partial": false },
        { "date": "2026-09-23", "total_tokens": 1200, "cost_usd_micros": 2400, "partial": false }
      ],
      "models": [
        { "model": "gpt-5.5", "total_tokens": 1800, "cost_usd_micros": 3600, "partial": false }
      ]
    }
  ],
  "spend": {
    "today": {
      "cost_usd_micros": 12400,
      "total_tokens": 6200,
      "partial": false,
      "by_provider": [
        { "provider": "claude", "cost_usd_micros": 10000, "total_tokens": 5000, "partial": false },
        { "provider": "codex", "cost_usd_micros": 2400, "total_tokens": 1200, "partial": false }
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

`GetSettings` returns and `SetSettings` accepts one JSON document. Unknown fields are ignored;
missing fields take their defaults, so `SetSettings` always replaces the whole document.

| field | type | default | description |
| --- | --- | --- | --- |
| `refresh_interval_secs` | integer | `300` | Scheduled refresh interval, `60`–`3600`. Applies from each account's next scheduled refresh. |
| `notifications.almost_out` | bool | `true` | Notify when a window drops under 10 % remaining. |
| `notifications.cutting_it_close` | bool | `true` | Notify when pace rises to `close`. |
| `notifications.will_run_out` | bool | `true` | Notify when pace rises to `running_out` or `spent`. |
| `notifications.reset` | bool | `false` | Notify when a window that was `warning` or worse resets. |
| `headline` | object | `{"mode":"auto"}` | `{"mode":"auto"}` or `{"mode":"pinned","account_id":"codex:…","window":"session"}`. A pin needs a non-empty account id and window. |
| `show_usage` | bool | `true` | Shells show the local usage section. |
| `reduced_motion` | bool | `false` | Shells disable animations. |

```json
{
  "refresh_interval_secs": 300,
  "notifications": { "almost_out": true, "cutting_it_close": true, "will_run_out": true, "reset": false },
  "headline": { "mode": "auto" },
  "show_usage": true,
  "reduced_motion": false
}
```

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
- Hidden accounts are not evaluated.

Example: title `Codex · Work — Session`, body `Under 10% left · resets in 42m`.
