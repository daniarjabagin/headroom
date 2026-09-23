use headroom_core::provider::ProviderError;
use headroom_core::quota::LimitsSnapshot;

use super::labels::plan_label;

const UNPAID_PLANS: [&str; 2] = ["free", "guest"];

pub(super) fn require_subscription(
    plan_type: Option<&str>,
    snapshot: LimitsSnapshot,
) -> Result<LimitsSnapshot, ProviderError> {
    let has_data = !snapshot.windows.is_empty() || !snapshot.balances.is_empty();
    if has_data || is_paid(plan_type) {
        Ok(snapshot)
    } else {
        Err(no_subscription(plan_type))
    }
}

pub(super) fn no_subscription(plan_type: Option<&str>) -> ProviderError {
    let detail = match known_plan(plan_type) {
        Some(plan) => format!(
            "No active ChatGPT subscription ({} plan).",
            plan_label(plan)
        ),
        None => "No active ChatGPT subscription.".to_owned(),
    };
    ProviderError::NoSubscription { detail }
}

fn is_paid(plan_type: Option<&str>) -> bool {
    known_plan(plan_type)
        .is_some_and(|plan| !UNPAID_PLANS.contains(&plan.to_ascii_lowercase().as_str()))
}

fn known_plan(plan_type: Option<&str>) -> Option<&str> {
    plan_type.map(str::trim).filter(|plan| !plan.is_empty())
}

#[cfg(test)]
mod tests {
    use headroom_core::account::AccountIdentity;
    use headroom_core::quota::{LimitsSource, QuotaWindow, WindowId};
    use headroom_core::units::Percent;

    use super::super::test_support::at;
    use super::*;

    fn snapshot(windows: Vec<QuotaWindow>) -> LimitsSnapshot {
        LimitsSnapshot {
            identity: AccountIdentity {
                email: None,
                plan: None,
                stable_key: "u/a".into(),
            },
            windows,
            balances: Vec::new(),
            notices: Vec::new(),
            fetched_at: at("2026-09-23T10:00:00Z"),
            source: LimitsSource::Live,
        }
    }

    fn weekly() -> QuotaWindow {
        QuotaWindow {
            id: WindowId::Weekly,
            label: "Weekly".into(),
            used: Percent::new(3.0),
            resets_at: None,
            period: None,
        }
    }

    fn detail(result: Result<LimitsSnapshot, ProviderError>) -> Option<String> {
        match result {
            Err(ProviderError::NoSubscription { detail }) => Some(detail),
            _ => None,
        }
    }

    #[test]
    fn unpaid_or_missing_plan_without_limits_has_no_subscription() {
        let cases = [
            (
                Some("free"),
                Some("No active ChatGPT subscription (Free plan)."),
            ),
            (
                Some(" Guest "),
                Some("No active ChatGPT subscription (Guest plan)."),
            ),
            (None, Some("No active ChatGPT subscription.")),
            (Some(""), Some("No active ChatGPT subscription.")),
            (Some("plus"), None),
            (Some("prolite"), None),
        ];
        for (plan, expected) in cases {
            let result = require_subscription(plan, snapshot(Vec::new()));
            assert_eq!(detail(result).as_deref(), expected, "{plan:?}");
        }
    }

    #[test]
    fn any_reported_limit_keeps_the_snapshot() {
        let kept = require_subscription(Some("free"), snapshot(vec![weekly()])).unwrap();
        assert_eq!(kept.windows, [weekly()]);
        assert!(require_subscription(None, snapshot(vec![weekly()])).is_ok());
    }
}
