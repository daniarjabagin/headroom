#![cfg(target_os = "linux")]

use std::future::poll_fn;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use headroom_core::account::{AccountId, AccountIdentity, AccountRef, CredentialOwner, ProviderId};
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, CliLogin, HomeVar, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::{LimitsSnapshot, LimitsSource, QuotaWindow, WindowId};
use headroom_core::units::{MicroUsd, Percent};
use headroom_core::usage::PriceBook;
use headroom_daemon::catalog::ProviderCatalog;
use headroom_daemon::clock::SystemClock;
use headroom_daemon::notify::text::Locale;
use headroom_daemon::state::payload::AccountStatus;
use headroom_daemon::{BusTarget, DaemonConfig, DaemonError, StatePayload};
use jiff::{SignedDuration, Timestamp};
use tokio::sync::oneshot;
use zbus::export::futures_core::Stream;

#[path = "dbus_service/socket.rs"]
mod socket;

const CODEX: ProviderId = ProviderId::from_static("codex");

static DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
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

struct PrivateBus {
    child: Child,
    address: String,
}

impl PrivateBus {
    fn start() -> Option<PrivateBus> {
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut line = String::new();
        BufReader::new(child.stdout.take()?)
            .read_line(&mut line)
            .ok()?;
        let address = line.trim().to_owned();
        (!address.is_empty()).then_some(PrivateBus { child, address })
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

#[derive(Default)]
struct StaticProvider {
    accounts: Mutex<Vec<AccountRef>>,
    stall: Mutex<Duration>,
}

fn codex(name: &str) -> AccountRef {
    AccountRef {
        id: AccountId(format!("codex:{name}")),
        provider: CODEX,
        home: PathBuf::from(format!("/nonexistent/headroom-test/{name}")),
        owner: CredentialOwner::Cli,
    }
}

impl StaticProvider {
    fn with_work() -> Arc<StaticProvider> {
        let provider = StaticProvider::default();
        provider.set_accounts(vec![codex("work")]);
        Arc::new(provider)
    }

    fn set_accounts(&self, accounts: Vec<AccountRef>) {
        if let Ok(mut current) = self.accounts.lock() {
            *current = accounts;
        }
    }

    fn set_stall(&self, stall: Duration) {
        if let Ok(mut current) = self.stall.lock() {
            *current = stall;
        }
    }

    fn stall(&self) -> Duration {
        self.stall.lock().map(|stall| *stall).unwrap_or_default()
    }
}

#[async_trait]
impl Provider for StaticProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &DESCRIPTOR
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        self.accounts
            .lock()
            .map(|accounts| accounts.clone())
            .map_err(|_| ProviderError::LocalData("poisoned".into()))
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, _account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        tokio::time::sleep(self.stall()).await;
        let now = Timestamp::now();
        Ok(LimitsSnapshot {
            identity: AccountIdentity {
                email: Some("ada@example.com".into()),
                plan: Some("Pro".into()),
                stable_key: "u/a".into(),
            },
            windows: vec![QuotaWindow {
                id: WindowId::Session,
                label: "Session".into(),
                used: Percent::new(10.0),
                resets_at: now.checked_add(SignedDuration::from_hours(4)).ok(),
                period: Some(SignedDuration::from_hours(5)),
            }],
            balances: Vec::new(),
            notices: Vec::new(),
            fetched_at: now,
            source: LimitsSource::Live,
        })
    }

    fn read_usage(
        &self,
        _home: &Path,
        _cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }
}

struct NoPrices;

impl PriceBook for NoPrices {
    fn cost(&self, _: &UsageEvent) -> Option<MicroUsd> {
        None
    }
}

#[zbus::proxy(
    interface = "io.github.headroom.Daemon1",
    default_service = "io.github.headroom.Daemon",
    default_path = "/io/github/headroom/Daemon"
)]
trait Daemon {
    fn get_state(&self) -> zbus::Result<String>;
    fn list_providers(&self) -> zbus::Result<String>;
    fn refresh(&self, account_id: &str) -> zbus::Result<()>;
    fn refresh_now(&self) -> zbus::Result<()>;
    fn rescan(&self) -> zbus::Result<()>;
    fn get_settings(&self) -> zbus::Result<String>;
    fn set_settings(&self, json: &str) -> zbus::Result<()>;
    fn update_settings(&self, patch: &str) -> zbus::Result<()>;
    fn set_account_label(&self, account_id: &str, label: &str) -> zbus::Result<()>;
    fn set_account_order(&self, ids: &[&str]) -> zbus::Result<()>;
    fn set_account_hidden(&self, account_id: &str, hidden: bool) -> zbus::Result<()>;
    #[zbus(signal)]
    fn state_changed(&self, state: String) -> zbus::Result<()>;
}

