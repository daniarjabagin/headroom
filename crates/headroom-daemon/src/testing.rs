use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use headroom_core::account::{AccountId, AccountIdentity, AccountRef, CredentialOwner, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, CliLogin, HomeVar, ProviderDescriptor};
use headroom_core::event::{EventKey, ServiceTier, UsageEvent};
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource, QuotaWindow, WindowId};
use headroom_core::tokens::TokenCounts;
use headroom_core::units::{MicroUsd, Percent, Tokens};
use headroom_core::usage::PriceBook;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

use crate::catalog::ProviderCatalog;
use crate::clock::testing::ManualClock;
use crate::core::{Core, CoreParts};
use crate::home::{HomeDisplay, UsageHome};
use crate::notify::{Notification, Notifier, NotifyError};
use crate::random::FixedRandom;
use crate::storage::Storage;

pub const CODEX: ProviderId = ProviderId::from_static("codex");
pub const CLAUDE: ProviderId = ProviderId::from_static("claude");

pub static CODEX_DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: CODEX,
    display_name: "Codex",
    add_account: &[AddAccountMethod::CliLogin(CliLogin {
        program: "codex",
        args: &["login"],
        home_var: HomeVar::Direct("CODEX_HOME"),
        credentials_file: "auth.json",
        needs_pty: false,
        scrub_env: &[],
    })],
    multi_account: true,
    local_usage: true,
};

pub static CLAUDE_DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    id: CLAUDE,
    display_name: "Claude",
    add_account: &[AddAccountMethod::AutoDetect { reason: "test" }],
    multi_account: true,
    local_usage: true,
};

pub fn descriptor_of(id: &ProviderId) -> &'static ProviderDescriptor {
    if *id == CLAUDE {
        &CLAUDE_DESCRIPTOR
    } else {
        &CODEX_DESCRIPTOR
    }
}

pub fn catalog() -> ProviderCatalog {
    ProviderCatalog::new([&CODEX_DESCRIPTOR, &CLAUDE_DESCRIPTOR])
}

pub fn ts(text: &str) -> Timestamp {
    text.parse().unwrap()
}

pub fn account(provider: ProviderId, name: &str) -> AccountRef {
    let home = PathBuf::from(format!("/home/ada/.{provider}"));
    AccountRef {
        id: AccountId(format!("{provider}:{name}")),
        provider,
        home,
        owner: CredentialOwner::Cli,
    }
}

pub fn usage_home_of(account: &AccountRef) -> UsageHome {
    UsageHome {
        provider: account.provider.clone(),
        home: account.home.clone(),
    }
}

pub fn session(used: f64, resets_at: &str) -> QuotaWindow {
    QuotaWindow {
        id: WindowId::Session,
        label: "Session".into(),
        used: Percent::new(used),
        resets_at: Some(ts(resets_at)),
        period: Some(SignedDuration::from_hours(5)),
    }
}

pub fn weekly(used: f64, resets_at: &str) -> QuotaWindow {
    QuotaWindow {
        id: WindowId::Weekly,
        label: "Weekly".into(),
        used: Percent::new(used),
        resets_at: Some(ts(resets_at)),
        period: Some(SignedDuration::from_hours(7 * 24)),
    }
}

pub fn snapshot(windows: Vec<QuotaWindow>, fetched_at: &str) -> LimitsSnapshot {
    LimitsSnapshot {
        identity: AccountIdentity {
            email: Some("ada@example.com".into()),
            plan: Some("Pro".into()),
            stable_key: "user/account".into(),
        },
        windows,
        balances: Vec::new(),
        notices: Vec::new(),
        fetched_at: ts(fetched_at),
        source: LimitsSource::Live,
    }
}

pub fn event(key: &str, at: &str, model: &str, input: u64, output: u64) -> UsageEvent {
    UsageEvent {
        key: EventKey(key.into()),
        at: ts(at),
        model: model.into(),
        tier: ServiceTier::Standard,
        tokens: TokenCounts {
            input: Tokens(input),
            output: Tokens(output),
            ..TokenCounts::default()
        },
        web_search_requests: 0,
        reported_cost: None,
        project: None,
    }
}

