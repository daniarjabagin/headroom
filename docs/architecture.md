# Headroom architecture

This is the contract every crate and shell implements. Research behind it lives in `docs/research/`,
visual design in `docs/design/`.

## Crates

| crate | kind | owns | depends on |
| --- | --- | --- | --- |
| `headroom-core` | lib, no I/O | units, domain model, log cursors, `Provider` trait, pacing, severity, usage aggregation | `serde`, `serde_json`, `jiff`, `thiserror`, `async-trait`, `sha2`, `hex` |
| `headroom-pricing` | lib | price catalog (bundled LiteLLM + models.dev snapshots + supplement), model alias resolution, `PriceBook` impl, cost math | core |
| `headroom-providers` | lib | `jsonl` incremental reader, `http` helpers, `codex/`, `claude/` | core |
| `headroom-daemon` | lib | account registry, scheduler, SQLite storage, D-Bus service, notifications, state assembly | core, pricing, providers |
| `headroom` | bin | CLI (`clap`): `daemon`, `status`, `accounts`, `refresh`, `tui`, `waybar` | all |

Time library: `jiff` everywhere. Async runtime: `tokio`. HTTP: `reqwest` + `rustls`. D-Bus: `zbus`.
Storage: `rusqlite` with `bundled`. File watching: `notify`.

## Units (`headroom-core::units`)

```rust
pub struct Tokens(pub u64);
pub struct MicroUsd(pub i64);
pub struct Percent(f64);
```

- `Tokens` and `MicroUsd` implement `Add`, `AddAssign`, `Sum`, saturating where overflow is possible.
- `Percent::new(f64) -> Percent` rejects NaN and negatives by clamping to `0.0`; values above 100 are
  allowed (boosted plans). `Percent::remaining()` is `max(0, 100 - used)`.
- Timestamps are `jiff::Timestamp`, durations `jiff::SignedDuration`.

## Token counts

```rust
pub struct TokenCounts {
    pub input: Tokens,
    pub cache_read: Tokens,
    pub cache_write_5m: Tokens,
    pub cache_write_1h: Tokens,
    pub output: Tokens,
    pub reasoning: Tokens,
}
```

- `input` is **uncached** input only. Codex reports input including cached, the parser subtracts.
- `output` includes reasoning. `reasoning` is an informational subset and never added to totals.
- `total() = input + cache_read + cache_write_5m + cache_write_1h + output`.

## Usage events

```rust
pub struct UsageEvent {
    pub key: EventKey,
    pub at: Timestamp,
    pub model: String,
    pub tier: ServiceTier,
    pub tokens: TokenCounts,
    pub web_search_requests: u32,
}
pub enum ServiceTier { Standard, Priority, Fast }
pub struct EventKey(pub String);
```

- `EventKey` is globally unique per billed API response: Codex `response_id`, Claude
  `message.id + ":" + requestId`. Fallback key for old logs: stable hash of `(at, model, tokens)`.
- Providers emit raw events only. Cost is never stored; it is computed at query time so price
  updates re-cost history.
- The daemon stores events keyed by `(provider, usage_home, key)`. Duplicate keys keep the event with
  the larger `tokens.total()`.

## Incremental log reading

Cursor types live in `headroom-core::cursor` so the `Provider` trait can reference them; the reader
lives in `headroom-providers::jsonl`.

```rust
pub struct FileCursor { pub inode: u64, pub size: u64, pub mtime_ns: i128, pub offset: u64, pub state: serde_json::Value }
pub struct LogCursors(pub BTreeMap<PathBuf, FileCursor>);
impl LogCursors { fn cursor_mut(&mut self, path: &Path) -> &mut FileCursor; fn retain(&mut self, keep: impl FnMut(&Path) -> bool); }

pub fn read_new_lines(path: &Path, cursor: &mut FileCursor) -> Result<Vec<String>, JsonlError>;
pub fn prune_missing(cursors: &mut LogCursors);
pub fn jsonl_files(root: &Path) -> Result<Vec<PathBuf>, JsonlError>;
```

- Read from `offset`, emit only complete lines, keep a partial trailing line for next time. Blank lines
  are skipped, `\r\n` is normalized.
- Inode change, shrink (below the recorded size or offset) or mtime rewrite (same size with a new mtime,
  or an older mtime) resets the cursor to 0 and clears `state`.
- `state` is parser-specific (Codex: current model, tier, previous totals).
- Cursors are serializable; the daemon persists them. Deleted files are pruned with `prune_missing`.
- `jsonl_files` walks a directory recursively for `*.jsonl`, sorted, without following symlinks below
  the root. A missing root yields an empty list.

## Accounts

```rust
pub enum ProviderKind { Codex, Claude }
pub struct AccountId(pub String);
pub enum CredentialOwner { Cli, Headroom }
pub struct AccountRef {
    pub id: AccountId,
    pub provider: ProviderKind,
    pub home: PathBuf,
    pub owner: CredentialOwner,
}
pub struct AccountIdentity { pub email: Option<String>, pub plan: Option<String>, pub stable_key: String }
```

