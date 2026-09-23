use super::super::hosts::HOSTS_FILE;
use super::*;

const MULTI: &str = include_str!("fixtures/hosts_multi.yml");

fn write_hosts(dir: &Path, text: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join(HOSTS_FILE), text).unwrap();
}

fn single(login: &str) -> String {
    format!("github.com:\n    users:\n        {login}:\n    user: {login}\n")
}

#[test]
fn every_gh_account_and_every_headroom_login_is_an_account() {
    let root = tempfile::tempdir().unwrap();
    let config = CopilotConfig::for_home(root.path());
    write_hosts(&config.gh_config_dir, MULTI);
    let owned = config.headroom_accounts_dir().join("a1");
    write_hosts(&owned, &single("mona"));
    fs::create_dir_all(config.headroom_accounts_dir().join("b2-empty")).unwrap();
    let accounts = discover_accounts(&config);
    let summary: Vec<_> = accounts
        .iter()
        .map(|a| (a.id.clone(), a.home.clone(), a.owner))
        .collect();
    assert_eq!(
        summary,
        [
            (
                account_id("octo-work"),
                config.gh_config_dir.clone(),
                CredentialOwner::Cli
            ),
            (
                account_id("octocat"),
                config.gh_config_dir.clone(),
                CredentialOwner::Cli
            ),
            (account_id("mona"), owned, CredentialOwner::Headroom),
        ]
    );
}

#[test]
fn a_user_known_to_both_is_listed_once_and_logins_ignore_case() {
    let root = tempfile::tempdir().unwrap();
    let config = CopilotConfig::for_home(root.path());
    write_hosts(&config.gh_config_dir, &single("octocat"));
    write_hosts(
        &config.headroom_accounts_dir().join("a1"),
        &single("OctoCat"),
    );
    let accounts = discover_accounts(&config);
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].owner, CredentialOwner::Cli);
    assert_eq!(stable_key("OctoCat"), "github.com/octocat");
}

#[test]
fn a_login_home_names_its_account_and_the_login_is_resolved_back() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("login");
    assert_eq!(headroom_account_at(&home), Ok(None));
    write_hosts(&home, &single("mona"));
    let account = headroom_account_at(&home).unwrap().unwrap();
    assert_eq!(account.owner, CredentialOwner::Headroom);
    assert_eq!(login_for(&account).unwrap(), "mona");
    write_hosts(&home, &single("someone-else"));
    assert_eq!(login_for(&account), Err(ProviderError::NotSignedIn));
}
