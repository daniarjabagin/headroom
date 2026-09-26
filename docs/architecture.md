# Headroom architecture

This is the contract every crate and shell implements: the domain model, the pacing formula, the
providers, the daemon and the transports.

## Crates

| crate | kind | owns | depends on |
| --- | --- | --- | --- |
| `headroom-core` | lib, no I/O | units, domain model, log cursors, `Provider` trait, pacing, severity, usage aggregation | `serde`, `serde_json`, `jiff`, `thiserror`, `async-trait`, `sha2`, `hex` |
| `headroom-pricing` | lib | price catalog (bundled LiteLLM + models.dev snapshots + supplement), model alias resolution, `PriceBook` impl, cost math | core |
| `headroom-providers` | lib | `jsonl` incremental reader, `http` helpers (shared client, `Retry-After` parsing), provider `registry`, `secrets` store, `key_accounts` (records, key-hash identities, stored-key reader), one module per provider | core, `zbus`, `rusqlite` |
| `headroom-daemon` | lib | account registry, provider catalog, scheduler with adaptive refresh (`activity`), credential watcher (`credentials`), SQLite storage, usage ingest and spend reports (`usage`, `usage/report`), provider status pages (`status`), update checks, transports (D-Bus service on Linux, Unix socket everywhere), notifications with thresholds and quiet hours, state assembly (panel items, collapsed cards, recovery, refresh modes) | core, `zbus` (Linux only) |
| `headroom` | bin | CLI (`clap`): `daemon`, `status`, `accounts` (`add`/`login`/`remove` also stream JSON progress for shells), `providers`, `refresh`, `update` (self-update of script installs, JSON progress for shells), `waybar`, `guard`, `spend`, `diagnostics`; the daemon's logging (stderr plus a size-capped file, level reloaded from settings), the GitHub release feed, the status-page HTTP fetcher and install-method detection | all |
| `headroom-tray` | bin | SNI tray (`ksni`) and GTK 4/libadwaita popup, settings and first-run window for desktops without GNOME Shell or Plasma; `popup_model` turns the payload into view models (pure), `icon` draws the tray pixmaps, `shortcut` registers the global shortcut (GlobalShortcuts portal on Wayland, key grab on X11) | GTK 4, libadwaita, `ksni`, `zbus`, `x11rb`; talks to the daemon over D-Bus only and uses no other workspace crate |

The daemon crate never names a provider: the binary builds the providers from
`headroom_providers::registry` and hands them, with the registry's descriptors, to the daemon.

Time library: `jiff` everywhere. Async runtime: `tokio`. HTTP: `reqwest` + `rustls`. D-Bus: `zbus`.
Storage: `rusqlite` with `bundled`. File watching: `notify`.

## Units (`headroom-core::units`)

```rust
pub struct Tokens(pub u64);
pub struct MicroUsd(pub i64);
pub struct CurrencyCode(String);
pub struct Money { pub currency: CurrencyCode, pub micros: i64 }
pub struct Percent(f64);
```

- `Tokens` and `MicroUsd` implement `Add`, `AddAssign`, `Sum`, saturating where overflow is possible.
- `Money` is an amount in millionths of one unit of an ISO 4217 currency (`CurrencyCode::parse`
  accepts three ASCII letters and upper-cases them). It has no arithmetic on purpose: amounts in
  different currencies are never summed or converted. US-dollar figures use `MicroUsd`; spend totals
  are USD only.
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
    pub reported_cost: Option<MicroUsd>,
}
pub enum ServiceTier { Standard, Priority, Fast }
pub struct EventKey(pub String);
```

- `EventKey` is globally unique per billed API response: Codex `response_id`, Claude
  `message.id + ":" + requestId` (or `message.id` alone when `requestId` is missing), Grok
  `params._meta.eventId + ":" + model`. Claude
  `usage.iterations[]` entries not covered by the top-level usage (all but the last) get
  `{base}:iter:{index}`. Fallback key when no id exists: stable hash of `(at, model, tokens)`.
- Providers emit raw events only. Cost is computed at query time so price updates re-cost history,
  except `reported_cost`: the exact cost a tool logged itself (Grok `costUsdTicks`). It is stored
  with the event (nullable `usage_events.reported_cost`, migration 003; older rows are NULL) and
  `aggregate` uses it instead of the price book. Codex and Claude leave it `None`.
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
pub struct ProviderId(Cow<'static, str>);   // "codex", "claude", "opencode", …
pub struct AccountId(pub String);
pub enum CredentialOwner { Cli, Headroom }
pub struct AccountRef {
    pub id: AccountId,
    pub provider: ProviderId,
    pub home: PathBuf,
    pub owner: CredentialOwner,
}
pub struct AccountIdentity { pub email: Option<String>, pub plan: Option<String>, pub stable_key: String }
```

- `ProviderId` is an open string id, lowercase `[a-z0-9_-]+`. `ProviderId::from_static` wraps a
  literal for a registry entry (registry tests check every literal), `ProviderId::parse` checks
  runtime text; serde reads and writes it as a plain string and rejects invalid ids. Storage keeps
  the same string, so ids stored by older builds stay valid.
- `id` = `"{provider}:{sha256(stable_key)[..12]}"`. Codex `stable_key` = `user_id + "/" + account_id`
  from the id_token; Claude = `accountUuid + "/" + organizationUuid` from `.claude.json`.
- `CredentialOwner::Cli` homes (`~/.codex`, `$CODEX_HOME`, `~/.claude`, `$CLAUDE_CONFIG_DIR`, discovered
  Claude config dirs) are **read-only**: never refresh, never write. Expired access token →
  `ProviderError::SignInExpired` and the UI asks the user to open the CLI.
