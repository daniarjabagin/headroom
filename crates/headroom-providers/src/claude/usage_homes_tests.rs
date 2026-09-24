use std::fs;

use serde_json::json;
use tempfile::TempDir;

use super::*;

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn identity(account: &str) -> String {
    json!({ "oauthAccount": { "accountUuid": account, "organizationUuid": "org-1" } }).to_string()
}

fn with_projects(dir: &Path) -> PathBuf {
    fs::create_dir_all(dir.join(PROJECTS_DIR)).unwrap();
    dir.to_path_buf()
}

fn setup() -> (TempDir, ClaudeConfig) {
    let home = tempfile::tempdir().unwrap();
    let config = ClaudeConfig::for_home(home.path().to_path_buf());
    (home, config)
}

#[test]
fn dirs_signed_into_the_same_account_are_all_usage_homes() {
    let (home, config) = setup();
    let default_dir = with_projects(&home.path().join(".claude"));
    write(&home.path().join(".claude.json"), &identity("acc-1"));
    let copy = with_projects(&home.path().join(".claude-copy"));
    write(&copy.join(IDENTITY_FILE), &identity("acc-1"));
    write(&copy.join(CREDENTIALS_FILE), "{}");
    assert_eq!(usage_homes(&config), [default_dir, copy]);
}

#[test]
fn dirs_with_projects_but_no_identity_are_usage_homes() {
    let (home, config) = setup();
    let api_key = with_projects(&home.path().join(".claude-api"));
    write(
        &api_key.join(IDENTITY_FILE),
        r#"{"primaryApiKey":"sk-fake"}"#,
    );
    let logged_out = with_projects(&home.path().join(".config/claude-old"));
    write(&logged_out.join(CREDENTIALS_FILE), "{}");
    let default_dir = with_projects(&home.path().join(".claude"));
    assert_eq!(usage_homes(&config), [default_dir, api_key, logged_out]);
}

#[test]
fn scanned_dirs_need_projects_and_a_config_file() {
    let (home, config) = setup();
    with_projects(&home.path().join(".cache"));
    write(
        &home.path().join(".claude-empty").join(CREDENTIALS_FILE),
        "{}",
    );
    with_projects(&home.path().join("visible"));
    write(&home.path().join("visible").join(CREDENTIALS_FILE), "{}");
    assert!(usage_homes(&config).is_empty());
}

#[test]
fn config_dir_override_comes_first_and_default_dir_stays() {
    let (home, mut config) = setup();
    let custom = with_projects(&home.path().join("work/claude"));
    config.config_dir = Some(custom.clone());
    let default_dir = with_projects(&home.path().join(".claude"));
    assert_eq!(usage_homes(&config), [custom, default_dir]);
}

#[test]
fn override_linked_to_the_default_dir_is_listed_once() {
    let (home, mut config) = setup();
    let default_dir = with_projects(&home.path().join(".claude"));
    let link = home.path().join("claude-link");
    std::os::unix::fs::symlink(&default_dir, &link).unwrap();
    config.config_dir = Some(link.clone());
    assert_eq!(usage_homes(&config), [link]);
}

#[test]
fn headroom_owned_dirs_with_projects_are_usage_homes() {
    let (_home, config) = setup();
    let root = config.headroom_accounts_dir();
    let used = with_projects(&root.join("a"));
    fs::create_dir_all(root.join("b")).unwrap();
    assert_eq!(usage_homes(&config), [used]);
}
