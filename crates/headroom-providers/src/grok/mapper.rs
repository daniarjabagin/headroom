use headroom_core::account::AccountIdentity;
use headroom_core::pace::Tone;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{LimitsSnapshot, LimitsSource, Notice, QuotaWindow, WindowId};
use headroom_core::units::Percent;
use jiff::Timestamp;
use serde_json::Number;

use super::client::{RawBilling, RawCreditsConfig, RawPeriod, RawSettings};

const WEEKLY_PERIOD_TYPE: &str = "USAGE_PERIOD_TYPE_WEEKLY";
const NO_SUBSCRIPTION: &str = "No active Grok subscription.";
const LEGACY_BILLING: &str = "Legacy Grok billing has no weekly pool.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PlanLookup {
    Found(String),
    Missing,
    Failed,
}

impl PlanLookup {
    pub(super) fn from_settings(settings: Result<RawSettings, ProviderError>) -> PlanLookup {
        match settings {
            Ok(settings) => settings
                .subscription_tier_display
                .map(|tier| tier.trim().to_owned())
                .filter(|tier| !tier.is_empty())
                .map_or(PlanLookup::Missing, PlanLookup::Found),
            Err(error) => {
                tracing::warn!(%error, "grok plan name unavailable");
                PlanLookup::Failed
            }
        }
    }

    fn name(&self) -> Option<String> {
        match self {
            PlanLookup::Found(name) => Some(name.clone()),
            PlanLookup::Missing | PlanLookup::Failed => None,
        }
    }
}

pub(super) fn no_subscription() -> ProviderError {
    ProviderError::NoSubscription {
        detail: NO_SUBSCRIPTION.to_owned(),
    }
}

pub(super) fn map_limits(
    billing: &RawBilling,
    plan: &PlanLookup,
    identity: AccountIdentity,
    now: Timestamp,
) -> Result<LimitsSnapshot, ProviderError> {
    let config = &billing.config;
    let weekly = weekly_window(config)?;
    if weekly.is_none() && *plan == PlanLookup::Missing {
        return Err(no_subscription());
    }
    let legacy = weekly
        .is_none()
        .then(|| notice(Tone::Neutral, LEGACY_BILLING));
    Ok(LimitsSnapshot {
        identity: AccountIdentity {
            plan: plan.name(),
            ..identity
        },
        windows: weekly.into_iter().collect(),
        balances: Vec::new(),
        notices: legacy.into_iter().chain([extra_usage(config)]).collect(),
        fetched_at: now,
        source: LimitsSource::Live,
    })
}

fn weekly_window(config: &RawCreditsConfig) -> Result<Option<QuotaWindow>, ProviderError> {
    let Some(period) = config
        .current_period
        .as_ref()
        .filter(|period| period.kind.as_deref().map(str::trim) == Some(WEEKLY_PERIOD_TYPE))
    else {
        return Ok(None);
    };
    let (start, end) = period_bounds(period)?;
    Ok(Some(QuotaWindow {
        id: WindowId::Weekly,
        label: "Weekly".to_owned(),
        used: Percent::new(config.credit_usage_percent.unwrap_or(0.0)),
        resets_at: Some(end),
        period: Some(end.duration_since(start)),
    }))
}

fn period_bounds(period: &RawPeriod) -> Result<(Timestamp, Timestamp), ProviderError> {
    let parse = |value: Option<&str>| value.and_then(|text| text.trim().parse::<Timestamp>().ok());
    match (parse(period.start.as_deref()), parse(period.end.as_deref())) {
        (Some(start), Some(end)) if end > start => Ok((start, end)),
        _ => Err(ProviderError::InvalidResponse(
            "billing period has no valid start and end".into(),
        )),
    }
}

fn extra_usage(config: &RawCreditsConfig) -> Notice {
    let cap = config
        .on_demand_cap
        .as_ref()
        .and_then(|value| value.val.as_ref())
        .filter(|cap| cap.as_f64().is_some_and(|value| value > 0.0));
    match cap {
        Some(cap) => notice(Tone::Good, &format!("Extra usage on, cap {}", display(cap))),
        None => notice(Tone::Neutral, "Extra usage off"),
    }
}

fn display(number: &Number) -> String {
    number
        .as_u64()
        .map_or_else(|| number.to_string(), |whole| whole.to_string())
}

fn notice(tone: Tone, text: &str) -> Notice {
    Notice {
        tone,
        text: text.to_owned(),
    }
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