- `CredentialOwner::Headroom` homes live in `$XDG_DATA_HOME/headroom/accounts/{provider}/{uuid}/`
  (mode `0700`, files `0600`). `headroom accounts add <provider>` runs the provider's CLI login with
  its home variable (`CODEX_HOME`, `CLAUDE_CONFIG_DIR`, …) pointing there, or stores a pasted API
  key (see [API-key accounts](#api-key-accounts)). Headroom may refresh these tokens, writing back by
  patching a `serde_json::Value` and atomically replacing the file only if its content is unchanged
  since it was read.
- Removing an account (`headroom accounts remove <id>`) depends on the owner. A Headroom-owned account
  is signed out by deleting its home (and its stored API key). A CLI-owned home is never deleted or
  changed, so removing a CLI-owned account *dismisses* it: D-Bus `DismissAccount` records that CLI
  record (provider, account id, home) in the daemon's `dismissed_homes` table, never in the settings.
  `Provider::discover` lists every signed-in home, so one id may appear at a CLI home and at a
  Headroom-owned home; the daemon drops dismissed CLI records first and only then keeps the first
  record per id (`DismissedHomes::resolve`, then `first_per_id`). Dismissed records leave
  `accounts[]`, the headline, notifications and refreshes, and a refresh that finishes after the
  dismissal is discarded. The same person can then be added again through `headroom accounts add`:
  the Headroom-owned home (whose tokens Headroom refreshes) wins and shows with `owner: "headroom"`.
  `headroom accounts restore [<provider>]` (D-Bus `RestoreAccounts`) clears dismissals, and the CLI
  home wins again where the provider lists it first. Usage homes are independent of accounts, so a
  dismissed account's local usage and spend still count.
- Local token usage belongs to a **usage home** (a directory with the tool's logs), not an account.
  Usage homes are discovered on their own (`Provider::usage_homes`): a home with logs but no OAuth
  account (API-key users, signed-out users) still counts, and so does a second config dir signed into
  an account already discovered elsewhere. An account links to the usage home at its own `home` when
  that home has logs, or, for a Headroom-owned account, to a CLI home signed in to the same account id
  that is not shown as an account of its own (for example after it was dismissed); the account's
  `usage[]` entry then merges all of its homes with logs, while spend still counts each home once.
  Accounts sharing a home share usage.

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
pub enum BalanceAmount { Usd(MicroUsd), Money(Money), Count { value: u64, unit: String } }

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
    pub basis: Option<Basis>,
    pub active_left: Option<SignedDuration>,
}
pub enum Basis { Recent, Window, Paused }
pub enum Severity { Untracked, Healthy, Close, RunningOut, Spent }
pub enum Tone { Neutral, Good, Warning, Critical }
```

Formula (based on OpenQuota's pace model, softened so an early burst in a long window does not look
alarming):

```
spent      remaining rounds to 0                                   → Spent
untracked  used == 0 | no reset | no period | reset in past        → Untracked
start = reset − period; elapsed = now − start; progress = clamp(elapsed / period, 0, 1)
elapsed < max(60 s, min(15 % of period, 24 h))                     → Untracked (too young)
projected = used / progress
projected ≤ 90 → Healthy;  used < 5 → Untracked;  projected ≤ 100 → Close;  else RunningOut
runs_out_at = start + elapsed · 100 / used   (only if now < runs_out_at < reset)
```

The young-window threshold is 45 min for a 5 h session and 24 h for a weekly window. Examples: weekly,
13 % used after 18 h → Untracked (tone Good by level); the same after 30 h → projected ≈ 73 % →
Healthy.

`even_pace = progress · 100` whenever reset and period are valid (also for Untracked). `projected` and
`runs_out_at` are `None` for Untracked and Spent. Spent means `round(100 − used) ≤ 0`, i.e. less than
0.5 % remaining.

One function maps a window to `Tone`, used by every surface (panel, popup, tray, CLI, notifications):

| condition | tone |
| --- | --- |
| Spent | Critical |
| RunningOut, `runs_out_at − now ≤ max(15 % of period, 1 h)` | Critical |
| RunningOut, used ≥ 90 % | Critical |
| RunningOut, otherwise (run-out far away or unknown) | Warning |
| Close | Warning |
| Healthy | Good |
| Untracked, used ≥ 90 % | Critical |
| Untracked, used ≥ 80 % | Warning |
| Untracked, otherwise | Good |
| no data (no window) | Neutral (`Tone::default()`) |

`tone(window: &QuotaWindow, pace: &Pace, now: Timestamp) -> Tone` implements the rows with data.
Example: 5 h session, 77 % used after 2.5 h → runs out in ~44 min, within the 1 h horizon → Critical.

### Forecast (`headroom-core::forecast`)

`pace` above is the window average (`basis = window`). The payload and notifications use
`forecast(window, activity, now)`, which may replace it:

- **History.** The daemon records one `quota_samples` row per change of `used_percent` per account
  window (`history::sample_step`), at the snapshot's data time; a window reset or a drop restarts
  the history, rows older than 8 days are pruned.
- **Activity.** `Activity { samples, signal, observed_at }`. `Signal.liveness` is `Unknown` when
  none of the account's usage homes (its own or linked CLI homes) has local logs, else `Live` or
  `Idle` from the log activity tracker; `Signal.poll_interval` is the effective refresh interval;
  `observed_at` is the snapshot's data time.
- **Paused.** Only with `Idle`: the percent is unchanged since the last sample and
  `observed_at − last change > idle_after`, where `idle_after = max(clamp(period / 30, 10 min,
  2 h), 2 × poll_interval)`. Idleness is measured between observations, not against the wall
  clock, so a failing refresh or a long interval never drifts into `paused`. Severity, tone and
  projection stay the window average, `runs_out_at` is dropped and `active_left` is the remaining
  percent at the last active rate.
- **Recent.** Only for windows of 24 h or less: the rate over the latest active stretch (at least
  three rises, gaps longer than `idle_after` end the stretch, lookback `clamp(period / 6, 30 min,
  4 h)` of active time, an overdue step slows it) projects `used + rate × time to reset`.
- **Live spend** (`forecast_with_spend`, `headroom-core::calibration`). Only for a `Live` account
  and windows of 24 h or less, before the percent-step rate. The daemon keeps one `SpendPoint` per
  local usage event of the last 26 h per usage home (`usage::weighted`, built from the events the
  usage summary already loaded after each ingest, which also re-emits the state; homes no longer
  discovered are dropped) with its cost, or no cost when the event is unpriced, and merges the
  account's own and linked homes (a single home's list is shared as is; nothing is gathered for an
  account that is not `Live` or has no window of 24 h or less). A linked home counts whole: spend a
  CLI home logged before it switched to this account is merged too, and when the polls do not show
  it the contradiction rule below turns the forecast back to percent steps. Calibration starts with
  the open interval from the last change to the last observation as a pair with zero rise (spend the
  provider has not shown dilutes the ratio), then walks the window's sample pairs from the newest
  back until 20 percent of rise, summing the rise and the micro-USD spent in each (money stays an
  integer; the ratio only scales a percent); it needs at least 3 percent of rise and $0.10 of spend,
  and a window reset clears it because only the current window's samples count. The spend forecast
  is skipped, falling back to the percent-step rate, when an interval it uses, the lookback or the
  time since the last change holds an unpriced event, or when `ratio × spend between the last change
  and the last observation` exceeds one reported step (the provider saw that spend and showed no
  step, so the ratio is contradicted — API-key sessions writing to the same CLI home, a home shared
  by two accounts). Otherwise the estimate is `used + ratio × spend since the last change`, capped at
  `used + 1 + ratio × spend since the last observation` and clamped to `[used, 100]`; the rate is
  `ratio × spend over the cadence lookback (never before the window start) / lookback`. They drive
  `projected`, `runs_out_at` and severity with `basis = recent`; `used_percent`, the meters,
  tone-by-level and the 5 % rule keep the provider's number. Pace alerts are decided on the
  percent-only forecast, so spend alone never raises one; a new "will run out" or "cutting it close"
  is held back (left armed) while the spend-aware forecast of the same window is below Close, so an
  alert never contradicts a calm popup. A red popup without an alert is expected.
- **Window average** otherwise, including every window longer than 24 h unless it is paused.

Young, untracked and spent windows keep the window-average result. Pace alerts fired on a `recent`
forecast are re-armed only by a window reset or by a `recent` observation with both the recent and
the window-average severity below Close.

## Usage summary (`headroom-core::usage`)

```rust
pub trait PriceBook: Send + Sync {
    fn cost(&self, event: &UsageEvent) -> Option<MicroUsd>;
}
pub struct UsageTotals { pub tokens: TokenCounts, pub cost: MicroUsd, pub unpriced_tokens: Tokens, pub unpriced_models: BTreeSet<String> }
pub struct PeriodUsage { pub totals: UsageTotals, pub models: Vec<ModelUsage> }
pub struct UsageSummary {
    pub today: PeriodUsage,
    pub yesterday: PeriodUsage,
    pub last_30_days: PeriodUsage,
    pub daily: Vec<(jiff::civil::Date, UsageTotals)>,
}
```

- `aggregate(events, &dyn PriceBook, &TimeZone, now) -> UsageSummary`. Day bucketing uses the given
  time zone. 30 days = today and the 29 previous days.
- Every period carries its per-model breakdown, sorted by cost desc, then tokens desc, then name.
- An event's cost is `reported_cost` when present (priced, never partial), otherwise
  `PriceBook::cost`.
- Unpriced events still count their tokens; their cost is excluded and reported via `unpriced_*` so
  the UI can mark the total as partial. Never price with a guessed default model.
- The price book sees the whole event so prices can depend on its time. The supplement's
  `dated_aliases` map a model name to the model it was billed as on the event's UTC day (newest
  `from` date not after it; an entry without `from` covers everything older). `codex-auto-review`
  uses this; the displayed model name stays the logged one.

## Provider trait (`headroom-core::provider`)

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn descriptor(&self) -> &'static ProviderDescriptor;
    fn id(&self) -> &'static ProviderId { &self.descriptor().id }
    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError>;
    async fn account_at(&self, home: &Path) -> Result<Option<AccountRef>, ProviderError>; // default: discover, match home
    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError>;
    async fn fetch_limits(&self, account: &AccountRef) -> Result<LimitsSnapshot, ProviderError>;
    fn read_usage(&self, home: &Path, cursors: &mut LogCursors) -> Result<Vec<UsageEvent>, ProviderError>;
    async fn validate_key(&self, key: &str) -> Result<AccountIdentity, ProviderError>; // default: Unsupported
}

pub enum ProviderError {
    NotSignedIn,
    SignInExpired,
    ApiKeyOnly,
    NoSubscription { detail: String },
    RateLimited { retry_after: Option<SignedDuration> },
    Network(String),
    InvalidResponse(String),
    LocalData(String),
    Unsupported(String),
}
```

- Error messages are safe to show and never contain tokens or keys.
- `NoSubscription { detail }` is generic: each provider writes its own `detail` ("No active ChatGPT
  subscription (Free plan).", "No active plan."), which the payload shows as `error.message`.
- `account_at` identifies the account signed in at one Headroom-owned home even when discovery
  lists the same account from another home first (Codex and Claude override it); `accounts add`
  uses it after a login.
- `validate_key` checks a pasted API key with the provider and returns whose key it is. Providers
  that read stored keys get a `SecretReader` injected at construction (`headroom-core::secret`), so
  they stay testable without D-Bus.
- `fetch_limits` for Codex falls back to the newest `rate_limits` snapshot in local logs when the
  network call fails or the sign-in expired, returning `LimitsSource::LocalLog`.
- `read_usage` is synchronous and CPU-bound; the daemon runs it on `spawn_blocking`.
- `usage_homes` returns every directory whose logs should be read, independent of accounts and
  deduplicated by canonical path. The daemon reads usage from exactly this set.
  - Codex: the CLI home (`$CODEX_HOME` or `~/.codex`) when it has `sessions/` or `archived_sessions/`,
    plus Headroom-owned homes with one of those directories.
  - Grok: the CLI home (`$GROK_HOME` or `~/.grok`) and Headroom-owned homes that have `sessions/`.
  - Claude: dirs with a `projects/` directory among `$CLAUDE_CONFIG_DIR`, `~/.claude`, the scanned
    config dirs (hidden children of `~`, children of `$XDG_CONFIG_HOME`) that hold `.claude.json` or
    `.credentials.json` (no identity needed), and Headroom-owned dirs.

## Grok

- Accounts: Headroom-owned homes (`grok login` with `GROK_HOME`, listed first so an account signed in
  both places uses the Headroom home) and the CLI home read-only. Identity `user_id + "/" + team_id`
  from the `auth.json` entry (keyed `issuer::client_id`).
- Limits: `GET {GROK_CLI_CHAT_PROXY_BASE_URL or https://cli-chat-proxy.grok.com/v1}/billing?format=credits`
  and `/settings` with `Authorization: Bearer`, `X-XAI-Token-Auth: xai-grok-cli`. A
  `USAGE_PERIOD_TYPE_WEEKLY` period becomes the Weekly window (`creditUsagePercent`, absent = 0);
  otherwise a Neutral "Legacy Grok billing" notice, or `NoSubscription` when settings report no
  `subscription_tier_display`. `onDemandCap.val` (US cents) `> 0` gives a Neutral "Extra usage on, cap $X.XX" notice,
  otherwise no notice. A failed settings call only drops the plan name.
- Tokens: an expired CLI token is `SignInExpired` and is never refreshed. A Headroom-owned token that
  expires within 5 min, or is rejected with 401/403, is refreshed once: exclusive `flock` on
  `auth.json.lock`, re-read (a token refreshed meanwhile is reused), `POST {issuer}/oauth2/token`
  (`grant_type=refresh_token`, `client_id`, `refresh_token`, form-encoded) only for the configured
  issuer `https://auth.x.ai`, patch this entry's `key`/`refresh_token`/`id_token`/`expires_at`, and
  replace the file atomically if its bytes are unchanged. 400/401/403 from the token endpoint →
  `SignInExpired`.
- Usage: `sessions/**/updates.jsonl`, lines with `params.update.sessionUpdate == "turn_completed"`;
  one event per `usage.modelUsage` entry. `input = inputTokens − cachedReadTokens −
  cacheCreationTokens` (cache writes go to `cache_write_5m`), `output = outputTokens`,
  `reasoning = reasoningTokens`. Time from `params._meta.agentTimestampMs`, else `timestamp`.
  `reported_cost` from the model's `costUsdTicks` (the turn's top-level value only when there is one
  model): 1 tick = 1e-10 USD, so micro-USD = ticks / 10 000 in integer math, rounding half up
  (remainder ≥ 5 000 adds one); negative or fractional ticks give `None` and the price book is used.

## Kilo Code, Warp and Poe

- **Kilo Code** (`kilo`): `GET https://api.kilo.ai/api/profile/balance` with `Authorization: Bearer`,
  plus `x-kilocode-organizationid` when the sign-in chose an organization (`accountId` in the
  `kilo` entry of `auth.json`). `{ balance, isDepleted }`: `balance` is USD that the server derives
  from micro-USD, so the JSON number's own text is parsed into `MicroUsd` exactly (plain decimal,
  at most six decimals, never through `f64`; anything else is an invalid response). One USD balance
  "Credit balance" (or "Organization credits"); `isDepleted` adds a Critical notice.
  `GET /api/profile` supplies the email; if it fails only the email is missing. Accounts: pasted
  keys (`key_accounts`), the CLI's `$XDG_DATA_HOME/kilo/auth.json` (`~/.local/share/kilo` on Linux
  and macOS, read-only) and Headroom-owned homes where `kilo auth login --provider kilo` ran with
  `XDG_DATA_HOME` pointing at the home (the CLI uses `xdg-basedir`, so it writes
  `<home>/kilo/auth.json`). The login uses clack prompts, so it runs on a PTY, and it scrubs
  `KILO_API_URL` and `KILO_AUTH_CONTENT`; Headroom always calls `api.kilo.ai`. Identity is the
  SHA-256 of the token (`key-sha256:…`, plus `/org:<id>` for an organization login); tokens live a
  year and are never refreshed. Local `kilo.db` usage is not read yet (Headroom has no OpenCode
  SQLite reader to reuse).
