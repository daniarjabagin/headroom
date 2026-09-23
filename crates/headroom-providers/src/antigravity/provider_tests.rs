use std::fs;

use headroom_core::quota::WindowId;
use headroom_core::secret::SecretString;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::antigravity::test_support::fake_process;

const LS_SUMMARY: &str = include_str!("fixtures/quota_summary_ls.json");
const CLOUD_SUMMARY: &str = include_str!("fixtures/quota_summary_cloud.json");
const USER_STATUS: &str = include_str!("fixtures/user_status.json");
const CODE_ASSIST: &str = include_str!("fixtures/load_code_assist.json");
const LS_PREFIX: &str = "/exa.language_server_pb.LanguageServerService";

fn fixed_now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
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

    fn proc_root(&self) -> PathBuf {
        self.root.path().join("proc")
    }

    fn provider(&self) -> AntigravityProvider {
        let config = AntigravityConfig {
            gemini_dir: self.root.path().join("home/.gemini"),
            proc_root: self.proc_root(),
            cloud_bases: vec![self.server.uri()],
            secret_bus: SecretBus::Disabled,
        };
        AntigravityProvider::with_clock(config, reqwest::Client::new(), fixed_now).unwrap()
    }

    fn run_app(&self) {
        let port = self.server.address().port();
        fs::create_dir_all(self.proc_root()).unwrap();
        fake_process(
            &self.proc_root(),
            4242,
            &[
                "/opt/Antigravity/language_server_linux_x64",
                "--ide_name",
                "antigravity",
                "--csrf_token",
                "csrf-1",
            ],
            &[(port, 31_337)],
        );
    }

    async fn ls_answers(&self, rpc: &str, status: u16, body: &str) {
        Mock::given(method("POST"))
            .and(path(format!("{LS_PREFIX}/{rpc}")))
            .and(header("x-codeium-csrf-token", "csrf-1"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .mount(&self.server)
            .await;
    }

    async fn cloud_answers(&self, route: &str, status: u16, body: &str) {
        Mock::given(method("POST"))
            .and(path(route))
            .and(header("authorization", "Bearer ya29.fake"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .mount(&self.server)
            .await;
    }
}

fn token(expiry: Option<&str>) -> AccessToken {
    AccessToken {
        secret: SecretString::new("ya29.fake".into()),
        expires_at: expiry.map(|text| text.parse().unwrap()),
    }
}

#[tokio::test]
async fn nothing_installed_and_nothing_running_is_not_detected() {
    let setup = Setup::new().await;
    assert_eq!(
        setup.provider().discover().await,
        Err(ProviderError::NotSignedIn)
    );
}

#[tokio::test]
async fn an_antigravity_dir_is_one_cli_account() {
    let setup = Setup::new().await;
    fs::create_dir_all(setup.root.path().join("home/.gemini/antigravity-cli")).unwrap();
    let accounts = setup.provider().discover().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].owner, CredentialOwner::Cli);
    assert_eq!(accounts[0].home, setup.root.path().join("home/.gemini"));
    assert!(accounts[0].id.0.starts_with("antigravity:"));
}

#[tokio::test]
async fn the_running_language_server_supplies_pools_and_plan() {
    let setup = Setup::new().await;
    setup.run_app();
    setup
        .ls_answers("RetrieveUserQuotaSummary", 200, LS_SUMMARY)
        .await;
    setup.ls_answers("GetUserStatus", 200, USER_STATUS).await;
    let provider = setup.provider();
    let account = provider.discover().await.unwrap().remove(0);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    let ids: Vec<_> = snapshot.windows.iter().map(|w| w.id.clone()).collect();
    assert_eq!(
        ids,
        [
            WindowId::Session,
            WindowId::Weekly,
            WindowId::Model("claude".into()),
            WindowId::Model("claude:weekly".into())
        ]
    );
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Pro"));
    assert_eq!(snapshot.identity.account_id(&ID), account.id);
    assert_eq!(snapshot.source, LimitsSource::Live);
    assert!(snapshot.notices.is_empty());
}

#[tokio::test]
async fn a_server_without_the_summary_rpc_degrades_to_a_notice() {
    let setup = Setup::new().await;
    setup.run_app();
    setup.ls_answers("RetrieveUserQuotaSummary", 404, "").await;
    let provider = setup.provider();
    let account = provider.discover().await.unwrap().remove(0);
    let snapshot = provider.fetch_limits(&account).await.unwrap();
    assert!(snapshot.windows.is_empty());
    assert_eq!(snapshot.notices, [warning(NOT_FOUND_TEXT)]);
}

#[tokio::test]
async fn without_any_sign_in_the_snapshot_explains_what_to_do() {
    let setup = Setup::new().await;
    let provider = setup.provider();
    let snapshot = provider.fetch_limits(&provider.account()).await.unwrap();
    assert!(snapshot.windows.is_empty());
    assert_eq!(snapshot.identity.plan, None);
    assert_eq!(snapshot.notices, [warning(NOT_FOUND_TEXT)]);
    assert!(setup.server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_keyring_token_reads_cloud_code_pools_and_plan() {
    let setup = Setup::new().await;
    setup
        .cloud_answers("/v1internal:retrieveUserQuotaSummary", 200, CLOUD_SUMMARY)
        .await;
    setup
        .cloud_answers("/v1internal:loadCodeAssist", 200, CODE_ASSIST)
        .await;
    let snapshot = setup
        .provider()
        .query_cloud(&token(Some("2026-09-23T11:00:00Z")))
        .await
        .unwrap();
    assert_eq!(snapshot.windows.len(), 2);
    assert_eq!(snapshot.identity.plan.as_deref(), Some("Ultra"));
}

#[tokio::test]
async fn a_failed_plan_lookup_keeps_the_pools() {
    let setup = Setup::new().await;
    setup
        .cloud_answers("/v1internal:retrieveUserQuotaSummary", 200, CLOUD_SUMMARY)
        .await;
    let snapshot = setup.provider().query_cloud(&token(None)).await.unwrap();
    assert_eq!(snapshot.windows.len(), 2);
    assert_eq!(snapshot.identity.plan, None);
}

#[tokio::test]
async fn an_expired_keyring_token_is_never_refreshed() {
    let setup = Setup::new().await;
    let result = setup
        .provider()
        .query_cloud(&token(Some("2026-09-23T09:59:00Z")))
        .await;
    assert_eq!(result, Err(ProviderError::SignInExpired));
    assert!(setup.server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn cloud_errors_surface_unchanged() {
    for (status, expected) in [
        (401, ProviderError::SignInExpired),
        (429, ProviderError::RateLimited { retry_after: None }),
    ] {
        let setup = Setup::new().await;
        setup
            .cloud_answers("/v1internal:retrieveUserQuotaSummary", status, "")
            .await;
        let result = setup.provider().query_cloud(&token(None)).await;
        assert_eq!(result, Err(expected));
    }
}

#[tokio::test]
async fn a_summary_without_pools_says_so() {
    let setup = Setup::new().await;
    setup
        .cloud_answers(
            "/v1internal:retrieveUserQuotaSummary",
            200,
            r#"{"groups":[]}"#,
        )
        .await;
    let snapshot = setup.provider().query_cloud(&token(None)).await.unwrap();
    assert_eq!(snapshot.notices, [warning(NO_POOLS_TEXT)]);
}