fn config(
    bus: &PrivateBus,
    db: &Path,
    provider: Arc<StaticProvider>,
    shutdown: oneshot::Receiver<()>,
) -> DaemonConfig {
    let socket = db.with_extension("sock");
    DaemonConfig {
        providers: vec![provider],
        catalog: ProviderCatalog::new([&DESCRIPTOR]),
        price_book: Arc::new(NoPrices),
        db_path: db.to_path_buf(),
        clock: Arc::new(SystemClock),
        tz: jiff::tz::TimeZone::UTC,
        bus: BusTarget::Address(bus.address.clone()),
        socket: Some(socket),
        system_locale: Locale::En,
        shutdown: Box::pin(async move {
            shutdown.await.ok();
        }),
    }
}

async fn state(proxy: &DaemonProxy<'_>) -> Option<StatePayload> {
    let json = proxy.get_state().await.ok()?;
    serde_json::from_str(&json).ok()
}

async fn wait_for_account(proxy: &DaemonProxy<'_>) -> Option<StatePayload> {
    for _ in 0..100 {
        if let Some(state) = state(proxy).await
            && state
                .accounts
                .first()
                .is_some_and(|a| !a.windows.is_empty())
        {
            return Some(state);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    None
}

async fn next_signal<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
    let next = poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx));
    tokio::time::timeout(Duration::from_secs(5), next)
        .await
        .ok()
        .flatten()
}

type Checked = Result<bool, Box<dyn std::error::Error>>;