- `id` = `"{provider}:{sha256(stable_key)[..12]}"`. Codex `stable_key` = `user_id + "/" + account_id`
  from the id_token; Claude = `accountUuid + "/" + organizationUuid` from `.claude.json`.
- `CredentialOwner::Cli` homes (`~/.codex`, `$CODEX_HOME`, `~/.claude`, `$CLAUDE_CONFIG_DIR`, discovered
  Claude config dirs) are **read-only**: never refresh, never write. Expired access token →
  `ProviderError::SignInExpired` and the UI asks the user to open the CLI.
- `CredentialOwner::Headroom` homes live in `$XDG_DATA_HOME/headroom/accounts/{provider}/{uuid}/`
  (mode `0700`, files `0600`). `headroom accounts add codex|claude` runs the CLI login with
  `CODEX_HOME` / `CLAUDE_CONFIG_DIR` pointing there. Headroom may refresh these tokens, writing back by
  patching a `serde_json::Value` and atomically replacing the file only if its content is unchanged
  since it was read.
- Local token usage belongs to a **usage home**, not an account. Accounts sharing a home share usage.

## Quota model

```rust
pub struct QuotaWindow {
    pub id: WindowId,
    pub label: String,
    pub used: Percent,
    pub resets_at: Option<Timestamp>,
    pub period: Option<SignedDuration>,
}
pub enum WindowId { Session, Weekly, Model(String), Other(String) }

pub struct Balance { pub id: String, pub label: String, pub amount: BalanceAmount }
pub enum BalanceAmount { Usd(MicroUsd), Count { value: u64, unit: String } }

pub struct LimitsSnapshot {
    pub identity: AccountIdentity,
    pub windows: Vec<QuotaWindow>,
    pub balances: Vec<Balance>,
    pub notices: Vec<Notice>,
    pub fetched_at: Timestamp,
    pub source: LimitsSource,
}
pub enum LimitsSource { Live, LocalLog { observed_at: Timestamp } }
pub struct Notice { pub tone: Tone, pub text: String }
```

- Session vs weekly is decided by window length (5 h → Session, 7 d → Weekly), never by slot.
- Every extra window the API returns (Codex `additional_rate_limits`, Claude `seven_day_*`,
  `limits[]`) is mapped generically to `Model(..)` / `Other(..)`, not dropped.

## Pacing and severity (`headroom-core::pace`)

```rust
pub struct Pace {
    pub severity: Severity,
    pub even_pace: Option<Percent>,
    pub projected: Option<Percent>,
    pub runs_out_at: Option<Timestamp>,
}
pub enum Severity { Untracked, Healthy, Close, RunningOut, Spent }
pub enum Tone { Neutral, Good, Warning, Critical }
```

Formula (from OpenQuota, see `docs/research/providers-codex-claude.md` §5.1):

```
spent      remaining rounds to 0                                   → Spent
untracked  used == 0 | no reset | no period | reset in past        → Untracked
start = reset − period; elapsed = now − start; progress = clamp(elapsed / period, 0, 1)
elapsed < max(60 s, 1 % of period)                                 → Untracked
projected = used / progress
projected ≤ 90 → Healthy;  used < 5 → Untracked;  projected ≤ 100 → Close;  else RunningOut
runs_out_at = start + elapsed · 100 / used   (only if now < runs_out_at < reset)
```

`even_pace = progress · 100` whenever reset and period are valid (also for Untracked). `projected` and
`runs_out_at` are `None` for Untracked and Spent. Spent means `round(100 − used) ≤ 0`, i.e. less than
0.5 % remaining.

One function maps a window to `Tone`, used by every surface (panel, popup, tray, TUI, notifications):

| condition | tone |
| --- | --- |
| Spent or RunningOut | Critical |
| Close | Warning |
| Healthy | Good |
| Untracked, used ≥ 90 % | Critical |
| Untracked, used ≥ 80 % | Warning |
| Untracked, otherwise | Good |
| no data (no window) | Neutral (`Tone::default()`) |

`tone(window: &QuotaWindow, pace: &Pace) -> Tone` implements the rows with data.

## Usage summary (`headroom-core::usage`)

```rust
pub trait PriceBook: Send + Sync {
    fn cost(&self, model: &str, tier: ServiceTier, tokens: &TokenCounts, web_search: u32) -> Option<MicroUsd>;
}
pub struct UsageTotals { pub tokens: TokenCounts, pub cost: MicroUsd, pub unpriced_tokens: Tokens, pub unpriced_models: BTreeSet<String> }
pub struct UsageSummary {
    pub today: UsageTotals,
    pub yesterday: UsageTotals,
    pub last_30_days: UsageTotals,
    pub daily: Vec<(jiff::civil::Date, UsageTotals)>,
    pub models: Vec<ModelUsage>,
}
```

