use headroom_core::pace::Tone;
use headroom_core::provider::ProviderError;
use headroom_core::quota::{Balance, BalanceAmount, Notice, QuotaWindow, WindowId};
use headroom_core::units::Percent;
use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;

use super::raw::{RawGrant, RawRequestLimitInfo, RawUser};

pub(super) const MONTHLY_ID: &str = "monthly";
pub(super) const BONUS_ID: &str = "bonus_credits";
pub(super) const UNLIMITED_NOTICE: &str = "Unlimited credits";
pub(super) const NO_CREDITS_NOTICE: &str = "No monthly credits on this plan";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Mapped {
    pub(super) windows: Vec<QuotaWindow>,
    pub(super) balances: Vec<Balance>,
    pub(super) notices: Vec<Notice>,
}

pub(super) fn map(user: &RawUser) -> Result<Mapped, ProviderError> {
    let limits = &user.request_limit_info;
    let (windows, notices) = if limits.is_unlimited {
        (Vec::new(), vec![neutral(UNLIMITED_NOTICE)])
    } else if limits.request_limit == 0 {
        (Vec::new(), vec![neutral(NO_CREDITS_NOTICE)])
    } else {
        (vec![monthly_window(limits)?], Vec::new())
    };
    Ok(Mapped {
        windows,
        balances: bonus_balance(user).into_iter().collect(),
        notices,
    })
}

fn monthly_window(limits: &RawRequestLimitInfo) -> Result<QuotaWindow, ProviderError> {
    let resets_at = limits
        .next_refresh_time
        .as_deref()
        .map(parse_time)
        .transpose()?;
    Ok(QuotaWindow {
        id: WindowId::Other(MONTHLY_ID.to_owned()),
        label: "Monthly credits".to_owned(),
        used: share_of(
            limits.requests_used_since_last_refresh,
            limits.request_limit,
        ),
        resets_at,
        period: None,
    })
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a display percentage of two exact credit counts"
)]
fn share_of(used: u64, limit: u64) -> Percent {
    Percent::new(used as f64 * 100.0 / limit as f64)
}

fn parse_time(text: &str) -> Result<Timestamp, ProviderError> {
    text.parse::<Timestamp>()
        .or_else(|_| {
            text.parse::<DateTime>()
                .and_then(|civil| civil.to_zoned(TimeZone::UTC))
                .map(|zoned| zoned.timestamp())
        })
        .map_err(|_| {
            ProviderError::InvalidResponse(format!("Warp sent an unreadable refresh time {text:?}"))
        })
}

fn bonus_balance(user: &RawUser) -> Option<Balance> {
    let grants: Vec<&RawGrant> = user_grants(user).chain(workspace_grants(user)).collect();
    if grants.is_empty() {
        return None;
    }
    let remaining = grants.iter().fold(0_u64, |sum, grant| {
        sum.saturating_add(grant.request_credits_remaining)
    });
    Some(Balance {
        id: BONUS_ID.to_owned(),
        label: "Bonus credits".to_owned(),
        amount: BalanceAmount::Count {
            value: remaining,
            unit: "credits".to_owned(),
        },
    })
}

fn user_grants(user: &RawUser) -> impl Iterator<Item = &RawGrant> {
    user.bonus_grants.iter().flatten()
}

fn workspace_grants(user: &RawUser) -> impl Iterator<Item = &RawGrant> {
    user.workspaces
        .iter()
        .flatten()
        .filter_map(|workspace| workspace.bonus_grants_info.as_ref())
        .filter_map(|info| info.grants.as_ref())
        .flatten()
}

fn neutral(text: &str) -> Notice {
    Notice {
        tone: Tone::Neutral,
        text: text.to_owned(),
    }
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
