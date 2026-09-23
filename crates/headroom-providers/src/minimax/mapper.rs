use headroom_core::provider::ProviderError;
use headroom_core::quota::{QuotaWindow, WindowId};
use headroom_core::units::Percent;
use jiff::{SignedDuration, Timestamp};

use super::raw::{RawBaseResp, RawModel, RawNumber, RawRemains};
use crate::plan_error::mentions_plan;

const GENERAL_MODEL: &str = "general";
const UNLIMITED_STATUS: i64 = 3;
const BASE_ALLOWANCE: f64 = 100.0;
const PERMILLE_PER_PERCENT: f64 = 10.0;
const AUTH_FAILED_CODES: [i64; 2] = [1004, 2049];
const RATE_LIMITED_CODE: i64 = 1002;
pub(super) const NO_PLAN: &str = "this MiniMax key has no Token Plan";

struct WindowSpec {
    id: WindowId,
    label: &'static str,
    default_period: SignedDuration,
}

struct WindowFields<'a> {
    status: Option<&'a RawNumber>,
    remaining: Option<&'a RawNumber>,
    start: Option<&'a RawNumber>,
    end: Option<&'a RawNumber>,
    allowance: f64,
}

pub(super) fn map_remains(raw: &RawRemains) -> Result<Vec<QuotaWindow>, ProviderError> {
    if let Some(base) = &raw.base_resp {
        check_status(base)?;
    }
    let general = general_model(raw)?;
    let windows: Vec<QuotaWindow> = [session(&general), weekly(&general)]
        .into_iter()
        .flatten()
        .collect();
    if windows.is_empty() {
        return Err(invalid("the MiniMax general quota has no usable windows"));
    }
    Ok(windows)
}

pub(super) fn check_status(base: &RawBaseResp) -> Result<(), ProviderError> {
    let message = base.status_msg.as_deref().unwrap_or_default();
    match base.status_code {
        0 => Ok(()),
        code if AUTH_FAILED_CODES.contains(&code) => Err(ProviderError::SignInExpired),
        RATE_LIMITED_CODE => Err(ProviderError::RateLimited { retry_after: None }),
        _ if message_mentions_plan(message) => Err(ProviderError::NoSubscription {
            detail: NO_PLAN.to_owned(),
        }),
        code => Err(invalid(&format!(
            "MiniMax returned status {code}: {message}"
        ))),
    }
}

fn message_mentions_plan(message: &str) -> bool {
    serde_json::to_vec(message).is_ok_and(|json| mentions_plan(&json))
}

fn invalid(problem: &str) -> ProviderError {
    ProviderError::InvalidResponse(problem.to_owned())
}

fn general_model(raw: &RawRemains) -> Result<RawModel, ProviderError> {
    raw.model_remains
        .iter()
        .flatten()
        .filter_map(|value| serde_json::from_value::<RawModel>(value.clone()).ok())
        .find(|model| model.model_name.as_deref() == Some(GENERAL_MODEL))
        .ok_or_else(|| invalid("the MiniMax response has no general quota"))
}

fn session(model: &RawModel) -> Option<QuotaWindow> {
    let spec = WindowSpec {
        id: WindowId::Session,
        label: "Session",
        default_period: WindowId::SESSION_PERIOD,
    };
    let fields = WindowFields {
        status: model.current_interval_status.as_ref(),
        remaining: model.current_interval_remaining_percent.as_ref(),
        start: model.start_time.as_ref(),
        end: model.end_time.as_ref(),
        allowance: BASE_ALLOWANCE,
    };
    window(&spec, &fields)
}

fn weekly(model: &RawModel) -> Option<QuotaWindow> {
    let spec = WindowSpec {
        id: WindowId::Weekly,
        label: "Weekly",
        default_period: WindowId::WEEKLY_PERIOD,
    };
    let fields = WindowFields {
        status: model.current_weekly_status.as_ref(),
        remaining: model.current_weekly_remaining_percent.as_ref(),
        start: model.weekly_start_time.as_ref(),
        end: model.weekly_end_time.as_ref(),
        allowance: boosted_allowance(model.weekly_boost_permille.as_ref()),
    };
    window(&spec, &fields)
}

fn boosted_allowance(permille: Option<&RawNumber>) -> f64 {
    permille
        .and_then(RawNumber::integer)
        .and_then(|permille| u32::try_from(permille).ok())
        .filter(|permille| *permille > 0)
        .map_or(BASE_ALLOWANCE, |permille| {
            f64::from(permille) / PERMILLE_PER_PERCENT
        })
}

fn window(spec: &WindowSpec, fields: &WindowFields<'_>) -> Option<QuotaWindow> {
    let unlimited = fields.status.and_then(RawNumber::integer) == Some(UNLIMITED_STATUS);
    let used = if unlimited {
        Percent::ZERO
    } else {
        used_percent(fields)?
    };
    let label = if unlimited {
        format!("{} (Unlimited)", spec.label)
    } else {
        spec.label.to_owned()
    };
    let end = fields.end.and_then(RawNumber::integer);
    Some(QuotaWindow {
        id: spec.id.clone(),
        label,
        used,
        resets_at: end.and_then(|ms| Timestamp::from_millisecond(ms).ok()),
        period: Some(period(fields.start.and_then(RawNumber::integer), end, spec)),
    })
}

fn used_percent(fields: &WindowFields<'_>) -> Option<Percent> {
    let Some(remaining) = fields.remaining.and_then(RawNumber::decimal) else {
        tracing::warn!("skipping MiniMax quota without a remaining percentage");
        return None;
    };
    if remaining < 0.0 {
        tracing::warn!(
            remaining,
            "skipping MiniMax quota with a negative remaining percentage"
        );
        return None;
    }
    let allowance = fields.allowance.max(remaining);
    Some(Percent::new(
        (allowance - remaining) / allowance * BASE_ALLOWANCE,
    ))
}

fn period(start: Option<i64>, end: Option<i64>, spec: &WindowSpec) -> SignedDuration {
    start
        .zip(end)
        .and_then(|(start, end)| end.checked_sub(start))
        .filter(|ms| *ms > 0)
        .map_or(spec.default_period, SignedDuration::from_millis)
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
