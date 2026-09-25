use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use headroom_core::account::{AccountId, AccountRef, CredentialOwner};
use jiff::Timestamp;
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::jwt::unsigned_token;
use super::{CodexConfig, CodexEnvironment, CodexProvider};
use crate::paths::HeadroomDirs;

pub(super) const USER_ID: &str = "user-fake0001";
pub(super) const ACCOUNT_ID: &str = "acct-fake0001";
pub(super) const EMAIL: &str = "someone@example.com";
pub(super) const NOW: &str = "2026-09-23T10:00:00Z";
pub(super) const SIGNED_IN_AT: &str = "2026-09-01T00:00:00Z";

pub(super) fn at(text: &str) -> Timestamp {
    text.parse().unwrap()
}

pub(super) fn id_token() -> String {
    id_token_for(USER_ID, ACCOUNT_ID, Some(at(SIGNED_IN_AT)))
}

pub(super) fn id_token_for(
    user_id: &str,
    account_id: &str,
    auth_time: Option<Timestamp>,
) -> String {
    let mut claims = json!({
        "email": EMAIL,
        "sub": "auth0|fake",
        "https://api.openai.com/auth": {
            "chatgpt_account_id": account_id,
            "chatgpt_user_id": user_id,
            "chatgpt_plan_type": "plus",
        }
    });
    if let Some(auth_time) = auth_time {
        claims["auth_time"] = json!(auth_time.as_second());
    }
    unsigned_token(&claims)
}

pub(super) fn access_token(expires_at: Timestamp) -> String {
    unsigned_token(&json!({ "exp": expires_at.as_second() }))
}

pub(super) fn auth_document(access_token: &str) -> Value {
    auth_document_with(&id_token(), ACCOUNT_ID, access_token)
}

pub(super) fn auth_document_with(id_token: &str, account_id: &str, access_token: &str) -> Value {
    json!({
        "OPENAI_API_KEY": null,
        "tokens": {
            "id_token": id_token,
            "access_token": access_token,
            "refresh_token": "rt-fake",
            "account_id": account_id,
        },
        "last_refresh": "2026-09-20T10:00:00Z",
    })
}

pub(super) fn write_auth(home: &Path, document: &Value) {
    fs::create_dir_all(home).unwrap();
    fs::write(home.join("auth.json"), document.to_string()).unwrap();
}

pub(super) fn set_mtime(path: &Path, when: &str) {
    let seconds = at(when).as_second().unsigned_abs();
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))
        .unwrap();
}

pub(super) fn write_rollout(home: &Path, relative: &str, content: &str) -> std::path::PathBuf {
    let path = home.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

pub(super) struct Setup {
    pub root: TempDir,
}

impl Setup {
    pub(super) fn new() -> Setup {
        Setup {
            root: tempfile::tempdir().unwrap(),
        }
    }

    pub(super) fn cli_home(&self) -> PathBuf {
        self.root.path().join("home/.codex")
    }

    pub(super) fn headroom_home(&self, name: &str) -> PathBuf {
        self.root
            .path()
            .join("data/headroom/accounts/codex")
            .join(name)
    }

    pub(super) fn provider(&self, api_base: &str) -> CodexProvider {
        CodexProvider::new(CodexConfig {
            environment: CodexEnvironment {
                codex_home: None,
                home_dir: Some(self.root.path().join("home")),
                headroom: Some(HeadroomDirs {
                    data: self.root.path().join("data/headroom"),
                }),
                keychain: None,
            },
            api_base: api_base.to_owned(),
            auth_base: api_base.to_owned(),
            clock: Arc::new(|| at(NOW)),
        })
        .unwrap()
    }

    pub(super) fn sign_in(&self, expires_at: &str) {
        write_auth(
            &self.cli_home(),
            &auth_document(&access_token(at(expires_at))),
        );
    }

    pub(super) fn account(&self) -> AccountRef {
        AccountRef {
            id: AccountId::from_stable_key(&super::ID, &format!("{USER_ID}/{ACCOUNT_ID}")),
            provider: super::ID,
            home: self.cli_home(),
            owner: CredentialOwner::Cli,
        }
    }
}

pub(super) async fn server_with(response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/backend-api/wham/usage"))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}
