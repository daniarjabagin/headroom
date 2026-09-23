use headroom_core::event::ServiceTier;
use headroom_core::tokens::TokenCounts;
use headroom_core::units::{MicroUsd, Tokens};
use headroom_core::usage::PriceBook;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::price_catalog::PriceCatalog;
use crate::test_support::event;

fn litellm_body(input_per_token: f64) -> Value {
    json!({
        "sample_spec": { "mode": "one of …" },
        "gpt-5.5": {
            "litellm_provider": "openai",
            "mode": "chat",
            "max_input_tokens": 1_050_000,
            "input_cost_per_token": input_per_token,
            "output_cost_per_token": 3e-05
        },
        "vertex_ai/claude-opus-5": { "litellm_provider": "vertex_ai", "mode": "chat", "input_cost_per_token": 5e-06 }
    })
}

fn models_dev_body() -> Value {
    json!({
        "openai": { "id": "openai", "models": { "gpt-5.3-codex-spark": { "cost": { "input": 7, "output": 14 } } } },
        "anthropic": { "models": {} },
        "zai": { "models": { "glm-5": { "cost": { "input": 1, "output": 2 } } } }
    })
}

fn sources(server: &MockServer) -> Sources {
    Sources {
        litellm: format!("{}/litellm.json", server.uri()),
        models_dev: format!("{}/api.json", server.uri()),
    }
}

async fn serve(server: &MockServer, route: &str, template: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path(route))
        .respond_with(template)
        .mount(server)
        .await;
}

fn input_cost(catalog: &PriceCatalog, model: &str) -> Option<MicroUsd> {
    let tokens = TokenCounts {
        input: Tokens(100_000),
        ..TokenCounts::default()
    };
    catalog.cost(&event(model, ServiceTier::Standard, &tokens, 0))
}

#[tokio::test]
async fn ok_response_writes_trimmed_cache_overlaid_on_bundled() {
    let server = MockServer::start().await;
    let ok = ResponseTemplate::new(200).insert_header("etag", "\"v1\"");
    serve(
        &server,
        "/litellm.json",
        ok.set_body_json(litellm_body(7e-06)),
    )
    .await;
    serve(
        &server,
        "/api.json",
        ResponseTemplate::new(200).set_body_json(models_dev_body()),
    )
    .await;
    let dir = tempfile::tempdir().unwrap();

    let outcome = refresh(dir.path(), &reqwest::Client::new(), &sources(&server))
        .await
        .unwrap();

    assert!(matches!(outcome.litellm, FeedStatus::Updated));
    assert!(matches!(outcome.models_dev, FeedStatus::Updated));
    let cached: Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("litellm.json")).unwrap()).unwrap();
    assert_eq!(cached["etag"], "\"v1\"");
    assert!(cached["data"].get("vertex_ai/claude-opus-5").is_none());
    assert!(cached["data"]["gpt-5.5"].get("max_input_tokens").is_none());
    let catalog = PriceCatalog::load(dir.path()).unwrap();
    assert_eq!(input_cost(&catalog, "gpt-5.5"), Some(MicroUsd(700_000)));
    assert_eq!(
        input_cost(&catalog, "gpt-5.3-codex-spark"),
        Some(MicroUsd(700_000))
    );
    assert_eq!(
        input_cost(&catalog, "claude-opus-5"),
        Some(MicroUsd(500_000))
    );
}

