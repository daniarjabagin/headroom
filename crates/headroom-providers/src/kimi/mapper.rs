use std::collections::BTreeMap;

use headroom_core::provider::ProviderError;
use headroom_core::quota::{QuotaWindow, WindowId};
use headroom_core::units::Percent;
use jiff::{SignedDuration, Timestamp};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::raw::{RawDetail, RawLimit, RawPool, RawUsages, RawWindow};

const LEVEL_PREFIX: &str = "LEVEL_";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct MappedUsage {
    pub(super) plan: Option<String>,
    pub(super) windows: Vec<QuotaWindow>,
}

pub(super) fn map_usage(raw: &RawUsages) -> Result<MappedUsage, ProviderError> {
    let mut windows = WindowList::default();
    for value in raw.limits.iter().flatten() {
        windows.push(parse::<RawLimit>(value, "limit").and_then(|limit| limit_window(&limit)));
    }
    if let Some(usage) = &raw.usage {
        let weekly = parse::<RawDetail>(usage, "usage").and_then(|detail| {
            counted_window(WindowId::Weekly, Some(WindowId::WEEKLY_PERIOD), &detail)
        });
        windows.push(weekly);
    }
    for (key, value) in pools(raw) {
        windows.push(pool_window(key, value));
    }
    let windows = windows.finish();
    if windows.is_empty() {
        return Err(ProviderError::InvalidResponse(
            "the Kimi usage response has no quota windows".to_owned(),
        ));
    }
    Ok(MappedUsage {
        plan: plan_name(raw),
        windows,
    })
}

#[derive(Default)]
struct WindowList(Vec<QuotaWindow>);

impl WindowList {
    fn push(&mut self, window: Option<QuotaWindow>) {
        if let Some(window) = window
            && !self.0.iter().any(|known| known.id == window.id)
        {
            self.0.push(window);
        }
    }

    fn finish(mut self) -> Vec<QuotaWindow> {
        self.0.sort_by_key(|window| rank(&window.id));
        self.0
    }
}

fn rank(id: &WindowId) -> u8 {
    match id {
        WindowId::Session => 0,
        WindowId::Weekly => 1,
        WindowId::Model(_) | WindowId::Other(_) => 2,
    }
}

fn parse<T: DeserializeOwned>(value: &Value, section: &str) -> Option<T> {
    serde_json::from_value(value.clone())
        .inspect_err(|error| tracing::warn!(section, %error, "skipping malformed Kimi quota"))
        .ok()
}

fn pools(raw: &RawUsages) -> impl Iterator<Item = (&String, &Value)> {
    let wrapped = raw
        .data
        .as_ref()
        .and_then(|data| data.quota.as_ref())
        .and_then(|quota| quota.usages.as_ref());
    raw.usages.iter().chain(wrapped).flat_map(BTreeMap::iter)
}

fn limit_window(limit: &RawLimit) -> Option<QuotaWindow> {
    let period = window_period(&limit.window)?;
    let id = WindowId::from_period(period).unwrap_or_else(|| other_id(period));
    counted_window(id, Some(period), &limit.detail)
}

fn window_period(window: &RawWindow) -> Option<SignedDuration> {
    let unit: i64 = match window.time_unit.as_str() {
        "TIME_UNIT_SECOND" => 1,
        "TIME_UNIT_MINUTE" => 60,
        "TIME_UNIT_HOUR" => 3_600,
        "TIME_UNIT_DAY" => 86_400,
        other => {
            tracing::warn!(
                unit = other,
                "skipping Kimi quota with an unknown time unit"
            );
            return None;
        }
    };
    let count = i64::try_from(window.duration.exact()?).ok()?;
    let seconds = count.checked_mul(unit).filter(|seconds| *seconds > 0)?;
    Some(SignedDuration::from_secs(seconds))
}

fn other_id(period: SignedDuration) -> WindowId {
    WindowId::Other(format!("{}s", period.as_secs()))
}

