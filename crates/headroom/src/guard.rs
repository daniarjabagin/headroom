use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::Result;
use headroom_core::pace::Tone;
use headroom_daemon::state::payload::{AccountStatus, AccountView, StatePayload, WindowView};
use jiff::Timestamp;
use serde::Serialize;

use crate::cli::{GuardArgs, WindowScope};
use crate::client;
use crate::paths::Globals;
use crate::render::format::{account_title, shown_windows};
use crate::render::guard::{json_report, no_data_text, verdict_text};
use crate::render::style::Palette;

const EXIT_BELOW: u8 = 1;
const EXIT_NO_DATA: u8 = 2;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckedLimit {
    pub account_id: String,
    pub provider: String,
    pub window: String,
    pub remaining_percent: f64,
    pub resets_at: Option<Timestamp>,
    pub tone: Tone,
    #[serde(skip)]
    pub name: String,
    #[serde(skip)]
    pub window_label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub min_percent: u8,
    pub window: WindowScope,
    pub checked: Vec<CheckedLimit>,
    pub failing: Vec<CheckedLimit>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.failing.is_empty()
    }

    pub fn lowest(&self) -> Option<&CheckedLimit> {
        self.checked
            .iter()
            .min_by(|a, b| a.remaining_percent.total_cmp(&b.remaining_percent))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoData {
    DaemonNotRunning,
    Unreadable(String),
    NoMatchingLimits,
    NoFreshLimits(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Checked(Report),
    NoData(NoData),
}

impl Outcome {
    pub fn exit_status(&self) -> u8 {
        match self {
            Outcome::Checked(report) if report.ok() => 0,
            Outcome::Checked(_) => EXIT_BELOW,
            Outcome::NoData(_) => EXIT_NO_DATA,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palettes {
    pub stdout: Palette,
    pub stderr: Palette,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Printed {
    Nothing,
    Stdout(String),
    Stderr(String),
}

pub async fn run(globals: &Globals, args: &GuardArgs) -> Result<ExitCode> {
    let outcome = match current_state(globals).await {
        Ok(state) => evaluate(&state, args),
        Err(reason) => Outcome::NoData(reason),
    };
    match printed(&outcome, args, Timestamp::now(), detect_palettes())? {
        Printed::Nothing => {}
        Printed::Stdout(text) => write!(io::stdout().lock(), "{text}")?,
        Printed::Stderr(text) => write!(io::stderr().lock(), "{text}")?,
    }
    Ok(ExitCode::from(outcome.exit_status()))
}

async fn current_state(globals: &Globals) -> Result<StatePayload, NoData> {
    let daemon = match client::running_daemon(globals).await {
        Ok(Some(daemon)) => daemon,
        Ok(None) => return Err(NoData::DaemonNotRunning),
        Err(error) => return Err(NoData::Unreadable(format!("{error:#}"))),
    };
    match client::fetch_state(&daemon).await {
        Ok((_, state)) => Ok(state),
        Err(error) => Err(NoData::Unreadable(format!("{error:#}"))),
    }
}

fn detect_palettes() -> Palettes {
    let no_color = std::env::var_os("NO_COLOR");
    Palettes {
        stdout: Palette::detect(no_color.as_deref(), io::stdout().is_terminal()),
        stderr: Palette::detect(no_color.as_deref(), io::stderr().is_terminal()),
    }
}

pub fn printed(
    outcome: &Outcome,
    args: &GuardArgs,
    now: Timestamp,
    palettes: Palettes,
) -> serde_json::Result<Printed> {
    if args.quiet {
        return Ok(Printed::Nothing);
    }
    Ok(match outcome {
        Outcome::Checked(report) if args.json => {
            Printed::Stdout(format!("{}\n", json_report(report)?))
        }
        Outcome::Checked(report) => Printed::Stdout(verdict_text(report, now, palettes.stdout)),
        Outcome::NoData(reason) => {
            Printed::Stderr(format!("{}\n", no_data_text(reason, palettes.stderr)))
        }
    })
}

pub fn evaluate(state: &StatePayload, args: &GuardArgs) -> Outcome {
    let accounts = selected_accounts(state, args);
    let checked = checked_limits(&accounts, args);
    if checked.is_empty() {
        return Outcome::NoData(missing_reason(&accounts));
    }
    let min = f64::from(args.min);
    let failing = checked
        .iter()
        .filter(|limit| limit.remaining_percent < min)
        .cloned()
        .collect();
    Outcome::Checked(Report {
        min_percent: args.min,
        window: args.window,
        checked,
        failing,
    })
}

fn selected_accounts<'a>(state: &'a StatePayload, args: &GuardArgs) -> Vec<&'a AccountView> {
    state
        .accounts
        .iter()
        .filter(|account| selected(account, args))
        .collect()
}

fn checked_limits(accounts: &[&AccountView], args: &GuardArgs) -> Vec<CheckedLimit> {
    accounts
        .iter()
        .filter(|account| has_fresh_data(account))
        .flat_map(|account| {
            let name = limit_name(account, accounts);
            shown_windows(account)
                .filter(|window| args.window.includes(&window.id))
                .map(move |window| checked_limit(account, window, name.clone()))
        })
        .collect()
}

fn missing_reason(accounts: &[&AccountView]) -> NoData {
    let excluded: Vec<String> = accounts
        .iter()
        .filter_map(|account| {
            let reason = exclusion_reason(account)?;
            Some(format!("{}: {reason}", limit_name(account, accounts)))
        })
        .collect();
    if excluded.is_empty() {
        NoData::NoMatchingLimits
    } else {
        NoData::NoFreshLimits(excluded)
    }
}

fn has_fresh_data(account: &AccountView) -> bool {
    status_problem(account.status).is_none()
}

fn exclusion_reason(account: &AccountView) -> Option<String> {
    let problem = status_problem(account.status)?;
    Some(
        account
            .error
            .as_ref()
            .map_or_else(|| problem.to_owned(), |error| error.message.clone()),
    )
}

fn status_problem(status: AccountStatus) -> Option<&'static str> {
    match status {
        AccountStatus::Fresh | AccountStatus::Refreshing => None,
        AccountStatus::Stale => Some("data is outdated"),
        AccountStatus::Error => Some("couldn't refresh"),
        AccountStatus::SignedOut => Some("signed out"),
        AccountStatus::NoSubscription => Some("no active subscription"),
    }
}

fn selected(account: &AccountView, args: &GuardArgs) -> bool {
    let provider = account.provider.as_str();
    !account.hidden
        && (args.provider.is_empty() || args.provider.iter().any(|id| id == provider))
        && args.account.as_deref().is_none_or(|id| id == account.id)
}

fn limit_name(account: &AccountView, selected: &[&AccountView]) -> String {
    let siblings = selected
        .iter()
        .filter(|other| other.provider == account.provider)
        .count();
    if siblings > 1 {
        account_title(account)
    } else {
        account.provider_name.clone()
    }
}

fn checked_limit(account: &AccountView, window: &WindowView, name: String) -> CheckedLimit {
    CheckedLimit {
        account_id: account.id.clone(),
        provider: account.provider.as_str().to_owned(),
        window: window.id.clone(),
        remaining_percent: window.remaining_percent,
        resets_at: window.resets_at,
        tone: window.tone,
        name,
        window_label: window.label.clone(),
    }
}

#[cfg(test)]
#[path = "guard_tests.rs"]
mod tests;
