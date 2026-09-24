use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const LIMITS: &str = include_str!("fixtures/request_limits.json");
const USER_FACING_ERROR: &str = include_str!("fixtures/user_facing_error.json");
const GRAPHQL_ERRORS: &str = include_str!("fixtures/graphql_errors.json");
const KEY: &str = "wk-test-key";

fn now() -> Timestamp {
    "2026-09-24T10:00:00Z".parse().unwrap()
}

async fn serving(response: ResponseTemplate) -> (MockServer, WarpClient) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql/v2"))
        .and(query_param("op", "GetRequestLimitInfo"))
        .and(header("authorization", format!("Bearer {KEY}")))
        .and(header("user-agent", "Warp/1.0"))
        .and(header("x-warp-client-id", "warp-app"))
        .and(header("x-warp-os-category", os_name()))
        .and(header("content-type", "application/json"))
        .and(body_partial_json(serde_json::json!({
            "operationName": "GetRequestLimitInfo",
            "variables": {"requestContext": {"clientContext": {}}},
        })))
        .respond_with(response)
        .mount(&server)
        .await;
    let client = WarpClient::new(
        crate::http::client().unwrap(),
        &format!("{}/", server.uri()),
    );
    (server, client)
}

fn json(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_string(body)
}

#[tokio::test]
async fn the_query_is_posted_as_the_warp_client_and_parses() {
    let (_server, client) = serving(json(200, LIMITS)).await;
    let user = client.request_limits(KEY, now()).await.unwrap();
    assert_eq!(user.request_limit_info.request_limit, 1500);
    assert_eq!(
        user.request_limit_info.requests_used_since_last_refresh,
        412
    );
}

#[test]
fn the_body_asks_for_limits_and_both_kinds_of_bonus_grants() {
    let body = request_body();
    let query = body["query"].as_str().unwrap();
    for field in [
        "requestLimitInfo",
        "requestsUsedSinceLastRefresh",
        "bonusGrants",
        "bonusGrantsInfo",
        "... on UserFacingError",
    ] {
        assert!(query.contains(field), "{field}");
    }
    assert_eq!(
        body["variables"]["requestContext"]["osContext"]["name"],
        os_name()
    );
}

#[tokio::test]
async fn user_facing_and_graphql_errors_are_shown_as_invalid_responses() {
    for (body, message) in [
        (USER_FACING_ERROR, "Warp answered: User not found"),
        (GRAPHQL_ERRORS, "Warp answered: Unauthenticated"),
        (r#"{"data":null}"#, "the Warp usage response has no data"),
        (
            r#"{"data":{"user":{"__typename":"Other"}}}"#,
            "the Warp usage response has an unknown user type",
        ),
    ] {
        let (_server, client) = serving(json(200, body)).await;
        assert_eq!(
            client.request_limits(KEY, now()).await.unwrap_err(),
            ProviderError::InvalidResponse(message.into())
        );
    }
}

#[test]
fn long_server_messages_are_cut() {
    let body = format!(
        r#"{{"data":{{"user":{{"__typename":"UserFacingError","error":{{"message":"{}"}}}}}}}}"#,
        "x".repeat(500)
    );
    let ProviderError::InvalidResponse(message) = parse_user(body.as_bytes()).unwrap_err() else {
        panic!("expected an invalid response");
    };
    assert_eq!(message.len(), "Warp answered: ".len() + 200);
}

#[tokio::test]
async fn http_failures_map_to_provider_errors() {
    let (_server, client) = serving(json(401, "")).await;
    assert_eq!(
        client.request_limits(KEY, now()).await,
        Err(ProviderError::SignInExpired)
    );
    let (_server, client) =
        serving(json(429, "Rate exceeded.").insert_header("retry-after", "60")).await;
    assert_eq!(
        client.request_limits(KEY, now()).await,
        Err(ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(60))
        })
    );
    let (_server, client) = serving(json(503, "")).await;
    assert!(matches!(
        client.request_limits(KEY, now()).await,
        Err(ProviderError::Network(_))
    ));
    let (_server, client) = serving(json(200, "<html>")).await;
    assert!(matches!(
        client.request_limits(KEY, now()).await,
        Err(ProviderError::InvalidResponse(_))
    ));
}