pub struct FlatPrices;

impl PriceBook for FlatPrices {
    fn cost(&self, event: &UsageEvent) -> Option<MicroUsd> {
        if event.model == "unknown" {
            return None;
        }
        Some(MicroUsd(i64::try_from(event.tokens.total().0).ok()? * 2))
    }
}

pub struct FakeProvider {
    pub kind: ProviderId,
    pub accounts: Mutex<Vec<AccountRef>>,
    pub limits: Mutex<Result<LimitsSnapshot, ProviderError>>,
    pub usage: Mutex<Vec<UsageEvent>>,
    pub homes: Mutex<Vec<PathBuf>>,
    pub discoveries: AtomicUsize,
}

impl FakeProvider {
    pub fn new(kind: ProviderId, accounts: Vec<AccountRef>, limits: LimitsSnapshot) -> Self {
        let homes = accounts.iter().map(|a| a.home.clone()).collect();
        FakeProvider {
            kind,
            homes: Mutex::new(homes),
            accounts: Mutex::new(accounts),
            limits: Mutex::new(Ok(limits)),
            usage: Mutex::new(Vec::new()),
            discoveries: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Provider for FakeProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        descriptor_of(&self.kind)
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        self.discoveries.fetch_add(1, Ordering::SeqCst);
        Ok(self.accounts.lock().unwrap().clone())
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(self.homes.lock().unwrap().clone())
    }

    async fn fetch_limits(&self, _account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        self.limits.lock().unwrap().clone()
    }

    fn read_usage(
        &self,
        home: &Path,
        cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError> {
        let events = std::mem::take(&mut *self.usage.lock().unwrap());
        cursors.cursor_mut(&home.join("log.jsonl")).offset += events.len() as u64;
        Ok(events)
    }
}

#[derive(Default)]
pub struct RecordingNotifier {
    pub sent: Mutex<Vec<Notification>>,
    pub failing: AtomicBool,
}

impl RecordingNotifier {
    pub fn texts(&self) -> Vec<(String, String)> {
        self.sent
            .lock()
            .unwrap()
            .iter()
            .map(|n| (n.title.clone(), n.body.clone()))
            .collect()
    }

    pub fn fail(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }
}

#[async_trait]
impl Notifier for RecordingNotifier {
    async fn notify(&self, notification: &Notification) -> Result<(), NotifyError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(NotifyError::NoSubscribers);
        }
        self.sent.lock().unwrap().push(notification.clone());
        Ok(())
    }
}

pub struct Harness {
    pub core: Arc<Core>,
    pub storage: Storage,
    pub clock: Arc<ManualClock>,
    pub notifier: Arc<RecordingNotifier>,
}

pub async fn harness(providers: Vec<Arc<dyn Provider>>) -> Harness {
    let storage = Storage::open_in_memory().unwrap();
    let clock = Arc::new(ManualClock::at("2026-09-23T10:00:00Z"));
    let notifier = Arc::new(RecordingNotifier::default());
    let parts = CoreParts {
        storage: storage.clone(),
        providers,
        catalog: catalog(),
        price_book: Arc::new(FlatPrices),
        clock: clock.clone(),
        random: Arc::new(FixedRandom(0.5)),
        tz: TimeZone::UTC,
        homes: HomeDisplay::new(Some(PathBuf::from("/home/ada"))),
        notifier: notifier.clone(),
        system_locale: crate::notify::text::Locale::En,
    };
    let core = Arc::new(Core::load(parts).await.unwrap());
    crate::registry::discover_all(&core).await;
    Harness {
        core,
        storage,
        clock,
        notifier,
    }
}

pub async fn eventually(condition: impl FnMut() -> bool) {
    poll_until(condition, std::time::Duration::from_millis(10), 500).await;
}

pub async fn eventually_virtual(condition: impl FnMut() -> bool) {
    poll_until(condition, std::time::Duration::from_secs(1), 7_200).await;
}

async fn poll_until(mut condition: impl FnMut() -> bool, step: std::time::Duration, tries: u32) {
    for _ in 0..tries {
        if condition() {
            return;
        }
        tokio::time::sleep(step).await;
    }
    panic!("condition not reached");
}
