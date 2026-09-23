use headroom_core::provider::ProviderError;
use headroom_core::quota::{QuotaWindow, WindowId};
use headroom_core::units::Percent;
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use super::raw::{QuotaReply, RawLimit, SubscriptionReply};

const PERCENT_KINDS: [&str; 2] = ["CREDIT_LIMIT", "TOKENS_LIMIT"];
const WEB_SEARCH_KIND: &str = "TIME_LIMIT";
const WEB_SEARCH_ID: &str = "web_search";
const NO_PLAN_PHRASE: &str = "coding plan";
const HOUR: i64 = 3_600;
const DAY: i64 = 24 * HOUR;
const MONTH: i64 = 30 * DAY;

pub(super) fn quota_limits(body: &[u8]) -> Result<Vec<RawLimit>, ProviderError> {
    let reply: QuotaReply = serde_json::from_slice(body).map_err(|error| {
        ProviderError::InvalidResponse(format!("cannot parse the Z.ai quota response: {error}"))
    })?;
    if reply.success == Some(false) {
        return Err(refusal(&reply));
    }
    reply
        .data
        .and_then(|data| data.limits)
        .or(reply.limits)
        .ok_or_else(|| {
            ProviderError::InvalidResponse("the Z.ai quota response has no limits".into())
        })
}

fn refusal(reply: &QuotaReply) -> ProviderError {
    let message = reply.msg.as_deref().unwrap_or_default();
    if message.to_lowercase().contains(NO_PLAN_PHRASE) {
        return super::no_plan();
    }
    let code = match &reply.code {
        Some(Value::Number(code)) => code.to_string(),
        Some(Value::String(code)) => code.clone(),
        _ => "unknown".to_owned(),
    };
    ProviderError::InvalidResponse(format!("Z.ai refused the quota request (code {code})"))
}

pub(super) fn windows(limits: &[RawLimit]) -> Result<Vec<QuotaWindow>, ProviderError> {
    let mut windows: Vec<QuotaWindow> = Vec::new();
    for limit in limits {
        let Some(window) = window(limit)? else {
            continue;
        };
        if !windows.iter().any(|known| known.id == window.id) {
            windows.push(window);
        }
    }
    Ok(windows)
}

fn window(limit: &RawLimit) -> Result<Option<QuotaWindow>, ProviderError> {
    let kind = limit.kind.as_deref().or(limit.name.as_deref());
    match kind {
        Some(kind) if PERCENT_KINDS.contains(&kind) => percent_window(limit),
        Some(WEB_SEARCH_KIND) => web_search_window(limit).map(Some),
        _ => Ok(None),
    }
}

fn percent_window(limit: &RawLimit) -> Result<Option<QuotaWindow>, ProviderError> {
    let Some(period) = period(limit)? else {
        return Ok(None);
    };
    let (id, label) = naming(period);
    Ok(Some(QuotaWindow {
        id,
        label,
        used: used(limit)?,
        resets_at: resets_at(limit),
        period: Some(period),
    }))
}

fn web_search_window(limit: &RawLimit) -> Result<QuotaWindow, ProviderError> {
    Ok(QuotaWindow {
        id: WindowId::Other(WEB_SEARCH_ID.to_owned()),
        label: "Web searches".to_owned(),
        used: used(limit)?,
        resets_at: resets_at(limit),
        period: period(limit).ok().flatten(),
    })
}

fn period(limit: &RawLimit) -> Result<Option<SignedDuration>, ProviderError> {
    let (Some(unit), Some(number)) = (limit.unit, limit.number) else {
        return Err(invalid("a Z.ai quota window has no length"));
    };
    let Some(unit_seconds) = unit_seconds(unit) else {
        tracing::debug!(unit, "skipping a Z.ai quota window with an unknown unit");
        return Ok(None);
    };
    number
        .checked_mul(unit_seconds)
        .filter(|_| number > 0)
        .map(|seconds| Some(SignedDuration::from_secs(seconds)))
        .ok_or_else(|| invalid("a Z.ai quota window has an invalid length"))
}

fn unit_seconds(unit: i64) -> Option<i64> {
    match unit {
        3 => Some(HOUR),
        4 => Some(DAY),
        5 => Some(MONTH),
        6 => Some(7 * DAY),
        _ => None,
    }
}

fn naming(period: SignedDuration) -> (WindowId, String) {
    let seconds = period.as_secs();
    match WindowId::from_period(period) {
        Some(WindowId::Session) => (WindowId::Session, "Session".to_owned()),
        Some(WindowId::Weekly) => (WindowId::Weekly, "Weekly".to_owned()),
        _ if seconds == MONTH => (WindowId::Other("monthly".into()), "Monthly".to_owned()),
        _ if seconds % DAY == 0 => {
            let days = seconds / DAY;
            (WindowId::Other(format!("{days}d")), format!("{days}-day"))
        }
        _ => {
            let hours = seconds / HOUR;
            (
                WindowId::Other(format!("{hours}h")),
                format!("{hours}-hour"),
            )
        }
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a display percentage of two exact integer counts"
)]
fn used(limit: &RawLimit) -> Result<Percent, ProviderError> {
    match (limit.current_value, limit.usage) {
        (Some(current), Some(total)) if total > 0 => {
            Ok(Percent::new(current as f64 * 100.0 / total as f64))
        }
        _ => limit
            .percentage
            .map(Percent::new)
            .ok_or_else(|| invalid("a Z.ai quota window has no usage")),
    }
}

fn resets_at(limit: &RawLimit) -> Option<Timestamp> {
    limit
        .next_reset_time
        .and_then(|millis| Timestamp::from_millisecond(millis).ok())
}

pub(super) fn plan(body: &[u8]) -> Option<String> {
    let reply: SubscriptionReply = serde_json::from_slice(body).ok()?;
    reply
        .data?
        .into_iter()
        .find_map(|subscription| subscription.product_name)
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
}

fn invalid(text: &str) -> ProviderError {
    ProviderError::InvalidResponse(text.to_owned())
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
