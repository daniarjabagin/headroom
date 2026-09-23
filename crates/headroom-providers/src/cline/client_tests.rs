use jiff::SignedDuration;
use wiremock::matchers::{body_json, header, header_regex, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const ME: &str = include_str!("fixtures/me_org.json");
const PLAN: &str = include_str!("fixtures/plan_active.json");
const PLAN_NONE: &str = include_str!("fixtures/plan_none.json");
const BALANCE: &str = include_str!("fixtures/balance.json");
const REFRESH: &str = include_str!("fixtures/refresh.json");
const ERROR_ENVELOPE: &str = include_str!("fixtures/error_envelope.json");

fn token() -> Secret {
    Secret::new("workos:fake.jwt.token".to_owned())
}

fn client(server: &MockServer) -> ClineClient {
    ClineClient::new(
        crate::http::client().unwrap(),
        &format!("{}/", server.uri()),
    )
}

async fn serve(route: &str, response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn requests_carry_the_workos_bearer_and_unwrap_the_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/users/me"))
        .and(header("authorization", "Bearer workos:fake.jwt.token"))
        .and(header("accept", "application/json"))
        .and(header_regex("user-agent", r"^headroom/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ME))
        .expect(1)
        .mount(&server)
        .await;
    let user = client(&server).me(&token()).await.unwrap();
    assert_eq!(user.id, "usr-0000000000000001");
    let organizations = user.organizations.unwrap();
    assert_eq!(organizations.len(), 2);
    assert!(organizations[0].active);
}

#[tokio::test]
async fn plan_and_balances_parse() {
    let server = serve(
        "/api/v1/users/me/plan",
        ResponseTemplate::new(200).set_body_string(PLAN),
    )
    .await;
    let plan = client(&server).plan(&token()).await.unwrap().unwrap();
    assert_eq!(
        plan.plan.unwrap().display_name.as_deref(),
        Some("ClinePass Pro")
    );
    let server = serve(
        "/api/v1/users/me/plan",
        ResponseTemplate::new(200).set_body_string(PLAN_NONE),
    )
    .await;
    assert_eq!(
        client(&server).plan(&token()).await.unwrap().unwrap().plan,
        None
    );
    let server = serve(
        "/api/v1/users/usr-1%2Fx/balance",
        ResponseTemplate::new(200).set_body_string(BALANCE),
    )
    .await;
    let balance = client(&server).balance(&token(), "usr-1/x").await.unwrap();
    assert_eq!(balance.balance, 12_345_678);
}

#[tokio::test]
async fn a_bare_body_without_an_envelope_is_accepted() {
    let server = serve(
        "/api/v1/organizations/org-1/balance",
        ResponseTemplate::new(200).set_body_string(r#"{"balance":5,"organizationId":"org-1"}"#),
    )
    .await;
    let balance = client(&server)
        .organization_balance(&token(), "org-1")
        .await
        .unwrap();
    assert_eq!(balance.balance, 5);
}

#[tokio::test]
async fn failures_map_to_provider_errors() {
    let cases = [
        (ResponseTemplate::new(401), ProviderError::SignInExpired),
        (ResponseTemplate::new(403), ProviderError::SignInExpired),
        (
            ResponseTemplate::new(429).insert_header("retry-after", "120"),
            ProviderError::RateLimited {
                retry_after: Some(SignedDuration::from_secs(120)),
            },
        ),
        (
            ResponseTemplate::new(503),
            ProviderError::Network("Cline API returned HTTP 503".into()),
        ),
        (
            ResponseTemplate::new(404),
            ProviderError::InvalidResponse("Cline API returned HTTP 404".into()),
        ),
        (
            ResponseTemplate::new(200).set_body_string(ERROR_ENVELOPE),
            ProviderError::InvalidResponse("Cline API error: User not found".into()),
        ),
    ];
    for (response, expected) in cases {
        let server = serve("/api/v1/users/me", response).await;
        assert_eq!(client(&server).me(&token()).await.unwrap_err(), expected);
    }
}

#[tokio::test]
async fn malformed_or_incomplete_bodies_are_invalid() {
    for body in [
        "<html>",
        r#"{"success":true,"data":{"balance":1.5}}"#,
        r#"{"success":true}"#,
    ] {
        let server = serve(
            "/api/v1/users/u/balance",
            ResponseTemplate::new(200).set_body_string(body),
        )
        .await;
        let error = client(&server).balance(&token(), "u").await.unwrap_err();
        assert!(
            matches!(error, ProviderError::InvalidResponse(_)),
            "{body}: {error:?}"
        );
    }
}

#[tokio::test]
async fn refresh_posts_the_refresh_token_and_rejections_mean_expired() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/refresh"))
        .and(body_json(serde_json::json!({
            "refreshToken": "fake-refresh",
            "grantType": "refresh_token"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_string(REFRESH))
        .expect(1)
        .mount(&server)
        .await;
    let refresh = Secret::new("fake-refresh".to_owned());
    let tokens = client(&server).refresh(&refresh).await.unwrap();
    assert_eq!(tokens.access_token, "fresh.jwt.token");
    assert_eq!(tokens.refresh_token.as_deref(), Some("rotated-refresh"));
    for status in [400, 401] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status))
            .mount(&server)
            .await;
        let error = client(&server).refresh(&refresh).await.err();
        assert_eq!(error, Some(ProviderError::SignInExpired));
    }
}
