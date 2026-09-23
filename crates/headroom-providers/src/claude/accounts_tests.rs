use headroom_core::account::AccountId;
use serde_json::json;
use tempfile::TempDir;

use super::*;

fn state(account: &str, organization: &str) -> String {
    json!({
        "oauthAccount": {
            "accountUuid": account,
            "organizationUuid": organization,
            "emailAddress": format!("{account}@example.com")
        }
    })
    .to_string()
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn login(dir: &Path, identity_file: &Path, account: &str) {
    write(&dir.join(CREDENTIALS_FILE), "{}");
    write(identity_file, &state(account, "org-1"));
}

fn login_scoped(dir: &Path, account: &str) {
    login(dir, &dir.join(".claude.json"), account);
}

fn id(account: &str) -> AccountId {
    AccountId::from_stable_key(ProviderKind::Claude, &format!("{account}/org-1"))
}

fn setup() -> (TempDir, ClaudeConfig) {
    let home = tempfile::tempdir().unwrap();
    let config = ClaudeConfig::for_home(home.path().to_path_buf());
    (home, config)
}

fn summary(accounts: &[AccountRef]) -> Vec<(AccountId, PathBuf, CredentialOwner)> {
    accounts
        .iter()
        .map(|a| (a.id.clone(), a.home.clone(), a.owner))
        .collect()
}

#[test]
fn default_dir_uses_identity_in_home() {
    let (home, config) = setup();
    login(
        &home.path().join(".claude"),
        &home.path().join(".claude.json"),
        "acc-default",
    );
    assert_eq!(
        summary(&discover_accounts(&config)),
        [(
            id("acc-default"),
            home.path().join(".claude"),
            CredentialOwner::Cli
        )]
    );
}

#[test]
fn default_dir_without_credentials_file_is_still_listed() {
    let (home, config) = setup();
    write(
        &home.path().join(".claude.json"),
        &state("acc-default", "org-1"),
    );
    assert_eq!(discover_accounts(&config).len(), 1);
}

#[test]
fn extra_dirs_in_home_and_config_are_discovered() {
    let (home, config) = setup();
    login(
        &home.path().join(".claude"),
        &home.path().join(".claude.json"),
        "acc-default",
    );
    login_scoped(&home.path().join(".claude-work"), "acc-work");
    login_scoped(&home.path().join(".config/claude-alt"), "acc-alt");
    assert_eq!(
        summary(&discover_accounts(&config)),
        [
            (
                id("acc-default"),
                home.path().join(".claude"),
                CredentialOwner::Cli
            ),
            (
                id("acc-work"),
                home.path().join(".claude-work"),
                CredentialOwner::Cli
            ),
            (
                id("acc-alt"),
                home.path().join(".config/claude-alt"),
                CredentialOwner::Cli
            ),
        ]
    );
}

#[test]
fn same_identity_in_two_dirs_appears_once() {
    let (home, config) = setup();
    login(
        &home.path().join(".claude"),
        &home.path().join(".claude.json"),
        "acc-default",
    );
    login_scoped(&home.path().join(".claude-copy"), "acc-default");
    login_scoped(&home.path().join(".claude-x"), "acc-x");
    login_scoped(&home.path().join(".claude-y"), "acc-x");
    assert_eq!(
        summary(&discover_accounts(&config)),
        [
            (
                id("acc-default"),
                home.path().join(".claude"),
                CredentialOwner::Cli
            ),
            (
                id("acc-x"),
                home.path().join(".claude-x"),
                CredentialOwner::Cli
            ),
        ]
    );
}

#[test]
fn candidates_need_credentials_and_identity() {
    let (home, config) = setup();
    write(
        &home.path().join(".no-creds/.claude.json"),
        &state("acc-a", "org-1"),
    );
    write(
        &home.path().join(".no-identity").join(CREDENTIALS_FILE),
        "{}",
    );
    write(&home.path().join("visible").join(CREDENTIALS_FILE), "{}");
    write(
        &home.path().join("visible/.claude.json"),
        &state("acc-b", "org-1"),
    );
    assert!(discover_accounts(&config).is_empty());
}

#[test]
fn corrupt_identity_is_skipped() {
    let (home, config) = setup();
    login_scoped(&home.path().join(".claude-ok"), "acc-ok");
    write(
        &home.path().join(".claude-bad").join(CREDENTIALS_FILE),
        "{}",
    );
    write(&home.path().join(".claude-bad/.claude.json"), "{not json");
    assert_eq!(
        summary(&discover_accounts(&config)),
        [(
            id("acc-ok"),
            home.path().join(".claude-ok"),
            CredentialOwner::Cli
        )]
    );
}

#[test]
fn config_dir_override_reads_identity_inside_it() {
    let (home, mut config) = setup();
    let custom = home.path().join("work/claude");
    config.config_dir = Some(custom.clone());
    login_scoped(&custom, "acc-custom");
    login(
        &home.path().join(".claude"),
        &home.path().join(".claude.json"),
        "acc-default",
    );
    assert_eq!(
        summary(&discover_accounts(&config)),
        [
            (id("acc-custom"), custom, CredentialOwner::Cli),
            (
                id("acc-default"),
                home.path().join(".claude"),
                CredentialOwner::Cli
            ),
        ]
    );
}

#[test]
fn headroom_owned_dirs_are_discovered() {
    let (home, config) = setup();
    let owned = home
        .path()
        .join(".local/share/headroom/accounts/claude/0f7e");
    login_scoped(&owned, "acc-owned");
    assert_eq!(
        summary(&discover_accounts(&config)),
        [(id("acc-owned"), owned, CredentialOwner::Headroom)]
    );
}

#[test]
fn empty_home_has_no_accounts() {
    let (_home, config) = setup();
    assert!(discover_accounts(&config).is_empty());
}
