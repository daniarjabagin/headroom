use headroom_core::account::AccountIdentity;

use super::super::auth::AUTH_FILE;
use super::*;

const AUTH: &str = include_str!("fixtures/auth.json");
const AUTH_ORG: &str = include_str!("fixtures/auth_organization.json");

fn write_login(dir: &Path, text: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join(AUTH_FILE), text).unwrap();
}

fn write_key_record(home: &Path) {
    fs::create_dir_all(home).unwrap();
    let identity = AccountIdentity {
        email: None,
        plan: None,
        stable_key: key_accounts::sha256_stable_key("kilo-pasted-key"),
    };
    key_accounts::save_record(home, &identity).unwrap();
}

fn summary(accounts: &[AccountRef]) -> Vec<(PathBuf, CredentialOwner)> {
    accounts
        .iter()
        .map(|account| (account.home.clone(), account.owner))
        .collect()
}

#[test]
fn key_cli_and_headroom_logins_are_all_found() {
    let root = tempfile::tempdir().unwrap();
    let config = KiloConfig::for_home(root.path());
    write_login(&config.data_dir, AUTH);
    let key_home = config.accounts_dir.join("a-key");
    write_key_record(&key_home);
    let login_home = config.accounts_dir.join("b-login");
    write_login(&login_home.join("kilo"), AUTH_ORG);
    fs::create_dir_all(config.accounts_dir.join("c-pending")).unwrap();
    let accounts = discover(&config).unwrap();
    assert_eq!(
        summary(&accounts),
        [
            (key_home, CredentialOwner::Headroom),
            (config.data_dir.clone(), CredentialOwner::Cli),
            (login_home.clone(), CredentialOwner::Headroom),
        ]
    );
    assert_eq!(
        headroom_account_at(&login_home).unwrap().unwrap(),
        accounts[2]
    );
}

#[test]
fn nothing_signed_in_is_empty_and_broken_logins_are_skipped() {
    let root = tempfile::tempdir().unwrap();
    let config = KiloConfig::for_home(root.path());
    assert_eq!(discover(&config), Ok(Vec::new()));
    write_login(&config.data_dir, "{");
    assert_eq!(discover(&config), Ok(Vec::new()));
    assert_eq!(headroom_account_at(root.path()), Ok(None));
}

#[test]
fn credentials_resolve_by_owner_and_detect_a_changed_login() {
    let root = tempfile::tempdir().unwrap();
    let config = KiloConfig::for_home(root.path());
    write_login(&config.data_dir, AUTH_ORG);
    let key_home = config.accounts_dir.join("a-key");
    write_key_record(&key_home);
    let accounts = discover(&config).unwrap();
    assert!(matches!(credential(&accounts[0]), Ok(Credential::Key)));
    let Ok(Credential::Login(login)) = credential(&accounts[1]) else {
        panic!("expected the CLI login");
    };
    assert_eq!(login.organization_id.as_deref(), Some("org-0000-fake"));
    write_login(&config.data_dir, AUTH);
    assert!(matches!(
        credential(&accounts[1]),
        Err(ProviderError::AccountChanged(ref m)) if m.contains("has changed")
    ));
    fs::remove_file(config.data_dir.join(AUTH_FILE)).unwrap();
    assert!(matches!(
        credential(&accounts[1]),
        Err(ProviderError::NotSignedIn)
    ));
}
