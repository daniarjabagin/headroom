use serde_json::json;

use super::super::auth::parse_credentials;
use super::super::client::parse_usage;
use super::super::mapper::map_usage;
use super::*;

const FREE_CREDENTIALS: &str = include_str!("fixtures/credentials_free.json");
const NO_LIMITS: &str = include_str!("fixtures/usage_no_limits.json");
const FULL: &str = include_str!("fixtures/usage_full.json");

fn credentials(subscription: &serde_json::Value) -> Credentials {
    let text = json!({ "claudeAiOauth": {
        "accessToken": "fake-access-token",
        "subscriptionType": subscription,
    }});
    parse_credentials(&text.to_string()).unwrap()
}

fn mapped(body: &str) -> MappedUsage {
    map_usage(&parse_usage(body.as_bytes()).unwrap())
}

fn lapsed(detail: &str) -> Result<MappedUsage, ProviderError> {
    Err(ProviderError::NoSubscription {
        detail: detail.into(),
    })
}

#[test]
fn unsubscribed_account_without_limits_has_no_subscription() {
    let free = parse_credentials(FREE_CREDENTIALS).unwrap();
    assert!(!free.subscribed);
    assert_eq!(
        require_subscription(&free, Ok(mapped(NO_LIMITS))),
        lapsed("No active Claude subscription.")
    );
    assert_eq!(
        require_subscription(&credentials(&json!("free")), Ok(mapped(NO_LIMITS))),
        lapsed("No active Claude subscription (Free plan).")
    );
}

#[test]
fn unsubscribed_account_rejected_by_the_api_has_no_subscription() {
    let free = parse_credentials(FREE_CREDENTIALS).unwrap();
    assert_eq!(
        require_subscription(&free, Err(ProviderError::SignInExpired)),
        lapsed("No active Claude subscription.")
    );
    assert_eq!(
        require_subscription(&free, Err(ProviderError::Network("down".into()))),
        Err(ProviderError::Network("down".into()))
    );
}

#[test]
fn unsubscribed_account_that_still_reports_limits_keeps_them() {
    let free = parse_credentials(FREE_CREDENTIALS).unwrap();
    let kept = require_subscription(&free, Ok(mapped(FULL))).unwrap();
    assert!(!kept.windows.is_empty());
}

#[test]
fn subscribed_account_passes_results_through() {
    for kind in ["pro", "max", "team", "enterprise", "unknown_future_plan"] {
        let paid = credentials(&json!(kind));
        assert!(paid.subscribed, "{kind}");
        assert_eq!(
            require_subscription(&paid, Err(ProviderError::SignInExpired)),
            Err(ProviderError::SignInExpired)
        );
        let empty = require_subscription(&paid, Ok(mapped(NO_LIMITS))).unwrap();
        assert!(empty.windows.is_empty());
    }
}

#[test]
fn missing_or_blank_subscription_is_unsubscribed() {
    let text = json!({ "claudeAiOauth": { "accessToken": "fake-access-token" } });
    assert!(!parse_credentials(&text.to_string()).unwrap().subscribed);
    assert!(!credentials(&json!("  ")).subscribed);
    assert!(!credentials(&json!("FREE")).subscribed);
}