#[tokio::test]
async fn not_modified_keeps_cache_and_sends_stored_etag() {
    let server = MockServer::start().await;
    let first = ResponseTemplate::new(200).insert_header("etag", "\"v1\"");
    Mock::given(method("GET"))
        .and(path("/litellm.json"))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .mount(&server)
        .await;
    serve(
        &server,
        "/litellm.json",
        first.set_body_json(litellm_body(7e-06)),
    )
    .await;
    serve(
        &server,
        "/api.json",
        ResponseTemplate::new(200).set_body_json(models_dev_body()),
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();

    refresh(dir.path(), &client, &sources(&server))
        .await
        .unwrap();
    let before = std::fs::read(dir.path().join("litellm.json")).unwrap();
    let outcome = refresh(dir.path(), &client, &sources(&server))
        .await
        .unwrap();

    assert!(matches!(outcome.litellm, FeedStatus::NotModified));
    assert_eq!(
        std::fs::read(dir.path().join("litellm.json")).unwrap(),
        before
    );
}

#[tokio::test]
async fn invalid_json_keeps_previous_catalog() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let client = reqwest::Client::new();
    serve(
        &server,
        "/litellm.json",
        ResponseTemplate::new(200).set_body_json(litellm_body(7e-06)),
    )
    .await;
    serve(
        &server,
        "/api.json",
        ResponseTemplate::new(200).set_body_json(models_dev_body()),
    )
    .await;
    refresh(dir.path(), &client, &sources(&server))
        .await
        .unwrap();

    server.reset().await;
    serve(
        &server,
        "/litellm.json",
        ResponseTemplate::new(200).set_body_string("{\"gpt-5.5\": "),
    )
    .await;
    serve(
        &server,
        "/api.json",
        ResponseTemplate::new(200).set_body_json(json!({ "zai": {} })),
    )
    .await;
    let outcome = refresh(dir.path(), &client, &sources(&server))
        .await
        .unwrap();

    assert!(matches!(
        outcome.litellm,
        FeedStatus::Failed(PricingError::Json { .. })
    ));
    assert!(matches!(
        outcome.models_dev,
        FeedStatus::Failed(PricingError::EmptyCatalog { .. })
    ));
    let catalog = PriceCatalog::load(dir.path()).unwrap();
    assert_eq!(input_cost(&catalog, "gpt-5.5"), Some(MicroUsd(700_000)));
    assert_eq!(
        input_cost(&catalog, "gpt-5.3-codex-spark"),
        Some(MicroUsd(700_000))
    );
}

#[tokio::test]
async fn server_errors_are_reported_per_feed() {
    let server = MockServer::start().await;
    serve(&server, "/litellm.json", ResponseTemplate::new(503)).await;
    serve(
        &server,
        "/api.json",
        ResponseTemplate::new(200).set_body_json(models_dev_body()),
    )
    .await;
    let dir = tempfile::tempdir().unwrap();

    let outcome = refresh(dir.path(), &reqwest::Client::new(), &sources(&server))
        .await
        .unwrap();

    assert!(matches!(
        outcome.litellm,
        FeedStatus::Failed(PricingError::Status { status: 503, .. })
    ));
    assert!(matches!(outcome.models_dev, FeedStatus::Updated));
    assert!(!dir.path().join("litellm.json").exists());
    let catalog = PriceCatalog::load(dir.path()).unwrap();
    assert_eq!(input_cost(&catalog, "gpt-5.5"), Some(MicroUsd(500_000)));
}

#[tokio::test]
async fn creates_missing_cache_directory() {
    let server = MockServer::start().await;
    serve(
        &server,
        "/litellm.json",
        ResponseTemplate::new(200).set_body_json(litellm_body(7e-06)),
    )
    .await;
    serve(
        &server,
        "/api.json",
        ResponseTemplate::new(200).set_body_json(models_dev_body()),
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("cache/pricing");

    refresh(&nested, &reqwest::Client::new(), &sources(&server))
        .await
        .unwrap();

    assert!(nested.join("models_dev.json").exists());
    assert!(!nested.join("models_dev.json.tmp").exists());
}

#[test]
fn default_sources_point_at_upstream() {
    let defaults = Sources::default();
    assert!(
        defaults
            .litellm
            .ends_with("model_prices_and_context_window.json")
    );
    assert_eq!(defaults.models_dev, "https://models.dev/api.json");
}
