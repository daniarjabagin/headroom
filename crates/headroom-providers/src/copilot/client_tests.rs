use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const PRO: &str = include_str!("fixtures/user_pro.json");

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn token() -> SecretString {
    SecretString::new("gho_fake_token".into())
}

async fn answering(response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(USER_PATH))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

async fn fetch(server: &MockServer) -> Result<RawUser, ProviderError> {
    UserClient::new(crate::http::client().unwrap(), &server.uri())
        .fetch(&token(), now())
        .await
}

#[tokio::test]
async fn the_request_carries_the_token_and_the_editor_headers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(USER_PATH))
        .and(header("authorization", "token gho_fake_token"))
        .and(header("accept", "application/json"))
        .and(header("user-agent", "GitHubCopilotChat/0.26.7"))
        .and(header("editor-version", "vscode/1.96.2"))
        .and(header("editor-plugin-version", "copilot-chat/0.26.7"))
        .and(header("x-github-api-version", "2025-04-01"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PRO))
        .expect(1)
        .mount(&server)
        .await;
    let raw = fetch(&server).await.unwrap();
    assert_eq!(raw.copilot_plan.as_deref(), Some("individual_pro"));
}

#[tokio::test]
async fn a_rejected_token_is_an_expired_sign_in() {
    for status in [401, 403] {
        let server = answering(ResponseTemplate::new(status).set_body_string(
            r#"{"message":"Bad credentials","documentation_url":"https://docs.github.com/rest"}"#,
        ))
        .await;
        assert_eq!(
            fetch(&server).await.unwrap_err(),
            ProviderError::SignInExpired
        );
    }
}

#[tokio::test]
async fn rate_limits_honour_retry_after_and_the_primary_limit_reset() {
    let limited = answering(ResponseTemplate::new(429).insert_header("retry-after", "60")).await;
    assert_eq!(
        fetch(&limited).await.unwrap_err(),
        ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(60))
        }
    );
    let reset = now().as_second() + 300;
    let primary = answering(
        ResponseTemplate::new(403)
            .insert_header("x-ratelimit-remaining", "0")
            .insert_header("x-ratelimit-reset", reset.to_string().as_str()),
    )
    .await;
    assert_eq!(
        fetch(&primary).await.unwrap_err(),
        ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_mins(5))
        }
    );
}

#[tokio::test]
async fn missing_copilot_server_errors_and_garbage_are_told_apart() {
    let missing = answering(ResponseTemplate::new(404).set_body_string("{}")).await;
    assert!(matches!(
        fetch(&missing).await,
        Err(ProviderError::NoSubscription { .. })
    ));
    let down = answering(ResponseTemplate::new(502)).await;
    assert!(matches!(fetch(&down).await, Err(ProviderError::Network(_))));
    let odd = answering(ResponseTemplate::new(400)).await;
    assert!(matches!(
        fetch(&odd).await,
        Err(ProviderError::InvalidResponse(_))
    ));
    let garbage = answering(ResponseTemplate::new(200).set_body_string("<html>")).await;
    assert!(matches!(
        fetch(&garbage).await,
        Err(ProviderError::InvalidResponse(_))
    ));
}
