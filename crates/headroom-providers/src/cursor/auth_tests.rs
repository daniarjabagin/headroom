use std::fs;

use super::*;
use crate::cursor::test_support::{
    config_in, fixed_now, sign_in_agent, sign_in_ide, token, valid_token, write_file,
    write_state_db,
};

#[test]
fn ide_login_is_read_from_the_state_database() {
    let home = tempfile::tempdir().unwrap();
    let config = config_in(home.path());
    sign_in_ide(&config, &valid_token("auth0|user_ide"), "pro");
    let credentials = load_credentials(&config).unwrap().unwrap();
    assert_eq!(credentials.source, Source::Ide);
    assert_eq!(credentials.subject, "auth0|user_ide");
    assert_eq!(credentials.membership.as_deref(), Some("pro"));
    assert_eq!(credentials.email.as_deref(), Some("someone@example.com"));
    assert_eq!(
        credentials.home(&config),
        home.path().join(".config/Cursor")
    );
    assert!(credentials.usable_token(fixed_now()).is_ok());
}

#[test]
fn agent_file_is_the_fallback() {
    let home = tempfile::tempdir().unwrap();
    let config = config_in(home.path());
    sign_in_agent(&config, &valid_token("auth0|user_cli"));
    let credentials = load_credentials(&config).unwrap().unwrap();
    assert_eq!(credentials.source, Source::Agent);
    assert_eq!(credentials.subject, "auth0|user_cli");
    assert_eq!(credentials.membership, None);
    assert_eq!(
        credentials.home(&config),
        home.path().join(".config/cursor")
    );
}

#[test]
fn ide_wins_unless_it_is_free_and_the_agent_is_someone_else() {
    let home = tempfile::tempdir().unwrap();
    let config = config_in(home.path());
    sign_in_ide(&config, &valid_token("auth0|user_ide"), "pro");
    sign_in_agent(&config, &valid_token("auth0|user_cli"));
    assert_eq!(
        load_credentials(&config).unwrap().unwrap().source,
        Source::Ide
    );

    sign_in_ide(&config, &valid_token("auth0|user_ide"), "free");
    assert_eq!(
        load_credentials(&config).unwrap().unwrap().source,
        Source::Agent
    );

    sign_in_ide(&config, &valid_token("auth0|user_cli"), "free");
    assert_eq!(
        load_credentials(&config).unwrap().unwrap().source,
        Source::Ide
    );
}

#[test]
fn nothing_signed_in_is_none() {
    let home = tempfile::tempdir().unwrap();
    let config = config_in(home.path());
    assert!(load_credentials(&config).unwrap().is_none());
    write_state_db(&config, &[("cursorAuth/stripeMembershipType", "free")]);
    write_file(&config.agent_auth_file(), r#"{"accessToken":"  "}"#);
    assert!(load_credentials(&config).unwrap().is_none());
}

#[test]
fn expired_token_is_sign_in_expired() {
    let home = tempfile::tempdir().unwrap();
    let config = config_in(home.path());
    let expired = token(
        "auth0|user_ide",
        fixed_now() - jiff::SignedDuration::from_secs(1),
    );
    sign_in_ide(&config, &expired, "pro");
    let credentials = load_credentials(&config).unwrap().unwrap();
    assert_eq!(
        credentials.usable_token(fixed_now()).unwrap_err(),
        ProviderError::SignInExpired
    );
}

#[test]
fn token_without_subject_is_a_local_data_error() {
    let home = tempfile::tempdir().unwrap();
    let config = config_in(home.path());
    write_state_db(&config, &[("cursorAuth/accessToken", "opaque-token")]);
    assert!(matches!(
        load_credentials(&config),
        Err(ProviderError::LocalData(_))
    ));
}

#[test]
fn broken_state_database_falls_back_to_the_agent() {
    let home = tempfile::tempdir().unwrap();
    let config = config_in(home.path());
    write_file(
        &config.state_db(),
        "this is not an sqlite database, only some text",
    );
    assert!(matches!(
        load_credentials(&config),
        Err(ProviderError::LocalData(_))
    ));
    sign_in_agent(&config, &valid_token("auth0|user_cli"));
    assert_eq!(
        load_credentials(&config).unwrap().unwrap().source,
        Source::Agent
    );
}

#[test]
fn malformed_or_huge_agent_files_are_errors() {
    let home = tempfile::tempdir().unwrap();
    let config = config_in(home.path());
    write_file(&config.agent_auth_file(), "{not json");
    assert!(matches!(
        load_credentials(&config),
        Err(ProviderError::LocalData(_))
    ));
    let huge = format!(r#"{{"accessToken":"{}"}}"#, "x".repeat(70 * 1024));
    write_file(&config.agent_auth_file(), &huge);
    assert!(matches!(
        load_credentials(&config),
        Err(ProviderError::LocalData(_))
    ));
}

#[test]
fn credential_files_are_never_modified() {
    let home = tempfile::tempdir().unwrap();
    let config = config_in(home.path());
    sign_in_ide(&config, &valid_token("auth0|user_ide"), "pro");
    sign_in_agent(&config, &valid_token("auth0|user_cli"));
    let db = fs::read(config.state_db()).unwrap();
    let agent = fs::read(config.agent_auth_file()).unwrap();
    load_credentials(&config).unwrap();
    assert_eq!(fs::read(config.state_db()).unwrap(), db);
    assert_eq!(fs::read(config.agent_auth_file()).unwrap(), agent);
}

#[test]
fn debug_output_hides_the_token() {
    let home = tempfile::tempdir().unwrap();
    let config = config_in(home.path());
    let secret = valid_token("auth0|user_ide");
    sign_in_ide(&config, &secret, "pro");
    let credentials = load_credentials(&config).unwrap().unwrap();
    assert!(!format!("{credentials:?}").contains(&secret));
}
