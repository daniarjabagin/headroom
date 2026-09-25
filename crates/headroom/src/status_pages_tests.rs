use jiff::SignedDuration;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const SUMMARY: &str = r#"{"components":[]}"#;

fn client() -> StatusPageClient {
    StatusPageClient::build(false).unwrap()
}

fn url(server: &MockServer) -> String {
    format!("{}/api/v2/summary.json", server.uri())
}

#[tokio::test]
async fn a_page_is_read_with_its_etag_and_a_headroom_user_agent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/summary.json"))
        .and(header("user-agent", AGENT))
        .and(header("accept", JSON))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "W/\"e1\"")
                .set_body_string(SUMMARY),
        )
        .expect(1)
        .mount(&server)
        .await;
    let answer = client().get(&url(&server), None).await;
    assert_eq!(
        answer,
        Ok(Fetched::Modified {
            body: SUMMARY.into(),
            etag: Some("W/\"e1\"".into())
        })
    );
}

#[tokio::test]
async fn the_etag_is_sent_and_not_modified_is_recognised() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("if-none-match", "W/\"e1\""))
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .mount(&server)
        .await;
    let answer = client().get(&url(&server), Some("W/\"e1\"")).await;
    assert_eq!(answer, Ok(Fetched::NotModified));
}

#[tokio::test]
async fn rate_limits_carry_the_wait_and_other_statuses_fail() {
    let server = MockServer::start().await;
    Mock::given(path("/limited"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "600"))
        .mount(&server)
        .await;
    Mock::given(path("/broken"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let limited = client()
        .get(&format!("{}/limited", server.uri()), None)
        .await;
    assert_eq!(
        limited,
        Err(FetchError::RateLimited {
            retry_after: Some(SignedDuration::from_mins(10))
        })
    );
    let broken = client()
        .get(&format!("{}/broken", server.uri()), None)
        .await;
    assert_eq!(
        broken,
        Err(FetchError::Failed(
            "the status page answered 503 Service Unavailable".into()
        ))
    );
}

#[tokio::test]
async fn oversized_answers_are_refused() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("x".repeat(BODY_LIMIT + 1)))
        .mount(&server)
        .await;
    let answer = client().get(&url(&server), None).await;
    assert!(matches!(answer, Err(FetchError::Failed(message)) if message.contains("larger")));
}

#[tokio::test]
async fn the_real_client_refuses_plain_http() {
    let answer = StatusPageClient::new()
        .unwrap()
        .get("http://127.0.0.1:9/api/v2/summary.json", None)
        .await;
    assert!(matches!(answer, Err(FetchError::Failed(_))));
}

#[test]
fn every_followed_provider_has_a_status_link() {
    for provider in headroom_daemon::status::followed_providers() {
        let descriptor = crate::providers::descriptor(provider.as_str()).unwrap();
        assert!(descriptor.links.status.is_some(), "{provider}");
    }
}
