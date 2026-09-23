use jiff::SignedDuration;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::minimax::mapper::NO_PLAN;

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

async fn fetch(status: u16, body: &str) -> Result<RawRemains, ProviderError> {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/token_plan/remains"))
        .and(header("authorization", "Bearer mm-test"))
        .respond_with(
            ResponseTemplate::new(status)
                .set_body_string(body)
                .insert_header("retry-after", "45"),
        )
        .mount(&server)
        .await;
    let client = MiniMaxClient::new(Client::new(), &format!("{}/", server.uri()));
    client.remains("mm-test", now()).await
}

#[tokio::test]
async fn remains_are_fetched_with_the_bearer_key() {
    let raw = fetch(200, include_str!("fixtures/remains_ok.json"))
        .await
        .unwrap();
    assert_eq!(raw.model_remains.map(|models| models.len()), Some(2));
    assert_eq!(raw.base_resp.map(|base| base.status_code), Some(0));
}

#[tokio::test]
async fn statuses_map_to_typed_errors() {
    assert_eq!(
        fetch(401, "").await.unwrap_err(),
        ProviderError::SignInExpired
    );
    assert_eq!(
        fetch(403, "").await.unwrap_err(),
        ProviderError::SignInExpired
    );
    assert_eq!(
        fetch(429, "").await.unwrap_err(),
        ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(45))
        }
    );
    assert!(matches!(
        fetch(502, "").await,
        Err(ProviderError::Network(_))
    ));
    assert!(matches!(
        fetch(200, "<html>").await,
        Err(ProviderError::InvalidResponse(_))
    ));
    assert!(matches!(
        fetch(404, "").await,
        Err(ProviderError::InvalidResponse(_))
    ));
}

#[tokio::test]
async fn an_error_status_with_a_base_response_uses_its_code() {
    assert_eq!(
        fetch(400, include_str!("fixtures/no_plan.json"))
            .await
            .unwrap_err(),
        ProviderError::NoSubscription {
            detail: NO_PLAN.into()
        }
    );
}
