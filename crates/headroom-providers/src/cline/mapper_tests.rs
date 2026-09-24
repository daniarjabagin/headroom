use super::super::client::parse_envelope;
use super::super::raw::RawUser;
use super::*;

const PLAN: &str = include_str!("fixtures/plan_active.json");
const PLAN_CANCELED: &str = include_str!("fixtures/plan_canceled.json");
const PLAN_NONE: &str = include_str!("fixtures/plan_none.json");
const BALANCE: &str = include_str!("fixtures/balance.json");
const BALANCE_ZERO: &str = include_str!("fixtures/balance_zero.json");
const ORG_BALANCE: &str = include_str!("fixtures/org_balance.json");
const ME_ORG: &str = include_str!("fixtures/me_org.json");

fn fetched(plan: &str, balance: &str) -> Fetched {
    Fetched {
        plan: parse_envelope(plan.as_bytes()).unwrap(),
        balance: parse_envelope(balance.as_bytes()).unwrap(),
        organization: None,
    }
}

fn usd(balance: &Balance) -> i64 {
    match balance.amount {
        BalanceAmount::Usd(MicroUsd(value)) => value,
        BalanceAmount::Count { .. } | BalanceAmount::Money(_) => panic!("{balance:?}"),
    }
}

#[test]
fn a_plan_shows_its_name_period_and_credits_in_micro_usd() {
    let mapped = map_account(&fetched(PLAN, BALANCE)).unwrap();
    assert_eq!(mapped.plan, "ClinePass Pro");
    assert_eq!(mapped.balances.len(), 1);
    assert_eq!(mapped.balances[0].id, "credits");
    assert_eq!(mapped.balances[0].label, "Credits");
    assert_eq!(usd(&mapped.balances[0]), 12_345_678);
    assert_eq!(
        mapped.notices,
        [Notice {
            tone: Tone::Neutral,
            text: "ClinePass Pro renews on 2026-10-01 (UTC).".into()
        }]
    );
}

#[test]
fn a_cancelled_plan_says_when_it_ends_and_falls_back_to_the_plan_name() {
    let mapped = map_account(&fetched(PLAN_CANCELED, BALANCE_ZERO)).unwrap();
    assert_eq!(mapped.plan, "clinepass_pro");
    assert_eq!(
        mapped.notices,
        [
            Notice {
                tone: Tone::Neutral,
                text: "clinepass_pro ends on 2026-10-01 (UTC).".into()
            },
            Notice {
                tone: Tone::Critical,
                text: "No Cline credits left.".into()
            }
        ]
    );
    assert_eq!(usd(&mapped.balances[0]), 0);
}

#[test]
fn without_a_plan_credits_mean_pay_as_you_go() {
    let mapped = map_account(&fetched(PLAN_NONE, BALANCE)).unwrap();
    assert_eq!(mapped.plan, "Pay as you go");
    assert!(mapped.notices.is_empty());
    let missing = Fetched {
        plan: None,
        ..fetched(PLAN_NONE, BALANCE)
    };
    assert_eq!(map_account(&missing).unwrap().plan, "Pay as you go");
}

#[test]
fn no_plan_and_no_credits_is_no_subscription() {
    for balance in [BALANCE_ZERO, r#"{"success":true,"data":{"balance":-40}}"#] {
        assert_eq!(
            map_account(&fetched(PLAN_NONE, balance)).unwrap_err(),
            ProviderError::NoSubscription {
                detail: "No Cline plan and no credits left.".into()
            }
        );
    }
}

#[test]
fn an_active_organization_shows_its_shared_balance_first() {
    let user: RawUser = parse_envelope(ME_ORG.as_bytes()).unwrap();
    let organization = user.organizations.unwrap().remove(0);
    let org_balance = parse_envelope(ORG_BALANCE.as_bytes()).unwrap();
    let with_org = Fetched {
        organization: Some((organization, org_balance)),
        ..fetched(PLAN_NONE, BALANCE_ZERO)
    };
    let mapped = map_account(&with_org).unwrap();
    assert_eq!(mapped.plan, "Pay as you go");
    let labels: Vec<(&str, &str, i64)> = mapped
        .balances
        .iter()
        .map(|balance| (balance.id.as_str(), balance.label.as_str(), usd(balance)))
        .collect();
    assert_eq!(
        labels,
        [
            ("organization_credits", "Example Team credits", 250_000_000),
            ("credits", "Credits", 0)
        ]
    );
}
