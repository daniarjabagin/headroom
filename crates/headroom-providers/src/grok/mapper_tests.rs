use jiff::SignedDuration;

use super::*;

const WEEKLY: &str = include_str!("fixtures/billing_weekly.json");
const EXTRA_USAGE: &str = include_str!("fixtures/billing_extra_usage.json");
const LEGACY: &str = include_str!("fixtures/billing_legacy.json");
const SETTINGS: &str = include_str!("fixtures/settings.json");

fn billing(text: &str) -> RawBilling {
    serde_json::from_str(text).unwrap()
}

fn identity() -> AccountIdentity {
    AccountIdentity {
        email: Some("ada@example.com".into()),
        plan: None,
        stable_key: "user-fake-1/team-fake-1".into(),
    }
}

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn super_grok() -> PlanLookup {
    PlanLookup::from_settings(Ok(serde_json::from_str(SETTINGS).unwrap()))
}

fn map(text: &str, plan: &PlanLookup) -> Result<LimitsSnapshot, ProviderError> {
    map_limits(&billing(text), plan, identity(), now())
}

#[test]
fn the_weekly_pool_becomes_a_weekly_window() {
    let snapshot = map(WEEKLY, &super_grok()).unwrap();
    assert_eq!(
        snapshot.windows,
        [QuotaWindow {
            id: WindowId::Weekly,
            label: "Weekly".into(),
            used: Percent::new(37.5),
            resets_at: Some("2026-09-25T21:36:52.140114Z".parse().unwrap()),
            period: Some(SignedDuration::from_hours(7 * 24)),
        }]
    );
    assert_eq!(snapshot.identity.plan.as_deref(), Some("SuperGrok"));
    assert_eq!(snapshot.identity.email.as_deref(), Some("ada@example.com"));
    assert!(snapshot.notices.is_empty());
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert_eq!(snapshot.fetched_at, now());
}

#[test]
fn an_omitted_percent_is_zero_and_a_cap_turns_extra_usage_on() {
    let snapshot = map(EXTRA_USAGE, &super_grok()).unwrap();
    assert_eq!(snapshot.windows[0].used, Percent::ZERO);
    assert_eq!(
        snapshot.notices,
        [notice(Tone::Neutral, "Extra usage on, cap $25.00")]
    );
}

#[test]
fn caps_are_cents_shown_exactly_as_dollars() {
    for (raw, shown) in [
        ("1", "$0.01"),
        ("12.5", "$0.125"),
        ("99", "$0.99"),
        ("123456789", "$1,234,567.89"),
        ("100000", "$1,000.00"),
        ("1.5e30", "1.5e+30 cents"),
    ] {
        let text = EXTRA_USAGE.replace("\"val\":2500", &format!("\"val\":{raw}"));
        let snapshot = map(&text, &super_grok()).unwrap();
        assert_eq!(
            snapshot.notices[0].text,
            format!("Extra usage on, cap {shown}")
        );
    }
}

#[test]
fn a_zero_or_missing_cap_emits_no_notice() {
    for text in [
        EXTRA_USAGE.replace("\"val\":2500", "\"val\":0"),
        EXTRA_USAGE.replace("\"onDemandCap\":{\"val\":2500},", ""),
    ] {
        assert!(map(&text, &super_grok()).unwrap().notices.is_empty());
    }
}

#[test]
fn legacy_billing_has_no_window_and_says_so() {
    for plan in [super_grok(), PlanLookup::Failed] {
        let snapshot = map(LEGACY, &plan).unwrap();
        assert!(snapshot.windows.is_empty());
        assert_eq!(snapshot.notices, [notice(Tone::Neutral, LEGACY_BILLING)]);
    }
}

#[test]
fn no_weekly_pool_and_no_plan_is_no_subscription() {
    let missing = PlanLookup::from_settings(Ok(RawSettings {
        subscription_tier_display: Some("  ".into()),
    }));
    assert_eq!(missing, PlanLookup::Missing);
    assert_eq!(map(LEGACY, &missing), Err(no_subscription()));
    assert_eq!(map(r#"{"config":{}}"#, &missing), Err(no_subscription()));
    assert!(map(WEEKLY, &missing).is_ok());
}

#[test]
fn a_weekly_period_without_valid_bounds_is_invalid() {
    let backwards = WEEKLY.replace("2026-09-25T21", "2026-09-10T21");
    let unparsable = WEEKLY.replace("2026-09-18T21:36:52.140114+00:00", "yesterday");
    for text in [backwards, unparsable] {
        assert!(matches!(
            map(&text, &super_grok()),
            Err(ProviderError::InvalidResponse(_))
        ));
    }
}

#[test]
fn a_missing_config_is_a_parse_error() {
    assert!(serde_json::from_str::<RawBilling>("{}").is_err());
}

#[test]
fn a_failed_settings_call_is_not_a_missing_plan() {
    let failed = PlanLookup::from_settings(Err(ProviderError::Network("down".into())));
    assert_eq!(failed, PlanLookup::Failed);
    assert_eq!(map(WEEKLY, &failed).unwrap().identity.plan, None);
}