- **Warp** (`warp`): `POST https://app.warp.dev/graphql/v2?op=GetRequestLimitInfo` with the API key
  as Bearer, `User-Agent: Warp/1.0` (the edge limiter answers 429 without it), `x-warp-client-id:
  warp-app` and `x-warp-os-*` headers matching the `osContext` variables (`Linux` or `macOS`,
  version `unknown`). `requestLimitInfo` becomes the window `Other("monthly")` "Monthly credits",
  used = `requestsUsedSinceLastRefresh / requestLimit`, reset `nextRefreshTime` (RFC 3339, or a
  time without offset read as UTC), no period; `isUnlimited` gives a Neutral "Unlimited credits"
  notice and a zero limit "No monthly credits on this plan". `requestCreditsRemaining` of the
  user's and every workspace's bonus grants is summed into one Count balance "Bonus credits"
  (unit `credits`) when any grant exists. `UserFacingError` and GraphQL `errors` become invalid
  responses carrying Warp's message (200 characters at most).
- **Poe** (`poe`): `GET https://api.poe.com/usage/current_balance` with Bearer →
  `current_point_balance` as a Count balance "Point balance" (unit `points`). The points history
  is not read: points are not USD, so there is no spend to add.
- All three: 401/403 → `SignInExpired` (a pasted key is rejected with a message naming the console
  URL), 429 → `RateLimited` with `Retry-After`, 5xx → `Network`.

