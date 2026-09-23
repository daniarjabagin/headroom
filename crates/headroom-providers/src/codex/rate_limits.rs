use std::fs;
use std::path::{Path, PathBuf};

use headroom_core::account::AccountIdentity;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{LimitsSnapshot, LimitsSource};
use headroom_core::units::Percent;
use jiff::{SignedDuration, Timestamp};
use serde::Deserialize;

use super::client::RawCredits;
use super::local_usage::rollout_files;
use super::mapper::{credits_balance, period_from_seconds, with_plan};
use super::number::FlexNumber;
use super::reverse::find_last_line;
use super::timestamp::{from_epoch_seconds, parse_log_timestamp};
use super::windows::{Family, Slot, WindowReading, classify};

const MAIN_LIMIT_ID: &str = "codex";
const SECONDS_PER_MINUTE: u64 = 60;

#[derive(Debug, Deserialize)]
struct RolloutLine {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    payload: Option<RolloutPayload>,
}

#[derive(Debug, Deserialize)]
struct RolloutPayload {
    #[serde(rename = "type")]
    kind: Option<String>,
    rate_limits: Option<RolloutRateLimits>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RolloutRateLimits {
    limit_id: Option<String>,
    primary: Option<RolloutWindow>,
    secondary: Option<RolloutWindow>,
    credits: Option<RawCredits>,
    plan_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RolloutWindow {
    used_percent: Option<FlexNumber>,
    window_minutes: Option<FlexNumber>,
    resets_at: Option<FlexNumber>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Observation {
    pub observed_at: Timestamp,
    pub limits: RolloutRateLimits,
}

pub(super) fn latest_snapshot(
    home: &Path,
    identity: AccountIdentity,
    now: Timestamp,
) -> Result<Option<LimitsSnapshot>, ProviderError> {
    Ok(latest_observation(home)?.map(|observation| snapshot(&observation, identity, now)))
}

pub(super) fn latest_observation(home: &Path) -> Result<Option<Observation>, ProviderError> {
    let mut best: Option<Observation> = None;
    for (path, modified) in newest_first(rollout_files(home)?) {
        if best
            .as_ref()
            .is_some_and(|best| best.observed_at >= modified)
        {
            break;
        }
        let found = find_last_line(&path, parse_observation).map_err(|error| {
            ProviderError::LocalData(format!("cannot read {}: {error}", path.display()))
        })?;
        if let Some(found) = found
            && best
                .as_ref()
                .is_none_or(|best| found.observed_at > best.observed_at)
        {
            best = Some(found);
        }
    }
    Ok(best)
}

pub(super) fn snapshot(
    observation: &Observation,
    identity: AccountIdentity,
    now: Timestamp,
) -> LimitsSnapshot {
    let limits = &observation.limits;
    LimitsSnapshot {
        identity: with_plan(identity, limits.plan_type.as_deref()),
        windows: classify(Family::Main, readings(limits, now)),
        balances: limits
            .credits
            .as_ref()
            .and_then(|credits| credits.balance.as_ref()?.whole())
            .map(credits_balance)
            .into_iter()
            .collect(),
        notices: Vec::new(),
        fetched_at: observation.observed_at,
        source: LimitsSource::LocalLog {
            observed_at: observation.observed_at,
        },
    }
}

fn newest_first(files: Vec<PathBuf>) -> Vec<(PathBuf, Timestamp)> {
    let mut stamped: Vec<(PathBuf, Timestamp)> = files
        .into_iter()
        .filter_map(|path| {
            let modified = fs::metadata(&path).ok()?.modified().ok()?;
            Some((path, Timestamp::try_from(modified).ok()?))
        })
        .collect();
    stamped.sort_by_key(|(_, modified)| std::cmp::Reverse(*modified));
    stamped
}

fn parse_observation(line: &str) -> Option<Observation> {
    if !line.contains("\"rate_limits\"") {
        return None;
    }
    let raw: RolloutLine = serde_json::from_str(line).ok()?;
    let payload = raw.payload?;
    let is_token_count =
        raw.kind.as_deref() == Some("event_msg") && payload.kind.as_deref() == Some("token_count");
    let limits = payload
        .rate_limits
        .filter(|limits| is_token_count && is_main_limit(limits))?;
    Some(Observation {
        observed_at: parse_log_timestamp(raw.timestamp.as_deref()?)?,
        limits,
    })
}

fn is_main_limit(limits: &RolloutRateLimits) -> bool {
    let main_id = limits
        .limit_id
        .as_deref()
        .is_none_or(|id| id == MAIN_LIMIT_ID);
    let has_window = [&limits.primary, &limits.secondary]
        .into_iter()
        .flatten()
        .any(|window| window.used_percent.is_some());
    main_id && has_window
}

fn readings(limits: &RolloutRateLimits, now: Timestamp) -> Vec<WindowReading> {
    [
        (Slot::Primary, &limits.primary),
        (Slot::Secondary, &limits.secondary),
    ]
    .into_iter()
    .filter_map(|(slot, window)| reading(slot, window.as_ref()?, now))
    .collect()
}

fn reading(slot: Slot, window: &RolloutWindow, now: Timestamp) -> Option<WindowReading> {
    let used = Percent::new(window.used_percent.as_ref()?.as_f64()?);
    let resets_at = window
        .resets_at
        .as_ref()
        .and_then(FlexNumber::as_f64)
        .and_then(from_epoch_seconds);
    let has_reset = resets_at.is_some_and(|reset| reset <= now);
    Some(WindowReading {
        slot,
        used: if has_reset { Percent::ZERO } else { used },
        period: window.window_minutes.as_ref().and_then(minutes_to_period),
        resets_at: resets_at.filter(|_| !has_reset),
    })
}

fn minutes_to_period(minutes: &FlexNumber) -> Option<SignedDuration> {
    period_from_seconds(minutes.whole()?.checked_mul(SECONDS_PER_MINUTE)?)
}

#[cfg(test)]
#[path = "rate_limits_tests.rs"]
mod tests;
