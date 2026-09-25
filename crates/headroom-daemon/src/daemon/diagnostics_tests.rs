use std::path::PathBuf;

use headroom_core::account::CredentialOwner;
use serde_json::{Value, json};

use super::*;
use crate::state::payload::AccountError;
use crate::testing::{CLAUDE, CODEX, harness, ts};

struct EnvControl;

impl LogControl for EnvControl {
    fn apply(&self, _level: LogLevel) {}

    fn status(&self) -> LogStatus {
        LogStatus {
            level: EffectiveLevel::Trace,
            source: LevelSource::Env,
            file: Some(PathBuf::from(
                "/home/ada/.local/state/headroom/headroom.log",
            )),
        }
    }
}

fn account(provider: &ProviderId, name: &str) -> AccountView {
    AccountView {
        id: format!("{provider}:{name}"),
        provider: provider.clone(),
        provider_name: provider.to_string(),
        label: Some(format!("Label {name}")),
        email: Some(format!("{name}@mail.example")),
        plan: Some("Pro".into()),
        hidden: false,
        owner: CredentialOwner::Cli,
        status: AccountStatus::Fresh,
        error: None,
        updated_at: Some(ts("2026-09-23T09:58:00Z")),
        source: Some(DataSource::Live),
        windows: Vec::new(),
        balances: Vec::new(),
        notices: Vec::new(),
        usage_home: format!("~/.{provider}-{name}"),
    }
}

fn failing(provider: &ProviderId, name: &str) -> AccountView {
    AccountView {
        status: AccountStatus::Error,
        error: Some(AccountError {
            kind: "network".into(),
            message: format!("could not reach the API for {name}@mail.example"),
        }),
        updated_at: None,
        source: None,
        hidden: true,
        owner: CredentialOwner::Headroom,
        ..account(provider, name)
    }
}

fn context(logging: Option<Arc<dyn LogControl>>) -> DiagnosticsContext {
    DiagnosticsContext {
        started_at: ts("2026-09-23T07:55:00Z"),
        system: SystemInfo {
            os: Some("Arch Linux".into()),
            desktop: Some("GNOME (wayland)".into()),
        },
        transports: vec![TransportKind::Dbus, TransportKind::Socket],
        logging,
        homes: HomeDisplay::new(Some(PathBuf::from("/home/ada"))),
    }
}

fn report(logging: Option<Arc<dyn LogControl>>) -> Diagnostics {
    let accounts = [account(&CLAUDE, "ada"), failing(&CODEX, "grace")];
    let facts = Facts {
        now: ts("2026-09-23T10:00:30Z"),
        settings_level: LogLevel::Warn,
        providers: &[CODEX, CLAUDE, ProviderId::from_static("grok")],
        accounts: &accounts,
        usage_homes: &[CLAUDE, CLAUDE],
    };
    context(logging).build(&facts)
}

#[test]
fn the_report_never_names_accounts() {
    let json = serde_json::to_string(&report(None)).unwrap();
    for secret in [
        "ada@mail.example",
        "grace@mail.example",
        "Label ada",
        "Label grace",
        "claude:ada",
        "codex:grace",
        "~/.claude-ada",
        "could not reach",
    ] {
        assert!(!json.contains(secret), "{secret} leaked into {json}");
    }
}

#[test]
fn the_report_has_the_documented_shape() {
    let value = serde_json::to_value(report(None)).unwrap();
    let Value::Object(fields) = &value else {
        panic!("not an object");
    };
    let keys: Vec<&str> = fields.keys().map(String::as_str).collect();
    let mut expected = [
        "app_version",
        "os",
        "desktop",
        "uptime_secs",
        "transports",
        "log_level",
        "log_level_source",
        "log_file",
        "providers",
        "accounts",
        "text",
    ];
    expected.sort_unstable();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, expected);
    assert_eq!(value["app_version"], APP_VERSION);
    assert_eq!(value["uptime_secs"], 7_530);
    assert_eq!(value["transports"], json!(["dbus", "socket"]));
    assert_eq!(value["log_level"], "warn");
    assert_eq!(value["log_level_source"], "settings");
    assert_eq!(value["log_file"], Value::Null);
    assert_eq!(
        value["providers"],
        json!([
            {"provider": "codex", "accounts": 1, "usage_homes": 0},
            {"provider": "claude", "accounts": 1, "usage_homes": 2}
        ])
    );
    assert_eq!(
        value["accounts"],
        json!([
            {"provider": "claude", "status": "fresh", "error_kind": null, "source": "live",
             "owner": "cli", "hidden": false, "updated_at": "2026-09-23T09:58:00Z"},
            {"provider": "codex", "status": "error", "error_kind": "network", "source": null,
             "owner": "headroom", "hidden": true, "updated_at": null}
        ])
    );
}

#[test]
fn the_log_control_reports_the_effective_level_and_file() {
    let report = report(Some(Arc::new(EnvControl)));
    assert_eq!(report.log_level, EffectiveLevel::Trace);
    assert_eq!(report.log_level_source, LevelSource::Env);
    assert_eq!(
        report.log_file.as_deref(),
        Some("~/.local/state/headroom/headroom.log")
    );
}

#[test]
fn the_text_is_ready_to_paste() {
    let text = report(Some(Arc::new(EnvControl))).text;
    let expected = format!(
        "Headroom {APP_VERSION}\n\
         OS: Arch Linux\n\
         Desktop: GNOME (wayland)\n\
         Uptime: 2h 5m\n\
         IPC: dbus, socket\n\
         Log level: trace (RUST_LOG)\n\
         Log file: ~/.local/state/headroom/headroom.log\n\
         Providers:\n  \
         codex: 1 account(s), 0 usage home(s)\n  \
         claude: 1 account(s), 2 usage home(s)\n\
         Accounts:\n  \
         1. claude · fresh · live · cli · updated 2026-09-23T09:58:00Z\n  \
         2. codex · error · error network · headroom · hidden\n"
    );
    assert_eq!(text, expected);
}

#[test]
fn uptime_reads_naturally() {
    assert_eq!(uptime_text(42), "0m 42s");
    assert_eq!(uptime_text(3_725), "1h 2m");
    assert_eq!(uptime_text(2 * 86_400 + 3_600 + 60), "2d 1h 1m");
}

#[test]
fn a_clock_behind_the_start_reports_zero_uptime() {
    assert_eq!(
        uptime_secs(ts("2026-09-23T10:00:00Z"), ts("2026-09-23T09:00:00Z")),
        0
    );
}

#[test]
fn without_a_daemon_the_text_keeps_the_local_facts() {
    let system = SystemInfo {
        os: None,
        desktop: Some("KDE (x11)".into()),
    };
    assert_eq!(
        offline_text(&system, Some("~/.local/state/headroom/headroom.log")),
        format!(
            "Headroom {APP_VERSION}\nOS: unknown\nDesktop: KDE (x11)\nDaemon: not running\n\
             Log file: ~/.local/state/headroom/headroom.log\n"
        )
    );
}

#[tokio::test]
async fn collecting_reads_the_live_state() {
    let harness = harness(Vec::new()).await;
    harness
        .core
        .update_settings(r#"{"logging":{"level":"debug"}}"#)
        .await
        .unwrap();
    let report = context(None).collect(&harness.core);
    assert_eq!(report.log_level, EffectiveLevel::Debug);
    assert_eq!(report.uptime_secs, 7_500);
    assert!(report.accounts.is_empty());
    assert!(report.providers.is_empty());
}
