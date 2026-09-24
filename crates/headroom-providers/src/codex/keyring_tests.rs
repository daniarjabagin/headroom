use std::path::PathBuf;

use super::super::test_support::{ACCOUNT_ID, access_token, at, auth_document, write_auth};
use super::*;
use crate::keychain::fake::FakeKeychain;

#[test]
fn store_key_matches_the_codex_test_vector() {
    assert_eq!(store_key(Path::new("~/.codex")), "cli|940db7b1d0e4eb40");
}

#[test]
fn store_key_hashes_the_canonical_home() {
    let root = tempfile::tempdir().unwrap();
    let real = root.path().join("real");
    let link = root.path().join("link");
    fs::create_dir_all(&real).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert_eq!(store_key(&link), store_key(&real));
    let canonical = fs::canonicalize(&real).unwrap();
    let digest = hex::encode(Sha256::digest(canonical.to_str().unwrap().as_bytes()));
    assert_eq!(store_key(&real), format!("cli|{}", &digest[..16]));
}

#[test]
fn store_mode_comes_from_the_top_level_setting() {
    let cases = [
        (
            "cli_auth_credentials_store = \"keyring\"\n",
            StoreMode::Keyring,
        ),
        ("cli_auth_credentials_store = 'auto'", StoreMode::Auto),
        ("cli_auth_credentials_store = \"file\"", StoreMode::File),
        (
            "cli_auth_credentials_store = \"ephemeral\"",
            StoreMode::File,
        ),
        ("model = \"o3\"", StoreMode::File),
        (
            "[profiles.x]\ncli_auth_credentials_store = \"keyring\"",
            StoreMode::File,
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(parse_store_mode(text), expected, "{text}");
    }
}

struct Setup {
    root: tempfile::TempDir,
    fake: FakeKeychain,
}

impl Setup {
    fn new(config: Option<&str>) -> Setup {
        let root = tempfile::tempdir().unwrap();
        let fake = FakeKeychain::new(root.path());
        fs::create_dir_all(root.path().join("codex")).unwrap();
        if let Some(config) = config {
            fs::write(root.path().join("codex/config.toml"), config).unwrap();
        }
        Setup { root, fake }
    }

    fn home(&self) -> PathBuf {
        self.root.path().join("codex")
    }

    fn keychain_item(&self, access: &str) {
        let document = auth_document(access).to_string();
        self.fake
            .insert(SERVICE, Some(&store_key(&self.home())), &document);
    }
}

fn token(expiry: &str) -> String {
    access_token(at(expiry))
}

#[tokio::test]
async fn keyring_mode_reads_the_codex_auth_item() {
    let setup = Setup::new(Some("cli_auth_credentials_store = \"keyring\"\n"));
    setup.keychain_item(&token("2026-10-01T00:00:00Z"));
    let credentials = load(Some(&setup.fake.security()), &setup.home())
        .await
        .unwrap();
    assert_eq!(credentials.account_id.as_deref(), Some(ACCOUNT_ID));
    assert_eq!(credentials.expires_at, Some(at("2026-10-01T00:00:00Z")));
    let expected = format!(
        "find-generic-password -s Codex Auth -a {} -w\n",
        store_key(&setup.home())
    );
    assert_eq!(setup.fake.argv_log(), expected);
}

#[tokio::test]
async fn auto_mode_prefers_an_existing_auth_file() {
    let setup = Setup::new(Some("cli_auth_credentials_store = \"auto\"\n"));
    setup.keychain_item(&token("2026-10-01T00:00:00Z"));
    let security = setup.fake.security();
    let from_keychain = load(Some(&security), &setup.home()).await.unwrap();
    assert_eq!(from_keychain.expires_at, Some(at("2026-10-01T00:00:00Z")));
    write_auth(
        &setup.home(),
        &auth_document(&token("2026-11-01T00:00:00Z")),
    );
    let from_file = load(Some(&security), &setup.home()).await.unwrap();
    assert_eq!(from_file.expires_at, Some(at("2026-11-01T00:00:00Z")));
}

#[tokio::test]
async fn file_mode_and_no_keychain_never_run_security() {
    let setup = Setup::new(None);
    write_auth(
        &setup.home(),
        &auth_document(&token("2026-11-01T00:00:00Z")),
    );
    assert!(
        load(Some(&setup.fake.security()), &setup.home())
            .await
            .is_ok()
    );
    let keyring = Setup::new(Some("cli_auth_credentials_store = \"keyring\"\n"));
    assert_eq!(
        load(None, &keyring.home()).await.unwrap_err(),
        ProviderError::NotSignedIn
    );
    assert_eq!(setup.fake.argv_log(), "");
    assert_eq!(keyring.fake.argv_log(), "");
}

#[tokio::test]
async fn a_missing_item_is_not_signed_in_and_failures_are_errors() {
    let setup = Setup::new(Some("cli_auth_credentials_store = \"keyring\"\n"));
    let security = setup.fake.security();
    assert_eq!(
        load(Some(&security), &setup.home()).await.unwrap_err(),
        ProviderError::NotSignedIn
    );
    setup.fake.fail_with(36);
    let error = load(Some(&security), &setup.home()).await.unwrap_err();
    assert!(
        matches!(&error, ProviderError::LocalData(message) if message.contains("Keychain")),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_hex_encoded_item_is_decoded() {
    let setup = Setup::new(Some("cli_auth_credentials_store = \"keyring\"\n"));
    let document = auth_document(&token("2026-10-01T00:00:00Z")).to_string();
    setup.fake.insert(
        SERVICE,
        Some(&store_key(&setup.home())),
        &hex::encode(document),
    );
    let credentials = load(Some(&setup.fake.security()), &setup.home())
        .await
        .unwrap();
    assert_eq!(credentials.account_id.as_deref(), Some(ACCOUNT_ID));
}
