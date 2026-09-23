use std::os::unix::fs::PermissionsExt;

use super::*;

const CREDENTIALS: &str = include_str!("fixtures/credentials.json");
const REFRESHED: &str = include_str!("fixtures/refreshed.json");

fn now() -> Timestamp {
    "2026-09-23T11:00:00Z".parse().unwrap()
}

fn home_with(text: &str) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join(CREDENTIALS_FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
    home
}

#[test]
fn the_cli_token_file_is_read() {
    let home = home_with(CREDENTIALS);
    assert!(has_credentials(home.path()));
    let tokens = load(home.path()).unwrap();
    assert_eq!(tokens.access.expose(), "fake-access-token");
    assert_eq!(tokens.refresh.expose(), "fake-refresh-token");
    let expected: Timestamp = "2026-09-23T11:30:00.123Z".parse().unwrap();
    assert_eq!(
        tokens.expires_at.map(Timestamp::as_second),
        Some(expected.as_second())
    );
    assert!(tokens.valid_for(now(), SignedDuration::from_mins(29)));
    assert!(!tokens.valid_for(now(), SignedDuration::from_mins(31)));
}

#[test]
fn missing_and_broken_files_are_reported_apart() {
    let empty = tempfile::tempdir().unwrap();
    assert!(!has_credentials(empty.path()));
    assert!(matches!(
        load(empty.path()),
        Err(ProviderError::NotSignedIn)
    ));
    for broken in [
        "{",
        r#"{"access_token":"a"}"#,
        r#"{"access_token":"","refresh_token":"r"}"#,
    ] {
        let home = home_with(broken);
        let error = load(home.path()).err().unwrap();
        assert!(matches!(error, ProviderError::LocalData(_)), "{broken}");
        assert!(!error.to_string().contains("fake"));
    }
}

#[test]
fn a_token_without_expiry_is_tried() {
    let home = home_with(r#"{"access_token":"a","refresh_token":"r"}"#);
    let tokens = load(home.path()).unwrap();
    assert_eq!(tokens.expires_at, None);
    assert!(tokens.valid_for(now(), SignedDuration::from_hours(1)));
}

#[test]
fn refreshed_tokens_are_written_back_privately_in_the_cli_format() {
    let home =
        home_with(r#"{"access_token":"a","refresh_token":"r","expires_at":1.5,"device":"keep"}"#);
    let previous = load(home.path()).unwrap();
    let refreshed: RefreshedTokens = serde_json::from_str(REFRESHED).unwrap();
    assert_eq!(refreshed.access().expose(), "fake-access-token-2");
    save_refreshed(home.path(), &previous, &refreshed, now()).unwrap();
    let path = home.path().join(CREDENTIALS_FILE);
    let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        saved,
        serde_json::json!({
            "access_token": "fake-access-token-2",
            "refresh_token": "fake-refresh-token-2",
            "expires_at": now().as_second() + 900,
            "expires_in": 900,
            "scope": "kimi-code",
            "token_type": "Bearer",
            "device": "keep"
        })
    );
    let mode = fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    let reloaded = load(home.path()).unwrap();
    assert!(reloaded.valid_for(now(), SignedDuration::from_mins(14)));
}

#[test]
fn an_unusable_expiry_is_not_saved() {
    let home = home_with(CREDENTIALS);
    let previous = load(home.path()).unwrap();
    let refreshed: RefreshedTokens =
        serde_json::from_str(r#"{"access_token":"a2","refresh_token":"r2","expires_in":0}"#)
            .unwrap();
    let error = save_refreshed(home.path(), &previous, &refreshed, now())
        .err()
        .unwrap();
    assert!(matches!(error, ProviderError::InvalidResponse(_)));
    assert_eq!(
        load(home.path()).unwrap().access.expose(),
        "fake-access-token"
    );
}
