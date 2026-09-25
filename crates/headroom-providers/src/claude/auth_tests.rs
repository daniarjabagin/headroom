use serde_json::json;

use super::*;

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn credentials_text(oauth: &serde_json::Value) -> String {
    json!({ "claudeAiOauth": oauth, "mcpOAuth": {} }).to_string()
}

fn oauth(expires_at: i64) -> serde_json::Value {
    json!({
        "accessToken": "fake-access-token",
        "refreshToken": "fake-refresh-token",
        "expiresAt": expires_at,
        "subscriptionType": "max",
        "rateLimitTier": "default_claude_max_5x",
        "scopes": ["user:inference", "user:profile", "user:sessions:claude_code"]
    })
}

fn future_millis() -> i64 {
    now().as_millisecond() + 3_600_000
}

#[test]
fn valid_credentials_yield_token_and_plan() {
    let credentials = parse_credentials(&credentials_text(&oauth(future_millis()))).unwrap();
    assert_eq!(credentials.plan.as_deref(), Some("Max 5x"));
    let token = credentials.usable_token(now()).unwrap();
    assert_eq!(token.secret(), "fake-access-token");
}

#[test]
fn expired_token_is_sign_in_expired() {
    let credentials = parse_credentials(&credentials_text(&oauth(now().as_millisecond()))).unwrap();
    assert_eq!(
        credentials.usable_token(now()).unwrap_err(),
        ProviderError::SignInExpired
    );
}

#[test]
fn expiry_is_told_apart_from_a_missing_scope() {
    let at_now = parse_credentials(&credentials_text(&oauth(now().as_millisecond()))).unwrap();
    assert!(at_now.is_expired(now()));
    let mut narrow = oauth(future_millis());
    narrow["scopes"] = json!(["user:inference"]);
    let narrow = parse_credentials(&credentials_text(&narrow)).unwrap();
    assert!(!narrow.is_expired(now()));
    assert!(!narrow.has_profile_scope());
}

#[test]
fn float_expiry_is_accepted() {
    let mut raw = oauth(0);
    raw["expiresAt"] = json!(1_000.0);
    let credentials = parse_credentials(&credentials_text(&raw)).unwrap();
    assert_eq!(
        credentials.usable_token(now()).unwrap_err(),
        ProviderError::SignInExpired
    );
}

#[test]
fn missing_expiry_is_tried() {
    let mut raw = oauth(0);
    raw.as_object_mut().unwrap().remove("expiresAt");
    let credentials = parse_credentials(&credentials_text(&raw)).unwrap();
    assert!(credentials.usable_token(now()).is_ok());
}

#[test]
fn inference_only_scopes_cannot_read_limits() {
    let mut raw = oauth(future_millis());
    raw["scopes"] = json!(["user:inference"]);
    let credentials = parse_credentials(&credentials_text(&raw)).unwrap();
    assert_eq!(
        credentials.usable_token(now()).unwrap_err(),
        ProviderError::SignInExpired
    );
}

#[test]
fn missing_or_empty_token_is_not_signed_in() {
    assert_eq!(
        parse_credentials("{}").unwrap_err(),
        ProviderError::NotSignedIn
    );
    let mut raw = oauth(future_millis());
    raw["accessToken"] = json!("");
    assert_eq!(
        parse_credentials(&credentials_text(&raw)).unwrap_err(),
        ProviderError::NotSignedIn
    );
}

#[test]
fn parse_errors_never_echo_contents() {
    let error =
        parse_credentials("{\"claudeAiOauth\":{\"accessToken\":42,\"x\":\"secret\"}}").unwrap_err();
    let ProviderError::LocalData(message) = error else {
        panic!("expected local data error");
    };
    assert!(!message.contains("42"));
    assert!(!message.contains("secret"));
    assert!(message.contains("line 1"));
}

#[test]
fn debug_output_redacts_token() {
    let credentials = parse_credentials(&credentials_text(&oauth(future_millis()))).unwrap();
    assert!(!format!("{credentials:?}").contains("fake-access-token"));
}

#[test]
fn missing_file_is_not_signed_in() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        load_credentials(dir.path()).unwrap_err(),
        ProviderError::NotSignedIn
    );
}

#[test]
fn file_is_loaded_from_config_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(CREDENTIALS_FILE),
        credentials_text(&oauth(future_millis())),
    )
    .unwrap();
    assert!(load_credentials(dir.path()).is_ok());
}

#[test]
fn plan_labels_follow_subscription_and_tier() {
    let cases = [
        ("max", Some("default_claude_max_5x"), Some("Max 5x")),
        ("max", Some("default_claude_max_20x"), Some("Max 20x")),
        ("pro", Some("default_claude_ai"), Some("Pro")),
        ("pro", None, Some("Pro")),
        ("team", Some("default_raven"), Some("Team")),
        ("enterprise", None, Some("Enterprise")),
        ("", None, None),
    ];
    for (subscription, tier, expected) in cases {
        assert_eq!(
            plan_label(subscription, tier).as_deref(),
            expected,
            "{subscription} {tier:?}"
        );
    }
}
