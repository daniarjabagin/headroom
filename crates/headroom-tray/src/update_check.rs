use jiff::Timestamp;
use serde::Deserialize;

use crate::dates::{Locale, exact_moment};
use crate::format::ago_text;
use crate::i18n::fill;
use crate::payload::UpdateCheck;

const FIRST_CHECKING_DAEMON: [u64; 3] = [0, 6, 0];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    UpToDate,
    Available,
    Failed,
    RateLimited,
    Disabled,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CheckOutcome {
    pub status: CheckStatus,
    #[serde(default)]
    pub checked_at: Option<Timestamp>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub until: Option<Timestamp>,
}

impl CheckOutcome {
    #[must_use]
    pub fn failed(checked_at: Option<Timestamp>) -> Self {
        Self {
            status: CheckStatus::Failed,
            checked_at,
            version: None,
            until: None,
        }
    }
}

pub fn parse_outcome(json: &str) -> Result<CheckOutcome, serde_json::Error> {
    serde_json::from_str(json)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CheckRun {
    #[default]
    Idle,
    Checking,
    Done(CheckOutcome),
}

fn version_parts(version: &str) -> Option<[u64; 3]> {
    let core = version.trim().trim_start_matches('v');
    let core = core.split(['-', '+']).next()?;
    let mut numbers = core.split('.').map(|part| part.parse::<u64>().ok());
    Some([numbers.next()??, numbers.next()??, numbers.next()??])
}

#[must_use]
pub fn daemon_checks_on_demand(app_version: Option<&str>) -> bool {
    app_version
        .and_then(version_parts)
        .is_some_and(|parts| parts >= FIRST_CHECKING_DAEMON)
}

#[must_use]
pub fn check_row_visible(
    app_version: Option<&str>,
    update_check: Option<&UpdateCheck>,
    checks_on: bool,
) -> bool {
    checks_on && update_check.is_some() && daemon_checks_on_demand(app_version)
}

fn current_outcome(run: &CheckRun, checked_at: Option<Timestamp>) -> Option<&CheckOutcome> {
    match run {
        CheckRun::Done(outcome) if outcome.checked_at >= checked_at => Some(outcome),
        CheckRun::Idle | CheckRun::Checking | CheckRun::Done(_) => None,
    }
}

pub struct CheckContext<'a> {
    pub locale: &'a Locale,
    pub version: &'a str,
    pub checked_at: Option<Timestamp>,
    pub now: Timestamp,
}

fn checked(ctx: &CheckContext, template: &'static str, version: &str) -> String {
    let lang = ctx.locale.lang;
    match ctx.checked_at {
        Some(at) => fill(
            lang.tr(template),
            &[("version", version), ("ago", &ago_text(lang, at, ctx.now))],
        ),
        None => fill(
            lang.tr("Headroom {version} · not checked yet"),
            &[("version", version)],
        ),
    }
}

fn failed_line(ctx: &CheckContext) -> String {
    let lang = ctx.locale.lang;
    ctx.checked_at.map_or_else(
        || lang.tr("Couldn't check for updates").to_owned(),
        |at| {
            fill(
                lang.tr("Couldn't check for updates · last checked {ago}"),
                &[("ago", &ago_text(lang, at, ctx.now))],
            )
        },
    )
}

fn rate_limited_line(ctx: &CheckContext, until: Option<Timestamp>) -> String {
    let lang = ctx.locale.lang;
    until.filter(|until| *until > ctx.now).map_or_else(
        || {
            lang.tr("GitHub is limiting checks. Try again later.")
                .to_owned()
        },
        |until| {
            fill(
                lang.tr("GitHub is limiting checks until {moment}"),
                &[("moment", &exact_moment(until, ctx.now, ctx.locale))],
            )
        },
    )
}

const UP_TO_DATE: &str = "You're up to date · Headroom {version} · checked {ago}";

#[must_use]
pub fn check_line(ctx: &CheckContext, run: &CheckRun) -> String {
    if *run == CheckRun::Checking {
        return ctx.locale.lang.tr("Checking for updates…").to_owned();
    }
    let Some(outcome) = current_outcome(run, ctx.checked_at) else {
        return checked(ctx, UP_TO_DATE, ctx.version);
    };
    match outcome.status {
        CheckStatus::Available => {
            let version = outcome.version.as_deref().unwrap_or(ctx.version);
            checked(
                ctx,
                "Headroom {version} is available · checked {ago}",
                version,
            )
        }
        CheckStatus::Failed => failed_line(ctx),
        CheckStatus::RateLimited => rate_limited_line(ctx, outcome.until),
        CheckStatus::Disabled => ctx.locale.lang.tr("Update checks are off").to_owned(),
        CheckStatus::UpToDate | CheckStatus::Unknown => checked(ctx, UP_TO_DATE, ctx.version),
    }
}

#[cfg(test)]
#[path = "update_check_tests.rs"]
mod tests;
