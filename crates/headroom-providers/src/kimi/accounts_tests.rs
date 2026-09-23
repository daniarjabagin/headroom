use tempfile::TempDir;

use super::*;
use crate::kimi::credentials::CREDENTIALS_FILE;

fn sign_in(home: &Path) {
    let path = home.join(CREDENTIALS_FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, include_str!("fixtures/credentials.json")).unwrap();
}

fn config(root: &TempDir) -> KimiConfig {
    KimiConfig {
        share_dir: root.path().join(".kimi"),
        accounts_dir: root.path().join("accounts/kimi"),
        ..KimiConfig::for_home(root.path())
    }
}

#[test]
fn key_cli_and_headroom_logins_are_discovered() {
    let root = tempfile::tempdir().unwrap();
    let config = config(&root);
    let keyed = config.accounts_dir.join("a-key");
    fs::create_dir_all(&keyed).unwrap();
    key_accounts::save_record(&keyed, &key_identity("sk-1", None)).unwrap();
    let login = config.accounts_dir.join("b-login");
    sign_in(&login);
    fs::create_dir_all(config.accounts_dir.join("c-pending")).unwrap();
    sign_in(&config.share_dir);

    let found = discover(&config).unwrap();
    let summary: Vec<_> = found.iter().map(|a| (a.home.clone(), a.owner)).collect();
    assert_eq!(
        summary,
        [
            (keyed, CredentialOwner::Headroom),
            (config.share_dir.clone(), CredentialOwner::Cli),
            (login.clone(), CredentialOwner::Headroom),
        ]
    );
    assert_eq!(found[0].id, key_identity("sk-1", None).account_id(&ID));
    assert_eq!(found[2].id, oauth_identity(&login).account_id(&ID));
    assert!(found.iter().all(|a| a.provider == ID));
}

#[test]
fn nothing_signed_in_finds_nothing() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(discover(&config(&root)), Ok(Vec::new()));
}

#[test]
fn a_share_dir_inside_the_accounts_root_is_listed_once() {
    let root = tempfile::tempdir().unwrap();
    let mut config = config(&root);
    let login = config.accounts_dir.join("login");
    sign_in(&login);
    config.share_dir = login;
    assert_eq!(discover(&config).unwrap().len(), 1);
}

#[test]
fn the_credential_kind_follows_the_home_contents() {
    let root = tempfile::tempdir().unwrap();
    let keyed = root.path().join("keyed");
    fs::create_dir_all(&keyed).unwrap();
    key_accounts::save_record(&keyed, &key_identity("sk-1", Some("Basic".into()))).unwrap();
    assert!(
        matches!(credential(&keyed), Ok(Credential::Key(identity)) if identity.plan.as_deref() == Some("Basic"))
    );
    let login = root.path().join("login");
    sign_in(&login);
    assert!(matches!(credential(&login), Ok(Credential::OAuth(_))));
    assert!(matches!(
        credential(&root.path().join("none")),
        Err(ProviderError::NotSignedIn)
    ));
}

#[test]
fn key_identities_are_stable_and_do_not_reveal_the_key() {
    let identity = key_identity("sk-secret", Some("Basic".into()));
    assert_eq!(identity, key_identity("sk-secret", Some("Basic".into())));
    assert_ne!(
        identity.stable_key,
        key_identity("sk-other", None).stable_key
    );
    assert_eq!(
        identity.stable_key.len(),
        "key:".len() + KEY_FINGERPRINT_HEX
    );
    assert!(!identity.stable_key.contains("secret"));
    assert_eq!(identity.email, None);
}
