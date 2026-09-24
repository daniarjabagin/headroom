use super::super::auth::CREDENTIALS_FILE;
use super::super::state_db::STATE_DB_FILE;
use super::super::state_db::tests::write_state_db;
use super::*;

fn write_credentials(dir: &Path, key: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join(CREDENTIALS_FILE),
        format!("windsurf_api_key = \"{key}\"\n"),
    )
    .unwrap();
}

fn write_app(config: &DevinConfig, key: &str) {
    let dir = config.app_state_dir();
    fs::create_dir_all(&dir).unwrap();
    write_state_db(
        &dir.join(STATE_DB_FILE),
        Some(&format!(r#"{{"apiKey":"{key}"}}"#)),
    );
}

fn summary(accounts: &[AccountRef]) -> Vec<(PathBuf, CredentialOwner)> {
    accounts
        .iter()
        .map(|account| (account.home.clone(), account.owner))
        .collect()
}

#[test]
fn cli_app_and_headroom_sign_ins_are_all_found() {
    let root = tempfile::tempdir().unwrap();
    let config = DevinConfig::for_home(root.path());
    write_credentials(&config.cli_dir(), "cli-key");
    write_app(&config, "app-key");
    let owned = config.headroom_accounts_dir().join("b1");
    write_credentials(&owned.join("devin"), "owned-key");
    fs::create_dir_all(config.headroom_accounts_dir().join("a0-pending")).unwrap();
    let accounts = discover_accounts(&config);
    assert_eq!(
        summary(&accounts),
        [
            (config.cli_dir(), CredentialOwner::Cli),
            (config.app_state_dir(), CredentialOwner::Cli),
            (owned, CredentialOwner::Headroom),
        ]
    );
    assert!(
        accounts
            .iter()
            .all(|account| account.provider == super::super::ID)
    );
}

#[test]
fn the_same_key_in_the_cli_and_the_app_is_listed_cli_first() {
    let root = tempfile::tempdir().unwrap();
    let config = DevinConfig::for_home(root.path());
    write_credentials(&config.cli_dir(), "shared-key");
    write_app(&config, "shared-key");
    let accounts = discover_accounts(&config);
    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].id, accounts[1].id);
    assert_eq!(accounts[0].home, config.cli_dir());
    assert_eq!(accounts[0].owner, CredentialOwner::Cli);
}

#[test]
fn broken_sign_ins_are_skipped_and_nothing_found_is_empty() {
    let root = tempfile::tempdir().unwrap();
    let config = DevinConfig::for_home(root.path());
    assert!(discover_accounts(&config).is_empty());
    fs::create_dir_all(config.cli_dir()).unwrap();
    fs::write(
        config.cli_dir().join(CREDENTIALS_FILE),
        "windsurf_api_key = \"k\"\napi_server_url = \"http://plain.example\"\n",
    )
    .unwrap();
    write_app(&config, "app-key");
    let accounts = discover_accounts(&config);
    assert_eq!(
        summary(&accounts),
        [(config.app_state_dir(), CredentialOwner::Cli)]
    );
}

#[test]
fn a_fresh_login_home_is_identified() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("new");
    assert_eq!(headroom_account_at(&home), Ok(None));
    write_credentials(&home.join("devin"), "owned-key");
    let account = headroom_account_at(&home).unwrap().unwrap();
    assert_eq!(account.home, home);
    assert_eq!(account.owner, CredentialOwner::Headroom);
    assert_eq!(
        key_dir(&home, CredentialOwner::Headroom),
        home.join("devin")
    );
}
