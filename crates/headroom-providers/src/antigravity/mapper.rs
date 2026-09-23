use headroom_core::quota::{QuotaWindow, WindowId};
use headroom_core::units::Percent;
use jiff::{SignedDuration, Timestamp};

use super::raw::{RawBucket, RawCodeAssist, RawGroup, RawUserStatusEnvelope};

struct Pool {
    bucket: &'static str,
    id: fn() -> WindowId,
    label: &'static str,
    period: SignedDuration,
}

const POOLS: [Pool; 4] = [
    Pool {
        bucket: "gemini-5h",
        id: || WindowId::Session,
        label: "Session",
        period: WindowId::SESSION_PERIOD,
    },
    Pool {
        bucket: "gemini-weekly",
        id: || WindowId::Weekly,
        label: "Weekly",
        period: WindowId::WEEKLY_PERIOD,
    },
    Pool {
        bucket: "3p-5h",
        id: || WindowId::Model("claude".into()),
        label: "Claude",
        period: WindowId::SESSION_PERIOD,
    },
    Pool {
        bucket: "3p-weekly",
        id: || WindowId::Model("claude:weekly".into()),
        label: "Claude weekly",
        period: WindowId::WEEKLY_PERIOD,
    },
];

const PLAN_PREFIX: &str = "Google AI ";
const PLAN_WORDS: [&str; 3] = ["Ultra", "Pro", "Free"];

pub(super) fn windows(groups: &[RawGroup]) -> Vec<QuotaWindow> {
    let buckets: Vec<RawBucket> = groups
        .iter()
        .flat_map(|group| group.buckets.iter())
        .filter_map(lenient_bucket)
        .collect();
    POOLS
        .iter()
        .filter_map(|pool| {
            let bucket = buckets
                .iter()
                .find(|bucket| bucket.bucket_id == pool.bucket)?;
            Some(window(pool, bucket))
        })
        .collect()
}

fn lenient_bucket(value: &serde_json::Value) -> Option<RawBucket> {
    match serde_json::from_value::<RawBucket>(value.clone()) {
        Ok(bucket) if bucket.remaining_fraction.is_finite() => Some(bucket),
        _ => {
            tracing::warn!("antigravity quota bucket without a usable fraction; not shown");
            None
        }
    }
}

fn window(pool: &Pool, bucket: &RawBucket) -> QuotaWindow {
    let remaining = bucket.remaining_fraction.clamp(0.0, 1.0);
    QuotaWindow {
        id: (pool.id)(),
        label: pool.label.to_owned(),
        used: Percent::new(100.0 - remaining * 100.0),
        resets_at: bucket
            .reset_time
            .as_deref()
            .and_then(|text| text.parse::<Timestamp>().ok()),
        period: Some(pool.period),
    }
}

pub(super) fn language_server_plan(status: &RawUserStatusEnvelope) -> Option<String> {
    let status = status.user_status.as_ref()?;
    let tier = status
        .user_tier
        .as_ref()
        .and_then(|tier| tier.name.as_deref());
    let plan_name = status
        .plan_status
        .as_ref()
        .and_then(|plan| plan.plan_info.as_ref())
        .and_then(|info| info.plan_name.as_deref());
    format_plan(tier.or(plan_name))
}

pub(super) fn cloud_plan(assist: &RawCodeAssist) -> Option<String> {
    let paid = assist
        .paid_tier
        .as_ref()
        .and_then(|tier| tier.name.as_deref());
    let current = assist
        .current_tier
        .as_ref()
        .and_then(|tier| tier.name.as_deref());
    format_plan(paid.or(current))
}

fn format_plan(raw: Option<&str>) -> Option<String> {
    let text = raw.map(str::trim).filter(|text| !text.is_empty())?;
    if let Some(rest) = text.strip_prefix(PLAN_PREFIX) {
        return Some(rest.trim().to_owned());
    }
    let lower = text.to_lowercase();
    PLAN_WORDS
        .iter()
        .find(|word| lower.contains(&word.to_lowercase()))
        .map_or_else(|| Some(text.to_owned()), |word| Some((*word).to_owned()))
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