## Pay-as-you-go balances (DeepSeek, Moonshot)

Both are API-key providers with no windows: they show the account balance as `BalanceAmount::Money`
in the currency the account is billed in. Requests go through `headroom-providers::bearer::get_json`
(`Authorization: Bearer`, `Accept: application/json`, 15 s timeout; 401/403 → `SignInExpired`, 429 →
`RateLimited` with `Retry-After`, 5xx → `Network`, anything else → `InvalidResponse`). Amounts are
read by `decimal::ExactMicros` from the exact JSON text (a decimal string or a JSON number, through
`serde_json::value::RawValue`, never `f64`) into millionths, rounding the seventh decimal half away
from zero; exponents, `+`, `NaN` and values beyond `i64` are errors. Identity is
`key_accounts::sha256_stable_key`. Fixtures are synthetic, built from the documented shapes.

- **DeepSeek** (`deepseek`): `GET https://api.deepseek.com/user/balance`
  ([doc](https://api-docs.deepseek.com/api/get-user-balance)). Each `balance_infos[]` entry becomes
  one balance `balance_<currency>` labelled "Balance" from `total_balance` (a decimal string; the
  currency is `CNY` or `USD`); a currency listed twice or an invalid code is `InvalidResponse`.
  `is_available == false` adds a notice: Critical "Balance is used up…" when every balance is ≤ 0,
  else Warning "Balance is not enough for API calls".
- **Moonshot API** (`moonshot`, the Kimi Open Platform, separate from Kimi Code): `GET
  {host}/v1/users/me/balance` ([international](https://platform.kimi.ai/docs/api/balance) in USD on
  `api.moonshot.ai`, [mainland](https://platform.kimi.com/docs/api/balance) in CNY on
  `api.moonshot.cn`). Keys of the two platforms are independent and the registry has no
  per-provider options, so the region is found by asking: international first, and on 401/403 the
  mainland host (other failures never switch hosts). The region that answered is remembered per
  account in memory and asked first on the next refresh. The answer must be `code == 0`,
  `status == true` with `data`; `available_balance`, `voucher_balance` and `cash_balance` (JSON
  numbers) become the balances `available` "Balance", `voucher` "Vouchers" and `cash` "Cash".
  `available_balance ≤ 0` adds a Critical "Balance is used up…" notice and `cash_balance < 0` a
  Critical "…in debt" notice.

## Provider registry

Descriptors live in `headroom-core::descriptor` (types only), the registry in
`headroom-providers::registry`.

```rust
pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub display_name: &'static str,
    pub add_account: &'static [AddAccountMethod],   // the first one is the default
    pub multi_account: bool,
    pub local_usage: bool,
    pub links: ProviderLinks,
}
pub struct ProviderLinks {                          // absolute https URLs, validated by the registry
    pub status: Option<&'static str>,
    pub dashboard: Option<&'static str>,
    pub usage: Option<&'static str>,
}
pub enum AddAccountMethod {
    CliLogin(CliLogin),
    ApiKey(ApiKeyPrompt),
    AutoDetect { reason: &'static str },
}
pub struct CliLogin {
    pub program: &'static str,
    pub args: &'static [&'static str],
    pub home_var: HomeVar,
    pub credentials_file: &'static str,   // relative to the directory home_var names
    pub needs_pty: bool,
    pub scrub_env: &'static [&'static str], // removed from the inherited environment
}
pub enum HomeVar { Direct(&'static str), XdgBase { var: &'static str, subdir: &'static str } }
pub struct ApiKeyPrompt { pub label: &'static str, pub console_url: &'static str, pub hint: &'static str }

// headroom-providers::registry
pub struct RegistryContext { pub http: reqwest::Client, pub secrets: Arc<dyn SecretReader> }
pub fn descriptors() -> impl Iterator<Item = &'static ProviderDescriptor>;
pub fn descriptor(id: &str) -> Option<&'static ProviderDescriptor>;
pub fn build_all(context: &RegistryContext) -> Vec<Arc<dyn Provider>>;
pub fn build(context: &RegistryContext, id: &str) -> Option<Result<Arc<dyn Provider>, ProviderError>>;
```

- One static entry per provider: its descriptor and a builder. Each builder takes its own settings
  from the process environment and shares the one HTTP client. A provider that cannot start is
  logged and left out; the others still run.
- `ProviderDescriptor::validate` checks an entry: valid id, a display name, at least one method,
  bare login program, upper-case home variable, credential file and XDG subdir relative and inside
  the home, `https` console URL. The registry test runs it for every entry and checks ids are unique.
- `HomeVar::Direct(VAR)` points `VAR` at the new home (`CODEX_HOME`). `HomeVar::XdgBase { var,
  subdir }` points an XDG base variable at the home, so the tool writes to `home/subdir`
  (`XDG_DATA_HOME` + `opencode`); the login waits for `credentials_file` there.
- `scrub_env` names inherited variables removed before the login starts, because they would send
  the credentials somewhere other than the new home or sign in to a different account: Cline
  `CLINE_DATA_DIR`, `CLINE_PROVIDER_SETTINGS_PATH`; Copilot `GH_TOKEN`, `GITHUB_TOKEN`,
  `GH_ENTERPRISE_TOKEN`, `GITHUB_ENTERPRISE_TOKEN`, `GH_HOST`; Grok `GROK_OIDC_ISSUER`,
  `GROK_OIDC_CLIENT_ID` (Headroom refreshes only `auth.x.ai` tokens). Codex keeps its environment:
  `OPENAI_API_KEY` does not change where `codex login` writes. Every launch path (attached terminal,
  pipes, pseudo-terminal) builds the command in `Launcher::command`, which applies the list.
- A `needs_pty` login (Cline) runs, when streamed to a shell, on a fresh pseudo-terminal: slave
  as stdin/stdout/stderr, opened `O_NOCTTY` and spawned in its own process group, so the child has a
  terminal (`isatty` is true, 120×40) but no controlling terminal and no job control. Echo is off so
  typed input is not reflected back; output is read from the master and streamed line by line
  (`crates/headroom/src/accounts/pty.rs`, `stream.rs`). Logins run in the user's own terminal attach
  to it directly either way.
- `headroom accounts add <id>` picks: `--api-key-stdin` → the provider's API-key method; else, when
  the default method is an API key and stdin is a terminal → a hidden prompt (echo off,
  `accounts/prompt.rs`); else the provider's first `CliLogin`; else an error naming what to do.

Registered providers, in registry order:

| id | name | add account | local usage |
| --- | --- | --- | --- |
| `codex` | Codex | `codex login` | yes |
| `claude` | Claude | `claude` login | yes |
| `opencode` | OpenCode | API key, or detected from `opencode` auth | no |
| `openrouter` | OpenRouter | API key | no |
| `zai` | Z.ai | API key | no |
| `kimi` | Kimi Code | API key, or `kimi` login | no |
| `minimax` | MiniMax | Token Plan key | no |
| `grok` | Grok | `grok` login, or detected | yes |
| `cline` | Cline | `cline` login (PTY), or detected | no |
| `devin` | Devin | `devin` login, or detected | no |
| `copilot` | Copilot | `gh` login, or detected | no |
| `cursor` | Cursor | detected from the IDE login (one account) | no |
| `antigravity` | Antigravity | detected (one account) | no |
| `ollama` | Ollama Cloud | detected from `~/.ollama/id_ed25519` (one account) | no |
| `kilo` | Kilo Code | `kilo auth login` (PTY), API key, or detected | no |
| `warp` | Warp | API key | no |
| `poe` | Poe | API key | no |
| `deepseek` | DeepSeek | API key | no |
| `moonshot` | Moonshot API | API key (international or mainland, detected) | no |

Ollama discovery signs one `POST /api/me` per discovery pass (every 10 min): 401/403 means the key
is not linked to an ollama.com account (local-only user) and no account is listed; any other failure
keeps the account so the refresh shows the error.
- The daemon's `ProviderCatalog` holds the compiled-in descriptors. It supplies `provider_name` for
  the state payload and notifications (the id when a stored account belongs to a provider this build
  lacks), orders `usage[]` by registry position, and answers D-Bus `ListProviders`.

## Secret store

`headroom-providers::secrets::SecretStore` keeps API keys for the CLI (which writes them) and the
daemon (which reads them through `SecretReader`).

- Secret Service over the existing `zbus` dependency (no extra crates): `OpenSession("plain")`,
  `ReadAlias("default")`, `Unlock`, `CreateItem` (replace), `SearchItems`, `Item.GetSecret`,
  `Item.Delete`, `Session.Close`. Attributes `{application: "io.github.daniarjabagin.headroom", provider, account}`,
  label `Headroom API key for <account id>`.
- Fallback file `$XDG_DATA_HOME/headroom/secrets/<account id>` (directory `0700`, file `0600`,
  written atomically) when no Secret Service answers, there is no default collection, or the
  collection is locked and unlocking would need a prompt. A key stored in the Secret Service removes
  any fallback file for that account.
- Reads try the Secret Service first, then the file. A locked item without a file copy is an error
  ("the keyring is locked"). Every call has a 10 s timeout; the bus connection is opened lazily and
  reused, so the daemon holds none until a provider reads a key.
- Keys are `SecretString` (redacted `Debug`, no `Display`) and never reach logs, errors, argv, the
  environment or D-Bus payloads.
- `secrets::read_foreign(bus, attributes)` reads another application's item (Antigravity's sign-in)
  read-only: `SearchItems`, then `GetSecret` on an unlocked match. It never calls `Unlock` and never
  prompts; a locked-only match is `Locked`, anything else unavailable is `Absent`.

### macOS credentials

- **Headroom's data dir** comes from one helper, `headroom-providers::paths::HeadroomDirs`:
  `$XDG_DATA_HOME/headroom` (else `~/.local/share/headroom`) on Linux,
  `~/Library/Application Support/Headroom` on macOS. Every provider takes its Headroom-owned homes
  from `HeadroomDirs::accounts(<provider>)` and the secrets fallback from `HeadroomDirs::secrets()`.
  Desktop apps' settings (Cursor and Devin `state.vscdb`) come from `paths::app_config_dir`:
  `$XDG_CONFIG_HOME` on Linux, `~/Library/Application Support` on macOS. CLI dot-dirs (`~/.codex`,
  `~/.claude`, `~/.grok`, `~/.kimi`, `~/.cline`, `~/.config/gh`, XDG paths of OpenCode and the Devin
  CLI) are the same on both systems.
- **Keychain access** goes only through `headroom-providers::keychain::Security`, which runs
  `/usr/bin/security` (5 s timeout, `tokio::process`, killed on drop, stderr discarded). Reads are
  `find-generic-password -s <service> [-a <account>] -w`; exit 44 is "not found", exits 36/51/128
  (interaction not allowed, auth failed, cancelled) are `Denied`, any other exit is `Failed`, and a
  missing program or a timeout is unavailable. Writes run `security -i` and send
  `add-generic-password -U -s "<service>" -l "<label>" -a "<account>" -X "<hex secret>"` on
  **stdin**; the secret never appears in argv, the line is limited to 4032 bytes, quoted values may not
  contain `"`, `\` or control characters, and every write is read back and compared. Items created by
  `/usr/bin/security` trust that tool, so reading them does not prompt. The program path and timeout are
  injectable; tests run a fake `security` script on Linux and assert its argv and stdin.
- **Secret store on macOS**: `SecretStore::new` picks the Keychain (service `io.github.daniarjabagin.headroom`,
  account = account id, label `Headroom <provider> API key`) instead of the Secret Service, which with
  its `zbus` dependency is compiled on Linux only. The file fallback and its rules are unchanged; a
  denied Keychain behaves like a locked keyring. `read_foreign` returns `Absent` off Linux.
- **Claude**: on macOS (`ClaudeConfig::keychain` is set) credentials are read from the Keychain first,
  then from `.credentials.json`. Service names: `Claude Code-credentials` for `~/.claude` reached
  without `CLAUDE_CONFIG_DIR`; `Claude Code-credentials-<first 8 hex of sha256(NFC(dir))>` for
  `$CLAUDE_CONFIG_DIR` (then the plain name, as Claude Code does), for scanned config dirs and for
  Headroom-owned homes (scoped name only, so a Headroom home never picks up the CLI's sign-in).
  Accounts tried: `$USER`, then a legacy item without an account. Item text is JSON, or hex-encoded
  JSON. Not found → the file; any other failure is a `LocalData` account error. Discovery on macOS
  accepts a config dir with `.claude.json` even without `.credentials.json` and never runs `security`.
  Claude has no token refresh on any OS, so nothing writes Claude Keychain items.
- **Codex**: with `cli_auth_credentials_store = "keyring"` in `$CODEX_HOME/config.toml` (or `"auto"`
  and no `auth.json`) the sign-in is read from the Keychain item `Codex Auth`, account
  `cli|<first 16 hex of sha256(canonical CODEX_HOME)>` (Codex `compute_store_key`; test vector
  `~/.codex` → `cli|940db7b1d0e4eb40`). Read-only; a missing item falls back to `auth.json`.
- **Copilot** needs no Keychain code: tokens come from `gh auth token`, and `gh` reads its own
  `gh:github.com` item. **Antigravity** discovery reads `/proc` and is Linux-only; on macOS the
  provider reports `NotSignedIn`.

## API-key accounts

`headroom accounts add <provider> --api-key-stdin` reads one line from stdin, calls
`Provider::validate_key`, creates a Headroom-owned home, stores the key under the new account id and
writes `account.json` (the `AccountIdentity`, `0600`) into the home
(`headroom-providers::key_accounts`). Providers list these homes with `key_accounts::discover` and
read the key with `key_accounts::stored_key` over their injected `SecretReader`. A key's identity
is a hash, never the key: `key_accounts::sha256_stable_key` (`key-sha256:<hex>`, OpenCode,
OpenRouter, Z.ai, DeepSeek, Moonshot) or `fingerprint_stable_key` (`key:<16 hex>`, Kimi, MiniMax); changing a provider's
scheme would change its account ids. Adding a key whose account already exists replaces
the stored key in the existing home. `headroom accounts remove` deletes the home and, for providers
that take API keys, the stored key.

## Adding a provider

1. Module `crates/headroom-providers/src/<id>/`: `mod.rs` (`pub const ID`, `pub static DESCRIPTOR`,
   `impl Provider`), `auth.rs`, `client.rs` (raw types), `mapper.rs`, `local_usage.rs` if the tool
   logs usage, `fixtures/` with anonymised real responses.
2. Descriptor: display name, add-account methods in order of preference (`CliLogin` with its home
   variable and credentials file, `ApiKey` with label, console URL and hint, or `AutoDetect` with a
   reason), `multi_account`, `local_usage`, and `links` (status page, dashboard, usage page; only
   `https` addresses that were checked to exist, `ProviderLinks::NONE` when there are none). A
   provider whose status page has a documented API can also get an entry in
   `headroom-daemon::status::sources` with the components that matter for it.
3. One entry in `registry::ENTRIES` with a builder that reads the provider's environment.
4. Discovery: CLI homes and Headroom-owned homes under `$XDG_DATA_HOME/headroom/accounts/<id>/`;
   API-key providers use `key_accounts::discover` and override `validate_key`.
5. Map "no active plan" answers to `ProviderError::NoSubscription { detail }` with a provider-specific
   message.
6. Tests: mapper and local log parser against fixtures, registry validation passes, and update
   `crates/headroom/src/render/fixtures/providers.json` (`UPDATE_SNAPSHOTS=1 cargo test -p headroom`).
7. Icon: `shell/*/icons/<id>.svg`; shells fall back to a generic icon when it is missing.

## Daemon

- **Scheduler**: per account, refresh every 5 min with ±10 % jitter; on the popup opening (D-Bus
  `Refresh`) refresh accounts older than 60 s. Single-flight per account; a forced request during a
  refresh queues one follow-up. `Refresh(account_id)` forces that account now (inside a rate-limit
  hold at most once per 60 s) and publishes it as `refreshing` at once; every refresh reads
  credentials from disk again, so a retry after a new CLI sign-in picks it up. Failures back off
  exponentially 60 s → 30 min with jitter; 429 honours `Retry-After` (at most 1 h), otherwise
  consecutive rate limits back off 5 → 10 → 20 → 40 → 60 min with jitter. A descriptor's
  `min_poll_interval` (Claude: 180 s, its usage endpoint is shared with Claude Code's own polling)
  is a floor for every scheduled delay of that provider's accounts. A rate limit with data keeps the
  account `fresh`/`stale` with `error.kind` `rate_limited`. Per-call timeout 30 s. `Refresh(account_id)` of an account whose
  error is `account_changed`, `not_signed_in` or `sign_in_expired` runs a (coalesced) rescan first.
- **Adaptive refresh** (`activity`): the usage watcher records when a usage home gets new log
  records; while a home has records from the last 10 minutes and `adaptive_refresh` is on, its
  accounts are scheduled every 60 s, or the provider's `min_poll_interval` when longer (never
  shortening backoff, rate-limit holds or `no_subscription` rechecks). The state reports `accounts[].refresh {mode, interval_secs, next_at, reason}`.
- **Credential watcher** (`credentials`): accounts whose last error is a sign-in error or
  `account_changed` and whose provider signs in through a CLI get a `notify` watch on their
  credential file, re-evaluated every 5 s; a change settles for 2 s and then acts like
  `Refresh(account_id)`. Headroom-owned Codex, Claude (Linux) and Grok homes refresh their own OAuth
  tokens in the provider crate under a lock with compare-before-write; CLI-owned files are never
  written. The state's `accounts[].recovery` tells shells what helps (`retry`, `sign_in`,
  `cli_login`).
- **Discovery**: every 10 min and on D-Bus `Rescan` (coalesced; a request during a running discovery
  waits for one follow-up). Accounts found by a rescan refresh at once; `headroom accounts add|remove`
  call it. Discovery that finds nothing to discover (`ProviderError::NotSignedIn`: tool not installed
  or not signed in) logs at `debug`; real discovery failures log at `warn`.
- **Usage**: the usage home set is refreshed with every discovery (a failed or timed-out
  `usage_homes` keeps that provider's previous set). inotify on each usage home, debounced 2 s, plus a
  60 s poll fallback. Events are stored per home and a session is logged in one home only, so summing
  homes never double counts.
  Without a running daemon, `headroom status` reads cached usage for every stored usage home whose
  directory still exists. Events keep the session's working directory as `project` (Claude and
  Codex); aggregation adds the last 7 days, top projects and blended cost per million priced tokens.
  `usage::report` answers `GetSpend` queries by model, project, provider or day, and `headroom spend`
  runs the same pure code on the database directly when no daemon is running.
- **Storage** (`$XDG_STATE_HOME/headroom/headroom.db` on Linux, `~/Library/Application Support/Headroom/headroom.db` on macOS, WAL): `accounts`, `limits_snapshots` (last good per
  account), `usage_events`, `log_cursors`, `notification_state`, `subscription_lapses`,
  `dismissed_homes` (dismissed CLI records), `settings`, `update_check` (last update check: time,
  `ETag`, latest stable release), `held_alerts` (alerts held during quiet hours), `status_cache`
  (last answer and `ETag` of each status page); `usage_events.project` since migration 006.
  Migrations are numbered SQL files applied in order.
- **Staleness**: a snapshot older than 10 min is `stale`. A failed refresh keeps the last good snapshot
  and attaches the error.
- **Notifications** (`org.freedesktop.Notifications`): milestones `AlmostOut` (remaining under
  `notifications.threshold_percent`, 10 % by default, or the provider's own threshold; `0` turns it
  off),
  `CuttingItClose` (severity rises to Close), `WillRunOut` (rises to RunningOut/Spent), `Reset` (a
  window that was Warning or worse has reset). First observation primes without alerting. State
  (fired set per window + `resets_at`) is persisted so restarts do not re-alert. Default action opens
  the popup via the shell. Hidden accounts, dismissed accounts and hidden windows
  (`display.hidden_windows`) are skipped.
  Texts are English or Russian per `display.language` (`system` resolves from `LC_ALL` /
  `LC_MESSAGES` / `LANG` at daemon start-up); all texts live in `notify/text.rs`. Titles name the
  provider by its registry display name. During quiet hours alerts go to `held_alerts` and are
  released as one summary when the range ends, dropping items that no longer apply
  (`notify::quiet`, `notify::held`, `notify::digest`).
- **Status pages** (`status`, only with `status_pages.enabled`): a poller reads the Atlassian
  Statuspage or incident.io page of each provider with a listed, visible account every 5 min ± 10 %
  with `ETag`s and per-page backoff, computes the indicator from a fixed component list per provider
  (`status::sources`, pure `assess`) and publishes `provider_status`. The HTTP side is injected by
  the binary.
- **Logging**: the binary installs a `tracing` subscriber with a reloadable filter; the daemon
  applies `logging.level` on every settings change unless `RUST_LOG` is set, and writes
  `headroom.log` (`0600`, rotated at 2 MiB) next to stderr. `GetDiagnostics` reports versions,
  platform, log settings and account health without ids, emails or labels.
- **Update checks** (`headroom-daemon::update`): the daemon owns the schedule, storage and state
  (`update` in the payload); the binary injects the HTTP side as a `ReleaseFeed` (GitHub
  `releases/latest` with `ETag`) and the detected install method (`self` from the install receipt,
  `package` from the `/usr/share/headroom/installed-by-package` marker, `unknown`). First check 2 min
  after start or 24 h after the stored last check, then every 24 h ± 10 %; rate limits wait at least
  1 h, failures retry after 1 h and are logged at `debug` only. `updates.check` and
  `headroom daemon --no-update-check` turn it off. Version comparison, release parsing and command
  texts are pure; see [Update checks](dbus-api.md#update-checks).
- **Self-update** (`headroom update`, binary): downloads `SHA256SUMS` and `SHA256SUMS.sig`,
  verifies the Ed25519 signature against the release key pinned in the binary
  (`update::signature::RELEASE_KEY`, the same key as `packaging/release/release-signing-key.pub.pem`;
  a missing or invalid signature is a hard failure), then downloads the tarball, verifies SHA-256,
  unpacks with `tar` into a temp dir and runs the bundled `install.sh` with the receipt's options.
  When the receipt records a `tray` variant, it also downloads
  `headroom-tray-<version>-<arch>-<variant>.tar.gz`, verifies it against the same signed
  `SHA256SUMS` and places it as the bundle's `tray/` directory, which `install.sh --tray` installs.
  Package and unknown installs get instructions instead. The update feed has its own HTTP client:
  HTTPS only, requests and redirects limited to `api.github.com`, `github.com`,
  `objects.githubusercontent.com` and `release-assets.githubusercontent.com`, and response bodies
  capped (release JSON 1 MiB, `SHA256SUMS` 64 KiB, signature 1 KiB, tarball 64 MiB).

## Transports

All commands and events go through one transport-neutral core; D-Bus and the socket are thin
adapters over it.

- `service::Service` wraps `Core` plus the rescan channel. Commands with more than one step
  (`DismissAccount`, `RestoreAccounts` rescan afterwards) live there, so both transports behave the
  same. `CommandError::is_invalid_argument` decides between D-Bus `InvalidArgs` / `Failed` and
  JSON-RPC `-32602` / `-32000`.
- `events::EventSink` (`state_changed`, `open_requested`, own `EventError`) receives events;
  `events::publish_changes` debounces state changes into it. `EventSinks` fans out to every active
  transport: `dbus::signals::BusSignals` emits the D-Bus signals, `ipc::Hub` writes notifications to
  socket subscribers.
- Alerts go through `notify::Notifier`: `DesktopNotifier` (`org.freedesktop.Notifications`) on Linux,
  `ipc::Hub` on other platforms, which sends an `Alert` notification to every `alerts` subscriber and fails
  with `NotifyError::NoSubscribers` when there is none, so the milestone is rolled back and retried.
- `ipc` serves line-delimited JSON-RPC 2.0 on a Unix socket ([ipc.md](ipc.md)): `protocol` parses
  and encodes lines (pure), `dispatch` maps methods to `Service` calls, `connection` runs one reader
  and one writer task per client with concurrent requests (at most 16 commands in flight per
  connection; the reader stops reading until one finishes), `listener` handles the socket file
  (0700 parent created when missing, an exclusive `flock` on a sibling `daemon.lock` held for the
  daemon's lifetime, stale-socket removal after a failed connect, 0600 socket, removal on shutdown
  only while the file is still ours).

| platform | daemon transports | alerts | CLI client |
| --- | --- | --- | --- |
| Linux | D-Bus; socket with `headroom daemon --socket [PATH]` | freedesktop notifications | D-Bus, or the socket when `HEADROOM_SOCKET` is set |
| macOS | socket, always | `Alert` to socket subscribers (the app posts them) | socket |

Default socket path: `$XDG_RUNTIME_DIR/headroom/daemon.sock` on Linux,
`~/Library/Application Support/Headroom/daemon.sock` on macOS, and `$TMPDIR/headroom-<uid>/daemon.sock`
when the path is longer than 103 bytes; that directory must be owned by the user with mode 0700 or the
daemon and CLI refuse it. `zbus`, the D-Bus service and the freedesktop notifier
compile only for `target_os = "linux"`.

Paths on macOS: Headroom's own files live in `~/Library/Application Support/Headroom` (database,
socket, Headroom-owned account homes, secrets fallback); the price cache in
`~/Library/Caches/headroom/pricing`.

## D-Bus API

- Bus name `io.github.daniarjabagin.Headroom`, object `/io/github/daniarjabagin/Headroom`, interface
  `io.github.daniarjabagin.Headroom1`.
- Methods: `GetState() -> s`, `ListProviders() -> s` (compiled-in providers, how to add their
  accounts and their links), `Refresh(account_id: s)` (`""` = all due), `RefreshNow()`, `Rescan()`
  (discover accounts now), `CheckForUpdates() -> s`, `GetSettings() -> s`, `SetSettings(json: s)`,
  `UpdateSettings(patch: s)`, `ResetSettings()`, `GetSpend(query: s) -> s`, `GetDiagnostics() -> s`,
  `SetAccountLabel(account_id: s, label: s)`, `SetAccountOrder(ids: as)`,
  `SetAccountHidden(account_id: s, hidden: b)`, `DismissAccount(account_id: s)` (CLI-owned accounts
  only), `RestoreAccounts(provider: s)` (`""` = all).
- Signals: `StateChanged(state: s)`, `OpenRequested()`.
- Payloads are JSON (easy in GJS, QML and Rust alike), schema versioned by a top-level `"version"`.
  Full schema: `docs/dbus-api.md`, maintained with the daemon.

State payload outline:

```jsonc
{
  "version": 1,
  "generated_at": "2026-09-23T10:00:00Z",
  "update": null,
  "update_check": { "checked_at": "…" },
  "display": { "theme": "system", "language": "system", "value_mode": "left", "…": "copy of settings.display" },
  "headline": { "account_id": "codex:…", "provider": "codex", "provider_name": "Codex", "account_label": "Work", "window": "session",
                "window_label": "Session", "used_percent": 62.0, "remaining_percent": 38.0, "tone": "warning" },
  "panel_items": [{ "…": "headline fields", "value_percent": 38.0, "even_pace_percent": 55.0, "logo": "codex" }],
  "panel_tone": "warning",
  "provider_status": [{ "provider": "codex", "indicator": "none", "tone": "neutral", "…": "…" }],
  "accounts": [{
    "id": "codex:1a2b3c4d5e6f", "provider": "codex", "provider_name": "Codex", "label": "Work", "email": "…", "plan": "Pro", "owner": "cli|headroom",
    "status": "fresh|stale|refreshing|error|signed_out", "error": null, "updated_at": "…", "source": "live|local_log|cache",
    "windows": [{ "id": "session", "label": "Session", "used_percent": 62.0, "remaining_percent": 38.0,
                  "resets_at": "…", "period_seconds": 18000, "tone": "warning",
                  "pace": { "severity": "close", "even_pace_percent": 55.0, "projected_percent": 97.0, "runs_out_at": null },
                  "hidden": false }],
    "balances": [{ "id": "credits", "label": "Credits", "usd_micros": 12500000 }],
    "notices": [],
    "usage_home": "~/.codex",
    "recovery": null, "collapsed": false,
    "refresh": { "mode": "live|idle", "interval_secs": 60, "next_at": "…", "reason": "activity" }
  }],
  "usage": [{ "provider": "codex", "provider_name": "Codex", "usage_home": "~/.codex",
              "today": { "tokens": { "input": 0, "cache_read": 0, "cache_write": 0, "output": 0, "reasoning": 0, "total": 0 }, "cost_usd_micros": 0, "partial": false,
                         "models": [{ "model": "gpt-5.5", "total_tokens": 0, "cost_usd_micros": 0, "partial": false }] },
              "yesterday": { … }, "last_30_days": { … },
              "daily": [{ "date": "2026-09-22", "total_tokens": 0, "cost_usd_micros": 0 }] }],
  "spend": { "today": { "cost_usd_micros": 0, "total_tokens": 0, "partial": false,
                        "by_provider": [{ "provider": "codex", "provider_name": "Codex", "…": "…", "models": [ … ] }],
                        "projects": [ … ], "projects_other": null, "cost_per_mtok_usd_micros": null },
             "yesterday": { … }, "last_7_days": { … }, "last_30_days": { … } }
}
```

Shells format numbers and countdowns for display but never recompute pace, tone or totals.
