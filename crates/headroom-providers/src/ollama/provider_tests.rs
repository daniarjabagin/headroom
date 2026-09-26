use std::fs;

use aws_lc_rs::signature::{ED25519, UnparsedPublicKey};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use headroom_core::quota::WindowId;
use tempfile::TempDir;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

use super::*;

const TEST_KEY: &str = include_str!("fixtures/test_id_ed25519");
const USAGE: &str = include_str!("fixtures/usage.json");
const ME: &str = include_str!("fixtures/me.json");
const TS: &str = "1790157600";

fn fixed_now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

struct Signed;

impl Match for Signed {
    fn matches(&self, request: &Request) -> bool {
        let Some(header) = request.headers.get("authorization") else {
            return false;
        };
        let Some((blob, signature)) = header.to_str().unwrap_or("").split_once(':') else {
            return false;
        };
        let key = SigningKey::parse(TEST_KEY).unwrap();
        let challenge = format!(
            "{},{}?{}",
            request.method,
            request.url.path(),
            request.url.query().unwrap_or("")
        );
        let signature = STANDARD.decode(signature).unwrap_or_default();
        blob == key.public_blob()
            && UnparsedPublicKey::new(&ED25519, key.public_raw())
                .verify(challenge.as_bytes(), &signature)
                .is_ok()
    }
}

struct Setup {
    root: TempDir,
    server: MockServer,
}

impl Setup {
    async fn new() -> Setup {
        Setup {
            root: tempfile::tempdir().unwrap(),
            server: MockServer::start().await,
        }
    }

    fn keys(&self) -> KeyPaths {
        KeyPaths {
            user: self.root.path().join("home/.ollama/id_ed25519"),
            system: self.root.path().join("system/.ollama/id_ed25519"),
        }
    }

    fn provider(&self) -> OllamaProvider {
        let config = OllamaConfig {
            keys: self.keys(),
            api_base: self.server.uri(),
        };
        OllamaProvider::with_clock(config, reqwest::Client::new(), fixed_now)
    }

    fn write_key(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    async fn answer(&self, verb: &str, route: &str, status: u16, body: &str) {
        Mock::given(method(verb))
            .and(path(route))
            .and(query_param("ts", TS))
            .and(Signed)
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .mount(&self.server)
            .await;
    }

    async fn account(&self) -> AccountRef {
        self.provider().discover().await.unwrap().remove(0)
    }
}

#[tokio::test]
async fn without_any_key_ollama_is_not_detected() {
    let setup = Setup::new().await;
    assert_eq!(
        setup.provider().discover().await,
        Err(ProviderError::NotSignedIn)
    );
}

#[tokio::test]
async fn the_user_key_is_one_cli_owned_account() {
    let setup = Setup::new().await;
    let keys = setup.keys();
    Setup::write_key(&keys.user, TEST_KEY);
    setup.answer("POST", "/api/me", 200, ME).await;
    let accounts = setup.provider().discover().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].home, keys.user.parent().unwrap());
    assert_eq!(accounts[0].owner, CredentialOwner::Cli);
    assert!(accounts[0].id.0.starts_with("ollama:"));
}

#[tokio::test]
async fn a_key_not_linked_to_ollama_com_is_not_an_account() {
    for status in [401, 403] {
        let setup = Setup::new().await;
        Setup::write_key(&setup.keys().user, TEST_KEY);
        setup.answer("POST", "/api/me", status, "{}").await;
        assert_eq!(setup.provider().discover().await, Ok(Vec::new()));
    }
}

#[tokio::test]
async fn discovery_keeps_the_account_when_the_link_check_fails() {
    for status in [500, 429] {
        let setup = Setup::new().await;
        Setup::write_key(&setup.keys().user, TEST_KEY);
        setup.answer("POST", "/api/me", status, "").await;
        assert_eq!(setup.provider().discover().await.unwrap().len(), 1);
    }
    let setup = Setup::new().await;
    Setup::write_key(&setup.keys().user, TEST_KEY);
    let config = OllamaConfig {
        keys: setup.keys(),
        api_base: "http://127.0.0.1:1".to_owned(),
    };
    let offline = OllamaProvider::with_clock(config, reqwest::Client::new(), fixed_now);
    assert_eq!(offline.discover().await.unwrap().len(), 1);
}

