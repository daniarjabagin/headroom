use super::super::client::parse_user;
use super::*;

fn user(body: &str) -> RawUser {
    parse_user(body.as_bytes()).unwrap()
}

fn limited(used: u64, limit: u64, refresh: Option<&str>) -> RawUser {
    RawUser {
        request_limit_info: RawRequestLimitInfo {
            is_unlimited: false,
            next_refresh_time: refresh.map(str::to_owned),
            request_limit: limit,
            requests_used_since_last_refresh: used,
        },
        bonus_grants: None,
        workspaces: None,
    }
}

#[test]
fn a_limited_plan_is_a_monthly_window_with_bonus_credits_summed() {
    let mapped = map(&user(include_str!("fixtures/request_limits.json"))).unwrap();
    let window = &mapped.windows[0];
    assert_eq!(window.id, WindowId::Other("monthly".into()));
    assert_eq!(window.label, "Monthly credits");
    assert!((window.used.value() - 412.0 * 100.0 / 1500.0).abs() < 1e-9);
    assert_eq!(
        window.resets_at,
        Some("2026-10-12T17:04:33.512Z".parse().unwrap())
    );
    assert_eq!(window.period, None);
    assert_eq!(
        mapped.balances,
        [Balance {
            id: "bonus_credits".into(),
            label: "Bonus credits".into(),
            amount: BalanceAmount::Count {
                value: 1320,
                unit: "credits".into()
            },
        }]
    );
    assert_eq!(mapped.notices, []);
}

#[test]
fn an_unlimited_plan_has_no_window_and_a_neutral_notice() {
    let mapped = map(&user(include_str!("fixtures/unlimited.json"))).unwrap();
    assert_eq!(mapped.windows, []);
    assert_eq!(mapped.balances, []);
    assert_eq!(mapped.notices, [neutral("Unlimited credits")]);
}

#[test]
fn a_zero_limit_is_reported_instead_of_dividing_by_zero() {
    let mapped = map(&limited(0, 0, None)).unwrap();
    assert_eq!(mapped.windows, []);
    assert_eq!(mapped.notices, [neutral(NO_CREDITS_NOTICE)]);
}

#[test]
fn overuse_and_times_without_an_offset_are_kept() {
    let mapped = map(&limited(1650, 1500, Some("2026-10-01T00:00:00"))).unwrap();
    let window = &mapped.windows[0];
    assert!((window.used.value() - 110.0).abs() < 1e-9);
    assert_eq!(
        window.resets_at,
        Some("2026-10-01T00:00:00Z".parse().unwrap())
    );
    assert_eq!(
        map(&limited(1, 2, None)).unwrap().windows[0].resets_at,
        None
    );
}

#[test]
fn an_unreadable_refresh_time_is_an_error() {
    assert_eq!(
        map(&limited(1, 2, Some("next month"))),
        Err(ProviderError::InvalidResponse(
            "Warp sent an unreadable refresh time \"next month\"".into()
        ))
    );
}

#[test]
fn empty_grant_lists_still_show_a_zero_bonus_balance() {
    let mut raw = limited(1, 2, None);
    raw.bonus_grants = Some(vec![RawGrant {
        request_credits_remaining: 0,
    }]);
    let mapped = map(&raw).unwrap();
    assert_eq!(
        mapped.balances[0].amount,
        BalanceAmount::Count {
            value: 0,
            unit: "credits".into()
        }
    );
}