async fn settings_are_validated(proxy: &DaemonProxy<'_>) -> Checked {
    let rejected = proxy.set_settings(r#"{"refresh_interval_secs":1}"#).await;
    let invalid_args = is_invalid_args(&rejected);
    proxy.set_settings(r#"{"reduced_motion":true}"#).await?;
    let stored = proxy.get_settings().await?;
    Ok(invalid_args && stored.contains(r#""reduced_motion":true"#))
}

fn is_invalid_args(result: &zbus::Result<()>) -> bool {
    matches!(
        result,
        Err(zbus::Error::MethodError(name, _, _))
            if name.as_str() == "org.freedesktop.DBus.Error.InvalidArgs"
    )
}

async fn settings_are_patched(proxy: &DaemonProxy<'_>) -> Checked {
    proxy
        .update_settings(
            r#"{"display":{"theme":"dark","hidden_windows":{"codex:work":["weekly"]}}}"#,
        )
        .await?;
    proxy
        .update_settings(r#"{"display":{"translucent":true,"hidden_windows":{"codex:work":null}}}"#)
        .await?;
    let unknown = proxy
        .update_settings(r#"{"display":{"compact":true}}"#)
        .await;
    let out_of_range = proxy
        .update_settings(r#"{"refresh_interval_secs":1}"#)
        .await;
    let stored: serde_json::Value = serde_json::from_str(&proxy.get_settings().await?)?;
    let display = &stored["display"];
    Ok(is_invalid_args(&unknown)
        && is_invalid_args(&out_of_range)
        && stored["reduced_motion"] == true
        && display["theme"] == "dark"
        && display["translucent"] == true
        && display["hidden_windows"] == serde_json::json!({}))
}

async fn providers_are_listed(proxy: &DaemonProxy<'_>) -> Checked {
    let listed: serde_json::Value = serde_json::from_str(&proxy.list_providers().await?)?;
    let expected = serde_json::json!({
        "version": 1,
        "providers": [{
            "id": "codex",
            "display_name": "Codex",
            "add_account": [{ "kind": "cli_login", "program": "codex" }],
            "multi_account": true,
            "local_usage": true
        }]
    });
    Ok(listed == expected)
}

async fn refresh_accepts_known_accounts(proxy: &DaemonProxy<'_>) -> Checked {
    let unknown_rejected = proxy.refresh("codex:missing").await.is_err();
    proxy.refresh("").await?;
    proxy.refresh("codex:work").await?;
    Ok(unknown_rejected)
}

async fn refresh_now_is_signalled_as_refreshing(
    proxy: &DaemonProxy<'_>,
    provider: &StaticProvider,
) -> Checked {
    let before = state(proxy).await.ok_or("no state")?;
    let mut changes = proxy.receive_state_changed().await?;
    provider.set_stall(Duration::from_secs(1));
    proxy.refresh_now().await?;
    let immediate = state(proxy).await.ok_or("no state")?;
    let mut signalled = false;
    while let Some(signal) = next_signal(&mut changes).await {
        let payload: StatePayload = serde_json::from_str(signal.args()?.state())?;
        if payload.accounts[0].status == AccountStatus::Refreshing {
            signalled = true;
            break;
        }
    }
    provider.set_stall(Duration::ZERO);
    let refreshed = wait_for_refresh_after(proxy, before.accounts[0].updated_at).await;
    Ok(immediate.accounts[0].status == AccountStatus::Refreshing && signalled && refreshed)
}

async fn wait_for_refresh_after(proxy: &DaemonProxy<'_>, previous: Option<Timestamp>) -> bool {
    for _ in 0..100 {
        if let Some(state) = state(proxy).await
            && state.accounts[0].status == AccountStatus::Fresh
            && state.accounts[0].updated_at > previous
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

async fn label_change_is_signalled(proxy: &DaemonProxy<'_>) -> Checked {
    let mut changes = proxy.receive_state_changed().await?;
    proxy.set_account_label("codex:work", "Work").await?;
    while let Some(signal) = next_signal(&mut changes).await {
        let payload: StatePayload = serde_json::from_str(signal.args()?.state())?;
        if payload.accounts[0].label.as_deref() == Some("Work") {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn order_and_visibility_apply(proxy: &DaemonProxy<'_>) -> Checked {
    proxy.set_account_order(&["codex:work"]).await?;
    let duplicate = proxy.set_account_order(&["codex:work", "codex:work"]).await;
    proxy.set_account_hidden("codex:work", true).await?;
    let hidden = state(proxy).await.ok_or("no state")?;
    Ok(duplicate.is_err() && hidden.accounts[0].hidden && hidden.headline.is_none())
}

async fn rescan_picks_up_added_and_removed_accounts(
    proxy: &DaemonProxy<'_>,
    provider: &StaticProvider,
) -> Checked {
    provider.set_accounts(vec![codex("work"), codex("home")]);
    proxy.rescan().await?;
    let added = state(proxy).await.ok_or("no state")?;
    provider.set_accounts(vec![codex("work")]);
    proxy.rescan().await?;
    let removed = state(proxy).await.ok_or("no state")?;
    let ids = |state: &StatePayload| -> Vec<String> {
        state.accounts.iter().map(|a| a.id.clone()).collect()
    };
    Ok(ids(&added) == ["codex:work", "codex:home"] && ids(&removed) == ["codex:work"])
}

#[tokio::test]
async fn serves_state_settings_and_signals_on_a_private_bus() {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("dbus-daemon unavailable, skipping");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let (stop, stopped) = oneshot::channel();
    let provider = StaticProvider::with_work();
    let daemon = tokio::spawn(headroom_daemon::run(config(
        &bus,
        &dir.path().join("a.db"),
        provider.clone(),
        stopped,
    )));
    let client = zbus::connection::Builder::address(bus.address.as_str())
        .unwrap()
        .build()
        .await
        .unwrap();
    let proxy = DaemonProxy::new(&client).await.unwrap();
    let initial = wait_for_account(&proxy).await.unwrap();
    assert_eq!(initial.version, 1);
    assert_eq!(
        initial.accounts[0].email.as_deref(),
        Some("ada@example.com")
    );
    assert_eq!(initial.accounts[0].provider_name, "Codex");
    assert!(providers_are_listed(&proxy).await.unwrap());
    assert!(settings_are_validated(&proxy).await.unwrap());
    assert!(settings_are_patched(&proxy).await.unwrap());
    assert!(refresh_accepts_known_accounts(&proxy).await.unwrap());
    assert!(
        refresh_now_is_signalled_as_refreshing(&proxy, &provider)
            .await
            .unwrap()
    );
    assert!(
        rescan_picks_up_added_and_removed_accounts(&proxy, &provider)
            .await
            .unwrap()
    );
    assert!(label_change_is_signalled(&proxy).await.unwrap());
    assert!(order_and_visibility_apply(&proxy).await.unwrap());
    let socket = dir.path().join("a.sock");
    assert!(socket::follows_the_daemon(&socket, &proxy).await.unwrap());
    let (_keep, second_stopped) = oneshot::channel();
    let second = headroom_daemon::run(config(
        &bus,
        &dir.path().join("b.db"),
        StaticProvider::with_work(),
        second_stopped,
    ))
    .await;
    assert!(matches!(second, Err(DaemonError::AlreadyRunning)));
    assert!(socket.exists());
    stop.send(()).unwrap();
    daemon.await.unwrap().unwrap();
    assert!(!socket.exists());
}