#[tokio::test]
async fn discovery_checks_the_link_once_with_a_signed_request() {
    let setup = Setup::new().await;
    Setup::write_key(&setup.keys().user, TEST_KEY);
    setup.answer("POST", "/api/me", 200, ME).await;
    setup.provider().discover().await.unwrap();
    let requests = setup.server.received_requests().await.unwrap();
    let calls: Vec<_> = requests
        .iter()
        .map(|request| (request.method.to_string(), request.url.path().to_owned()))
        .collect();
    assert_eq!(calls, [("POST".to_owned(), "/api/me".to_owned())]);
}

#[tokio::test]
async fn signed_requests_return_limits_plan_and_email() {
    let setup = Setup::new().await;
    Setup::write_key(&setup.keys().user, TEST_KEY);
    setup.answer("GET", "/api/usage", 200, USAGE).await;
    setup.answer("POST", "/api/me", 200, ME).await;
    let account = setup.account().await;
    let snapshot = setup.provider().fetch_limits(&account).await.unwrap();
    let ids: Vec<_> = snapshot.windows.iter().map(|w| w.id.clone()).collect();
    assert_eq!(
        ids,
        [
            WindowId::Session,
            WindowId::Weekly,
            WindowId::Other("monthly".into())
        ]
    );
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Pro"));
    assert_eq!(
        snapshot.identity.email.as_deref(),
        Some("someone@example.com")
    );
    assert_eq!(snapshot.identity.account_id(&ID), account.id);
    assert!(snapshot.notices.is_empty());
}

#[tokio::test]
async fn a_failed_plan_lookup_keeps_the_meters() {
    let setup = Setup::new().await;
    Setup::write_key(&setup.keys().user, TEST_KEY);
    setup.answer("GET", "/api/usage", 200, USAGE).await;
    setup.answer("POST", "/api/me", 500, "").await;
    let account = setup.account().await;
    let snapshot = setup.provider().fetch_limits(&account).await.unwrap();
    assert_eq!(snapshot.windows.len(), 3);
    assert_eq!(snapshot.identity.plan, None);
    assert_eq!(snapshot.notices.len(), 1);
}

#[tokio::test]
async fn unauthorized_and_rate_limited_answers_map_to_errors() {
    for (status, expected) in [
        (401, ProviderError::SignInExpired),
        (429, ProviderError::rate_limited(None)),
    ] {
        let setup = Setup::new().await;
        Setup::write_key(&setup.keys().user, TEST_KEY);
        setup.answer("GET", "/api/usage", status, "{}").await;
        let account = setup.account().await;
        assert_eq!(setup.provider().fetch_limits(&account).await, Err(expected));
    }
}

#[tokio::test]
async fn a_malformed_usage_body_is_an_invalid_response() {
    let setup = Setup::new().await;
    Setup::write_key(&setup.keys().user, TEST_KEY);
    setup
        .answer("GET", "/api/usage", 200, r#"{"activity":{}}"#)
        .await;
    let account = setup.account().await;
    assert!(matches!(
        setup.provider().fetch_limits(&account).await,
        Err(ProviderError::InvalidResponse(_))
    ));
}

#[tokio::test]
async fn a_broken_key_file_is_a_local_data_error() {
    let setup = Setup::new().await;
    Setup::write_key(&setup.keys().user, "not a key");
    let account = setup.account().await;
    let error = setup.provider().fetch_limits(&account).await.unwrap_err();
    assert!(matches!(error, ProviderError::LocalData(ref text) if text.contains("id_ed25519")));
    assert!(setup.server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn an_unreadable_system_key_explains_itself_without_calling_ollama() {
    let setup = Setup::new().await;
    let keys = setup.keys();
    fs::create_dir_all(&keys.system).unwrap();
    let account = setup.account().await;
    assert_eq!(account.home, keys.system.parent().unwrap());
    let snapshot = setup.provider().fetch_limits(&account).await.unwrap();
    assert!(snapshot.windows.is_empty());
    assert_eq!(snapshot.notices.len(), 1);
    assert!(snapshot.notices[0].text.contains("system service"));
    assert!(setup.server.received_requests().await.unwrap().is_empty());
}
