use std::fs;

use headroom_core::account::AccountId;
use headroom_core::quota::{LimitsSource, WindowId};
use serde_json::json;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::test_support::{
    ACCOUNT_ID, USER_ID, access_token, at, auth_document, write_auth, write_rollout,
};
use super::*;

const FULL: &str = include_str!("fixtures/usage_full.json");
const RATE_LIMITS: &str = include_str!("fixtures/rollout_rate_limits.jsonl");
const RECORDS: &str = include_str!("fixtures/rollout_records.jsonl");
const NOW: &str = "2026-09-23T10:00:00Z";

struct Setup {
    root: TempDir,
}

impl Setup {
    fn new() -> Setup {
        Setup {
            root: tempfile::tempdir().unwrap(),
        }
    }

    fn cli_home(&self) -> PathBuf {
        self.root.path().join("home/.codex")
    }

    fn headroom_home(&self, name: &str) -> PathBuf {
        self.root
            .path()
            .join("data/headroom/accounts/codex")
            .join(name)
    }

    fn provider(&self, api_base: &str) -> CodexProvider {
        CodexProvider::new(CodexConfig {
            environment: CodexEnvironment {
                codex_home: None,
                home_dir: Some(self.root.path().join("home")),
                data_dir: Some(self.root.path().join("data")),
            },
            api_base: api_base.to_owned(),
            clock: Arc::new(|| at(NOW)),
        })
        .unwrap()
    }

    fn sign_in(&self, expires_at: &str) {
        write_auth(
            &self.cli_home(),
            &auth_document(&access_token(at(expires_at))),
        );
    }

    fn account(&self) -> AccountRef {
        AccountRef {
            id: AccountId::from_stable_key(ProviderKind::Codex, &format!("{USER_ID}/{ACCOUNT_ID}")),
            provider: ProviderKind::Codex,
            home: self.cli_home(),
            owner: CredentialOwner::Cli,
        }
    }
}

async fn server_with(response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/backend-api/wham/usage"))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn discovers_cli_and_headroom_homes() {
    let setup = Setup::new();
    setup.sign_in("2026-09-24T00:00:00Z");
    let other = json!({ "tokens": { "access_token": "a", "account_id": "acct-other" } });
    write_auth(&setup.headroom_home("b"), &other);
    write_auth(&setup.headroom_home("c"), &auth_document("dup"));
    fs::create_dir_all(setup.headroom_home("a-empty")).unwrap();
    let accounts = setup.provider(DEFAULT_API_BASE).discover().await.unwrap();
    let summary: Vec<_> = accounts
        .iter()
        .map(|account| (account.home.clone(), account.owner))
        .collect();
    assert_eq!(
        summary,
        [
            (setup.cli_home(), CredentialOwner::Cli),
            (setup.headroom_home("b"), CredentialOwner::Headroom),
        ]
    );
    assert_eq!(accounts[0], setup.account());
}

#[tokio::test]
async fn discovery_reports_api_key_only_and_signed_out() {
    let setup = Setup::new();
    let provider = setup.provider(DEFAULT_API_BASE);
    assert_eq!(provider.discover().await, Err(ProviderError::NotSignedIn));
    write_auth(&setup.cli_home(), &json!({ "OPENAI_API_KEY": "sk-fake" }));
    assert_eq!(provider.discover().await, Err(ProviderError::ApiKeyOnly));
}

#[tokio::test]
async fn codex_home_override_is_used() {
    let setup = Setup::new();
    let custom = setup.root.path().join("custom");
    write_auth(&custom, &auth_document("opaque"));
    let mut provider = setup.provider(DEFAULT_API_BASE);
    provider.config.environment.codex_home = Some(custom.display().to_string());
    let accounts = provider.discover().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].home, custom);
}

#[tokio::test]
async fn live_limits_come_from_the_usage_api() {
    let setup = Setup::new();
    setup.sign_in("2026-09-24T00:00:00Z");
    let server = server_with(
        ResponseTemplate::new(200).set_body_raw(FULL.as_bytes().to_vec(), "application/json"),
    )
    .await;
    let snapshot = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await
        .unwrap();
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert_eq!(snapshot.windows.len(), 4);
    assert_eq!(
        snapshot.identity.email.as_deref(),
        Some("someone@example.com")
    );
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Plus"));
}

#[tokio::test]
async fn expired_sign_in_falls_back_to_logs_without_calling_the_api() {
    let setup = Setup::new();
    setup.sign_in("2026-09-23T09:00:00Z");
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let snapshot = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await
        .unwrap();
    assert!(matches!(snapshot.source, LimitsSource::LocalLog { .. }));
    assert_eq!(snapshot.windows[0].id, WindowId::Weekly);
}

#[tokio::test]
async fn network_and_rate_limit_failures_fall_back_to_logs() {
    let setup = Setup::new();
    setup.sign_in("2026-09-24T00:00:00Z");
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    for status in [500, 429, 401] {
        let server = server_with(ResponseTemplate::new(status)).await;
        let snapshot = setup
            .provider(&server.uri())
            .fetch_limits(&setup.account())
            .await
            .unwrap();
        assert!(matches!(snapshot.source, LimitsSource::LocalLog { .. }));
    }
}

#[tokio::test]
async fn without_logs_the_original_error_is_returned() {
    let setup = Setup::new();
    setup.sign_in("2026-09-24T00:00:00Z");
    let server = server_with(ResponseTemplate::new(429).insert_header("retry-after", "60")).await;
    let result = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await;
    assert_eq!(
        result,
        Err(ProviderError::RateLimited {
            retry_after: Some(jiff::SignedDuration::from_secs(60))
        })
    );
}

#[tokio::test]
async fn invalid_response_does_not_fall_back() {
    let setup = Setup::new();
    setup.sign_in("2026-09-24T00:00:00Z");
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    let server = server_with(ResponseTemplate::new(200).set_body_string("nope")).await;
    let result = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await;
    assert!(matches!(result, Err(ProviderError::InvalidResponse(_))));
}

#[tokio::test]
async fn signed_out_account_gets_no_offline_data() {
    let setup = Setup::new();
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RATE_LIMITS);
    let result = setup
        .provider(DEFAULT_API_BASE)
        .fetch_limits(&setup.account())
        .await;
    assert_eq!(result, Err(ProviderError::NotSignedIn));
}

#[tokio::test]
async fn credentials_file_is_never_modified() {
    let setup = Setup::new();
    setup.sign_in("2026-09-23T09:00:00Z");
    let before = fs::read(setup.cli_home().join("auth.json")).unwrap();
    let server = server_with(ResponseTemplate::new(401)).await;
    let _ = setup
        .provider(&server.uri())
        .fetch_limits(&setup.account())
        .await;
    assert_eq!(
        fs::read(setup.cli_home().join("auth.json")).unwrap(),
        before
    );
}

#[test]
fn read_usage_uses_the_injected_clock() {
    let setup = Setup::new();
    write_rollout(&setup.cli_home(), "sessions/rollout.jsonl", RECORDS);
    let mut cursors = LogCursors::default();
    let events = setup
        .provider(DEFAULT_API_BASE)
        .read_usage(&setup.cli_home(), &mut cursors)
        .unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(setup.provider(DEFAULT_API_BASE).kind(), ProviderKind::Codex);
}
