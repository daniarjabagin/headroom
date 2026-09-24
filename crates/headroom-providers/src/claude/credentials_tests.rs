use std::fs;
use std::path::PathBuf;

use headroom_core::account::{AccountId, CredentialOwner};
use jiff::Timestamp;
use serde_json::json;

use super::*;
use crate::keychain::fake::FakeKeychain;

fn now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

fn oauth(token: &str, subscription: &str) -> String {
    json!({ "claudeAiOauth": {
        "accessToken": token,
        "expiresAt": now().as_millisecond() + 3_600_000,
        "subscriptionType": subscription,
        "scopes": ["user:profile"]
    }})
    .to_string()
}

struct Setup {
    root: tempfile::TempDir,
    fake: FakeKeychain,
}

impl Setup {
    fn new() -> Setup {
        let root = tempfile::tempdir().unwrap();
        let fake = FakeKeychain::new(root.path());
        Setup { root, fake }
    }

    fn config(&self) -> ClaudeConfig {
        ClaudeConfig {
            user: Some("someone".to_owned()),
            keychain: Some(self.fake.security()),
            ..ClaudeConfig::for_home(self.root.path().join("home"))
        }
    }

    fn headroom_home(&self) -> PathBuf {
        let home = self.config().headroom_accounts_dir().join("h1");
        fs::create_dir_all(&home).unwrap();
        home
    }

    fn account(home: PathBuf, owner: CredentialOwner) -> AccountRef {
        AccountRef {
            id: AccountId("claude:000000000000".into()),
            provider: super::super::ID,
            home,
            owner,
        }
    }
}

fn token(credentials: &Credentials) -> String {
    credentials.usable_token(now()).unwrap().secret().to_owned()
}

#[tokio::test]
async fn a_headroom_home_reads_its_scoped_keychain_item() {
    let setup = Setup::new();
    let home = setup.headroom_home();
    let service = keychain::scoped_service(home.to_str().unwrap());
    setup
        .fake
        .insert(&service, Some("someone"), &oauth("kc-token", "max"));
    setup.fake.insert(
        keychain::SERVICE,
        Some("someone"),
        &oauth("cli-token", "pro"),
    );
    let account = Setup::account(home, CredentialOwner::Headroom);
    let credentials = load(&setup.config(), &account).await.unwrap();
    assert_eq!(token(&credentials), "kc-token");
    assert_eq!(credentials.plan.as_deref(), Some("Max"));
}

#[tokio::test]
async fn legacy_items_without_an_account_are_found() {
    let setup = Setup::new();
    let cli = setup.root.path().join("home/.claude");
    setup
        .fake
        .insert(keychain::SERVICE, None, &oauth("legacy-token", "pro"));
    let account = Setup::account(cli, CredentialOwner::Cli);
    let credentials = load(&setup.config(), &account).await.unwrap();
    assert_eq!(token(&credentials), "legacy-token");
    let argv = setup.fake.argv_log();
    let lines: Vec<&str> = argv.lines().collect();
    assert_eq!(
        lines,
        [
            "find-generic-password -s Claude Code-credentials -a someone -w",
            "find-generic-password -s Claude Code-credentials -w",
        ]
    );
}

#[tokio::test]
async fn a_hex_encoded_item_is_decoded() {
    let setup = Setup::new();
    let cli = setup.root.path().join("home/.claude");
    let encoded = hex::encode(oauth("hex-token", "pro"));
    setup
        .fake
        .insert(keychain::SERVICE, Some("someone"), &encoded);
    let account = Setup::account(cli, CredentialOwner::Cli);
    let credentials = load(&setup.config(), &account).await.unwrap();
    assert_eq!(token(&credentials), "hex-token");
}

#[tokio::test]
async fn a_missing_item_falls_back_to_the_credentials_file() {
    let setup = Setup::new();
    let home = setup.headroom_home();
    fs::write(home.join(CREDENTIALS_FILE), oauth("file-token", "pro")).unwrap();
    let account = Setup::account(home.clone(), CredentialOwner::Headroom);
    let credentials = load(&setup.config(), &account).await.unwrap();
    assert_eq!(token(&credentials), "file-token");
    fs::remove_file(home.join(CREDENTIALS_FILE)).unwrap();
    let missing = load(&setup.config(), &account).await.unwrap_err();
    assert_eq!(missing, ProviderError::NotSignedIn);
}

#[tokio::test]
async fn a_keychain_failure_is_an_account_error() {
    let setup = Setup::new();
    let home = setup.headroom_home();
    fs::write(home.join(CREDENTIALS_FILE), oauth("file-token", "pro")).unwrap();
    setup.fake.fail_with(51);
    let account = Setup::account(home, CredentialOwner::Headroom);
    let error = load(&setup.config(), &account).await.unwrap_err();
    let ProviderError::LocalData(message) = error else {
        panic!("expected a local data error, got {error:?}");
    };
    assert!(message.contains("Keychain"), "{message}");
}

#[tokio::test]
async fn without_a_keychain_only_the_file_is_read() {
    let setup = Setup::new();
    let home = setup.headroom_home();
    fs::write(home.join(CREDENTIALS_FILE), oauth("file-token", "pro")).unwrap();
    let config = ClaudeConfig {
        keychain: None,
        ..setup.config()
    };
    let account = Setup::account(home, CredentialOwner::Headroom);
    assert_eq!(token(&load(&config, &account).await.unwrap()), "file-token");
    assert_eq!(setup.fake.argv_log(), "");
}

#[test]
fn keychain_sign_ins_are_recognized_by_their_identity_file() {
    let setup = Setup::new();
    let home = setup.headroom_home();
    let file_only = ClaudeConfig {
        keychain: None,
        ..setup.config()
    };
    assert!(!has_sign_in(&setup.config(), &home));
    fs::write(home.join(".claude.json"), "{}").unwrap();
    assert!(has_sign_in(&setup.config(), &home));
    assert!(!has_sign_in(&file_only, &home));
    fs::write(home.join(CREDENTIALS_FILE), "{}").unwrap();
    assert!(has_sign_in(&file_only, &home));
}