fn counted_window(
    id: WindowId,
    period: Option<SignedDuration>,
    detail: &RawDetail,
) -> Option<QuotaWindow> {
    let Some(used) = used_percent(detail) else {
        tracing::warn!(window = ?id, "skipping Kimi quota without usable counts");
        return None;
    };
    Some(QuotaWindow {
        label: label(&id, period),
        id,
        used,
        resets_at: detail.reset_time.as_deref().and_then(reset_time),
        period,
    })
}

fn used_percent(detail: &RawDetail) -> Option<Percent> {
    let limit = detail.limit.exact().filter(|limit| *limit > 0)?;
    let used = match &detail.used {
        Some(used) => used.exact()?,
        None => limit.saturating_sub(detail.remaining.as_ref()?.exact()?),
    };
    Some(ratio_percent(used, limit))
}

#[allow(
    clippy::cast_precision_loss,
    reason = "exact integer counts become a display percentage only here"
)]
fn ratio_percent(used: u64, limit: u64) -> Percent {
    Percent::new(used as f64 * 100.0 / limit as f64)
}

fn pool_window(key: &str, value: &Value) -> Option<QuotaWindow> {
    let (id, period) = pool_kind(key)?;
    let pool = parse::<RawPool>(value, key)?;
    let ratio = pool.used_ratio.value()?;
    Some(QuotaWindow {
        label: label(&id, period),
        id,
        used: Percent::new(ratio * 100.0),
        resets_at: pool.reset_time.as_deref().and_then(reset_time),
        period,
    })
}

fn pool_kind(key: &str) -> Option<(WindowId, Option<SignedDuration>)> {
    let normalized: String = key.chars().filter(|c| *c != '_').collect();
    match normalized.to_ascii_lowercase().as_str() {
        "limit5h" => Some((WindowId::Session, Some(WindowId::SESSION_PERIOD))),
        "limit7d" => Some((WindowId::Weekly, Some(WindowId::WEEKLY_PERIOD))),
        "limitmonthtotal" => Some((WindowId::Other("monthly".to_owned()), None)),
        "limitmonthcode" => Some((WindowId::Other("monthly_code".to_owned()), None)),
        _ => {
            tracing::debug!(pool = key, "ignoring unknown Kimi quota pool");
            None
        }
    }
}

fn label(id: &WindowId, period: Option<SignedDuration>) -> String {
    match (id, period) {
        (WindowId::Session, _) => "Session".to_owned(),
        (WindowId::Weekly, _) => "Weekly".to_owned(),
        (WindowId::Other(name), _) if name == "monthly" => "Monthly".to_owned(),
        (WindowId::Other(name), _) if name == "monthly_code" => "Monthly (Code)".to_owned(),
        (_, Some(period)) => period_label(period),
        (WindowId::Model(name) | WindowId::Other(name), None) => name.clone(),
    }
}

fn period_label(period: SignedDuration) -> String {
    let seconds = period.as_secs();
    let (count, unit) = [(86_400, "day"), (3_600, "hour"), (60, "minute")]
        .into_iter()
        .find(|(size, _)| seconds % size == 0)
        .map_or((seconds, "second"), |(size, unit)| (seconds / size, unit));
    format!("{count}-{unit}")
}

fn reset_time(text: &str) -> Option<Timestamp> {
    text.trim().parse().ok()
}

fn plan_name(raw: &RawUsages) -> Option<String> {
    let level = raw.user.as_ref()?.membership.as_ref()?.level.as_deref()?;
    let level = level.trim();
    let name = level.strip_prefix(LEVEL_PREFIX).unwrap_or(level);
    let words: Vec<String> = name
        .split('_')
        .filter(|word| !word.is_empty())
        .map(title_case)
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

fn title_case(word: &str) -> String {
    let lower = word.to_lowercase();
    let mut chars = lower.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
