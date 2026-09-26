use headroom_core::units::Percent;
use jiff::SignedDuration;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const OK: &str = include_str!("fixtures/usage_ok.json");
const NO_PLAN: &str = include_str!("fixtures/no_subscription.json");
const INVALID_KEY: &str = include_str!("fixtures/invalid_key.json");
const RATE_LIMITED: &str = include_str!("fixtures/rate_limited.json");

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

async fn answer(response: ResponseTemplate) -> Result<RawUsage, ProviderError> {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/zen/go/v1/usage"))
        .and(header("authorization", "Bearer sk-fake-go-key"))
        .and(header("accept", "application/json"))
        .respond_with(response)
        .expect(1)
        .mount(&server)
        .await;
    let client = UsageClient::new(
        crate::http::client().unwrap(),
        &format!("{}/", server.uri()),
    );
    client.fetch("sk-fake-go-key", now()).await
}

#[tokio::test]
async fn a_subscribed_key_returns_three_windows() {
    let raw = answer(ResponseTemplate::new(200).set_body_string(OK))
        .await
        .unwrap();
    assert_eq!(
        raw.usage.rolling.resets_at,
        Some("2026-09-23T13:41:07.512Z".parse().unwrap())
    );
    let percents: Vec<_> = [raw.usage.rolling, raw.usage.weekly, raw.usage.monthly]
        .map(|window| Percent::new(window.percent))
        .into();
    assert_eq!(
        percents,
        [Percent::new(12.0), Percent::new(37.0), Percent::FULL]
    );
}

#[tokio::test]
async fn an_entitlement_error_means_no_subscription() {
    let error = answer(ResponseTemplate::new(403).set_body_string(NO_PLAN))
        .await
        .unwrap_err();
    assert_eq!(
        error,
        ProviderError::NoSubscription {
            detail: "No OpenCode Go subscription on this key.".into()
        }
    );
}

#[tokio::test]
async fn an_unknown_key_is_a_rejected_sign_in() {
    let error = answer(ResponseTemplate::new(401).set_body_string(INVALID_KEY))
        .await
        .unwrap_err();
    assert_eq!(error, ProviderError::SignInExpired);
}

#[tokio::test]
async fn rate_limits_honour_retry_after() {
    let response = ResponseTemplate::new(429)
        .insert_header("retry-after", "120")
        .set_body_string(RATE_LIMITED);
    assert_eq!(
        answer(response).await.unwrap_err(),
        ProviderError::rate_limited(Some(SignedDuration::from_secs(120)))
    );
}

#[test]
fn statuses_map_to_typed_errors() {
    let empty = HeaderMap::new();
    let map = |status: u16, body: &str| {
        status_error(
            StatusCode::from_u16(status).unwrap(),
            &empty,
            body.as_bytes(),
            now(),
        )
    };
    assert_eq!(map(403, INVALID_KEY), ProviderError::SignInExpired);
    assert_eq!(
        map(403, "<html>forbidden</html>"),
        ProviderError::SignInExpired
    );
    assert_eq!(map(429, RATE_LIMITED), ProviderError::rate_limited(None));
    assert_eq!(
        map(502, ""),
        ProviderError::Network("OpenCode usage endpoint returned HTTP 502".into())
    );
    assert_eq!(
        map(404, ""),
        ProviderError::InvalidResponse("OpenCode usage endpoint returned HTTP 404".into())
    );
}

#[test]
fn missing_windows_or_percent_are_invalid() {
    let bodies = [
        "",
        "{}",
        r#"{"usage":{"rolling":{"percent":1},"weekly":{"percent":1}}}"#,
        r#"{"usage":{"rolling":{},"weekly":{"percent":1},"monthly":{"percent":1}}}"#,
        r#"{"usage":{"rolling":{"percent":1,"resetsAt":"later"},"weekly":{"percent":1},"monthly":{"percent":1}}}"#,
    ];
    for body in bodies {
        assert!(
            matches!(
                parse_usage(body.as_bytes()),
                Err(ProviderError::InvalidResponse(_))
            ),
            "{body}"
        );
    }
    let minimal =
        r#"{"usage":{"rolling":{"percent":0},"weekly":{"percent":0},"monthly":{"percent":0}}}"#;
    assert_eq!(
        parse_usage(minimal.as_bytes())
            .unwrap()
            .usage
            .weekly
            .resets_at,
        None
    );
}