- `aggregate(events, &dyn PriceBook, &TimeZone, now) -> UsageSummary`. Day bucketing uses the given
  time zone. 30 days = today and the 29 previous days.
- Unpriced events still count their tokens; their cost is excluded and reported via `unpriced_*` so
  the UI can mark the total as partial. Never price with a guessed default model.

## Provider trait (`headroom-core::provider`)

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError>;
    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError>;
    fn read_usage(&self, home: &Path, cursors: &mut LogCursors) -> Result<Vec<UsageEvent>, ProviderError>;
}

pub enum ProviderError {
    NotSignedIn,
    SignInExpired,
    ApiKeyOnly,
    RateLimited { retry_after: Option<SignedDuration> },
    Network(String),
    InvalidResponse(String),
    LocalData(String),
}
```

- Error messages are safe to show and never contain tokens.
- `fetch_limits` for Codex falls back to the newest `rate_limits` snapshot in local logs when the
  network call fails or the sign-in expired, returning `LimitsSource::LocalLog`.
- `read_usage` is synchronous and CPU-bound; the daemon runs it on `spawn_blocking`.

## Daemon

- **Scheduler**: per account, refresh every 5 min with ±10 % jitter; on the popup opening (D-Bus
  `Refresh`) refresh accounts older than 60 s. Single-flight per account; a forced request during a
  refresh queues one follow-up. Failures back off exponentially 60 s → 30 min with jitter; 429 honours
  `retry_after` (default 5 min). Per-call timeout 30 s.
- **Usage**: inotify on each usage home's log directories, debounced 2 s, plus a 60 s poll fallback.
- **Storage** (`$XDG_STATE_HOME/headroom/headroom.db`, WAL): `accounts`, `limits_snapshots` (last good per
  account), `usage_events`, `log_cursors`, `notification_state`, `settings`. Migrations are numbered
  SQL files applied in order.
- **Staleness**: a snapshot older than 10 min is `stale`. A failed refresh keeps the last good snapshot
  and attaches the error.
- **Notifications** (`org.freedesktop.Notifications`): milestones `AlmostOut` (remaining < 10 %),
  `CuttingItClose` (severity rises to Close), `WillRunOut` (rises to RunningOut/Spent), `Reset` (a
  window that was Warning or worse has reset). First observation primes without alerting. State
  (fired set per window + `resets_at`) is persisted so restarts do not re-alert. Default action opens
  the popup via the shell.

## D-Bus API

- Bus name `io.github.headroom.Daemon`, object `/io/github/headroom/Daemon`, interface
  `io.github.headroom.Daemon1`.
- Methods: `GetState() -> s`, `Refresh(account_id: s)` (`""` = all), `GetSettings() -> s`,
  `SetSettings(json: s)`, `SetAccountLabel(account_id: s, label: s)`, `SetAccountOrder(ids: as)`,
  `SetAccountHidden(account_id: s, hidden: b)`.
- Signal: `StateChanged(state: s)`.
- Payloads are JSON (easy in GJS, QML and Rust alike), schema versioned by a top-level `"version"`.
  Full schema: `docs/dbus-api.md`, maintained with the daemon.

State payload outline:

```jsonc
{
  "version": 1,
  "generated_at": "2026-09-23T10:00:00Z",
  "headline": { "account_id": "codex:…", "window": "session", "remaining_percent": 38.0, "tone": "warning" },
  "accounts": [{
    "id": "codex:1a2b3c4d5e6f", "provider": "codex", "label": "Work", "email": "…", "plan": "Pro",
    "status": "fresh|stale|refreshing|error|signed_out", "error": null, "updated_at": "…", "source": "live|local_log|cache",
    "windows": [{ "id": "session", "label": "Session", "used_percent": 62.0, "remaining_percent": 38.0,
                  "resets_at": "…", "period_seconds": 18000, "tone": "warning",
                  "pace": { "severity": "close", "even_pace_percent": 55.0, "projected_percent": 97.0, "runs_out_at": null } }],
    "balances": [{ "id": "credits", "label": "Credits", "usd_micros": 12500000 }],
    "notices": [],
    "usage_home": "~/.codex"
  }],
  "usage": [{ "provider": "codex", "usage_home": "~/.codex",
              "today": { "tokens": { "input": 0, "cache_read": 0, "cache_write": 0, "output": 0, "reasoning": 0, "total": 0 }, "cost_usd_micros": 0, "partial": false },
              "yesterday": { … }, "last_30_days": { … },
              "daily": [{ "date": "2026-09-22", "total_tokens": 0, "cost_usd_micros": 0 }],
              "models": [{ "model": "gpt-5.5", "total_tokens": 0, "cost_usd_micros": 0 }] }]
}
```

Shells format numbers and countdowns for display but never recompute pace, tone or totals.
