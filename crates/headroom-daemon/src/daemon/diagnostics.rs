use std::fmt::Write as _;
use std::sync::Arc;

use headroom_core::account::{CredentialOwner, ProviderId};
use jiff::Timestamp;
use serde::Serialize;

use super::log_level::{EffectiveLevel, LevelSource, LogControl, LogStatus};
use super::system_info::SystemInfo;
use crate::core::Core;
use crate::home::HomeDisplay;
use crate::settings::LogLevel;
use crate::state::payload::{APP_VERSION, AccountStatus, AccountView, DataSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    Dbus,
    Socket,
}

pub struct DiagnosticsContext {
    pub started_at: Timestamp,
    pub system: SystemInfo,
    pub transports: Vec<TransportKind>,
    pub logging: Option<Arc<dyn LogControl>>,
    pub homes: HomeDisplay,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diagnostics {
    pub app_version: &'static str,
    pub os: Option<String>,
    pub desktop: Option<String>,
    pub uptime_secs: u64,
    pub transports: Vec<TransportKind>,
    pub log_level: EffectiveLevel,
    pub log_level_source: LevelSource,
    pub log_file: Option<String>,
    pub providers: Vec<ProviderCounts>,
    pub accounts: Vec<AccountDiagnostics>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderCounts {
    pub provider: ProviderId,
    pub accounts: usize,
    pub usage_homes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccountDiagnostics {
    pub provider: ProviderId,
    pub status: AccountStatus,
    pub error_kind: Option<String>,
    pub source: Option<DataSource>,
    pub owner: CredentialOwner,
    pub hidden: bool,
    pub updated_at: Option<Timestamp>,
}

pub struct Facts<'a> {
    pub now: Timestamp,
    pub settings_level: LogLevel,
    pub providers: &'a [ProviderId],
    pub accounts: &'a [AccountView],
    pub usage_homes: &'a [ProviderId],
}

impl DiagnosticsContext {
    #[must_use]
    pub fn new(started_at: Timestamp) -> DiagnosticsContext {
        DiagnosticsContext {
            started_at,
            system: SystemInfo::default(),
            transports: Vec::new(),
            logging: None,
            homes: HomeDisplay::default(),
        }
    }

    #[must_use]
    pub fn collect(&self, core: &Core) -> Diagnostics {
        let state = core.state();
        let providers: Vec<ProviderId> = core
            .catalog
            .payload()
            .providers
            .into_iter()
            .map(|view| view.id)
            .collect();
        let (settings_level, usage_homes) = {
            let model = core.model();
            let homes: Vec<ProviderId> = model
                .usage_homes
                .iter()
                .map(|home| home.provider.clone())
                .collect();
            (model.settings.logging.level, homes)
        };
        let facts = Facts {
            now: core.clock.now(),
            settings_level,
            providers: &providers,
            accounts: &state.accounts,
            usage_homes: &usage_homes,
        };
        self.build(&facts)
    }

    #[must_use]
    pub fn build(&self, facts: &Facts<'_>) -> Diagnostics {
        let log = self.log_status(facts.settings_level);
        let mut report = Diagnostics {
            app_version: APP_VERSION,
            os: self.system.os.clone(),
            desktop: self.system.desktop.clone(),
            uptime_secs: uptime_secs(self.started_at, facts.now),
            transports: self.transports.clone(),
            log_level: log.level,
            log_level_source: log.source,
            log_file: log.file.as_deref().map(|path| self.homes.show(path)),
            providers: provider_counts(facts),
            accounts: facts.accounts.iter().map(account_diagnostics).collect(),
            text: String::new(),
        };
        report.text = render_text(&report);
        report
    }

    fn log_status(&self, settings_level: LogLevel) -> LogStatus {
        self.logging.as_ref().map_or_else(
            || LogStatus {
                level: settings_level.into(),
                source: LevelSource::Settings,
                file: None,
            },
            |control| control.status(),
        )
    }
}

fn uptime_secs(started_at: Timestamp, now: Timestamp) -> u64 {
    u64::try_from(now.duration_since(started_at).as_secs()).unwrap_or(0)
}

fn provider_counts(facts: &Facts<'_>) -> Vec<ProviderCounts> {
    facts
        .providers
        .iter()
        .map(|provider| ProviderCounts {
            provider: provider.clone(),
            accounts: facts
                .accounts
                .iter()
                .filter(|a| &a.provider == provider)
                .count(),
            usage_homes: facts.usage_homes.iter().filter(|p| *p == provider).count(),
        })
        .filter(|counts| counts.accounts > 0 || counts.usage_homes > 0)
        .collect()
}

fn account_diagnostics(account: &AccountView) -> AccountDiagnostics {
    AccountDiagnostics {
        provider: account.provider.clone(),
        status: account.status,
        error_kind: account.error.as_ref().map(|error| error.kind.clone()),
        source: account.source,
        owner: account.owner,
        hidden: account.hidden,
        updated_at: account.updated_at,
    }
}

#[must_use]
pub fn offline_text(system: &SystemInfo, log_file: Option<&str>) -> String {
    let mut text = header(system);
    line(&mut text, "Daemon", "not running");
    line(&mut text, "Log file", log_file.unwrap_or("none"));
    text
}

fn render_text(report: &Diagnostics) -> String {
    let system = SystemInfo {
        os: report.os.clone(),
        desktop: report.desktop.clone(),
    };
    let mut text = header(&system);
    line(&mut text, "Uptime", &uptime_text(report.uptime_secs));
    line(&mut text, "IPC", &transports_text(&report.transports));
    let level = format!(
        "{} ({})",
        report.log_level.as_str(),
        report.log_level_source.as_str()
    );
    line(&mut text, "Log level", &level);
    line(
        &mut text,
        "Log file",
        report.log_file.as_deref().unwrap_or("none"),
    );
    text.push_str("Providers:\n");
    for counts in &report.providers {
        let _ = writeln!(
            text,
            "  {}: {} account(s), {} usage home(s)",
            counts.provider, counts.accounts, counts.usage_homes
        );
    }
    text.push_str("Accounts:\n");
    for (index, account) in report.accounts.iter().enumerate() {
        let _ = writeln!(text, "  {}. {}", index + 1, account_text(account));
    }
    text
}

fn header(system: &SystemInfo) -> String {
    let mut text = format!("Headroom {APP_VERSION}\n");
    line(&mut text, "OS", system.os.as_deref().unwrap_or("unknown"));
    line(
        &mut text,
        "Desktop",
        system.desktop.as_deref().unwrap_or("unknown"),
    );
    text
}

fn line(text: &mut String, name: &str, value: &str) {
    let _ = writeln!(text, "{name}: {value}");
}

fn transports_text(transports: &[TransportKind]) -> String {
    let names: Vec<&str> = transports
        .iter()
        .map(|kind| match kind {
            TransportKind::Dbus => "dbus",
            TransportKind::Socket => "socket",
        })
        .collect();
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
}

fn uptime_text(secs: u64) -> String {
    let (days, hours, minutes) = (secs / 86_400, secs % 86_400 / 3_600, secs % 3_600 / 60);
    match (days, hours) {
        (0, 0) => format!("{minutes}m {}s", secs % 60),
        (0, _) => format!("{hours}h {minutes}m"),
        _ => format!("{days}d {hours}h {minutes}m"),
    }
}

fn account_text(account: &AccountDiagnostics) -> String {
    let mut parts = vec![account.provider.to_string(), json_name(&account.status)];
    if let Some(kind) = &account.error_kind {
        parts.push(format!("error {kind}"));
    }
    if let Some(source) = &account.source {
        parts.push(json_name(source));
    }
    parts.push(json_name(&account.owner));
    if account.hidden {
        parts.push("hidden".to_owned());
    }
    if let Some(updated_at) = account.updated_at {
        parts.push(format!("updated {updated_at}"));
    }
    parts.join(" · ")
}

fn json_name(value: &impl Serialize) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(name)) => name,
        _ => String::new(),
    }
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
