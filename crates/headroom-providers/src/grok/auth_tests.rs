use serde_json::json;

use super::*;

const AUTH: &str = include_str!("fixtures/auth.json");

fn fixture() -> Value {
    serde_json::from_str(AUTH).unwrap()
}

fn ts(text: &str) -> Timestamp {
    text.parse().unwrap()
}

#[test]
fn the_cli_entry_maps_to_credentials() {
    let credentials = credentials_from(&fixture()).unwrap();
    assert_eq!(
        credentials.entry,
        "https://auth.x.ai::b1a00492-073a-47ea-816f-4c329264a828"
    );
    assert_eq!(credentials.access_token, "fake-access-token");
    assert_eq!(
        credentials.refresh_token.as_deref(),
        Some("fake-refresh-token")
    );
    assert_eq!(
        credentials.client_id.as_deref(),
        Some("b1a00492-073a-47ea-816f-4c329264a828")
    );
    assert_eq!(credentials.issuer.as_deref(), Some("https://auth.x.ai"));
    assert_eq!(credentials.expires_at, Some(ts("2026-09-22T22:55:28.610Z")));
    assert_eq!(
        credentials.identity,
        AccountIdentity {
            email: Some("ada@example.com".into()),
            plan: None,
            stable_key: "user-fake-1/team-fake-1".into(),
        }
    );
}

#[test]
fn issuer_and_client_fall_back_to_the_entry_name() {
    let document = json!({
        "https://auth.x.ai::client-9": { "key": "t", "user_id": "u", "refresh": "r" }
    });
    let credentials = credentials_from(&document).unwrap();
    assert_eq!(credentials.issuer.as_deref(), Some("https://auth.x.ai"));
    assert_eq!(credentials.client_id.as_deref(), Some("client-9"));
    assert_eq!(credentials.refresh_token.as_deref(), Some("r"));
    assert_eq!(credentials.expires_at, None);
    assert_eq!(credentials.identity.stable_key, "u/");
}

#[test]
fn expiry_checks_use_a_five_minute_margin_for_refresh() {
    let credentials = credentials_from(&fixture()).unwrap();
    let expiry = ts("2026-09-22T22:55:28.610Z");
    assert!(!credentials.is_expired(ts("2026-09-22T22:50:00Z")));
    assert!(credentials.expires_soon(ts("2026-09-22T22:50:30Z")));
    assert!(!credentials.expires_soon(ts("2026-09-22T22:50:00Z")));
    assert!(credentials.is_expired(expiry));
}

#[test]
fn documents_without_a_token_are_signed_out() {
    for document in [
        json!({}),
        json!({ "entry": { "key": "  ", "user_id": "u" } }),
        json!({ "entry": "text" }),
    ] {
        assert_eq!(
            credentials_from(&document),
            Err(ProviderError::NotSignedIn),
            "{document}"
        );
    }
}

#[test]
fn broken_entries_are_local_data_errors() {
    let no_user = json!({ "e": { "key": "t" } });
    let bad_expiry = json!({ "e": { "key": "t", "user_id": "u", "expires_at": "soon" } });
    assert!(matches!(
        credentials_from(&no_user),
        Err(ProviderError::LocalData(_))
    ));
    assert!(matches!(
        credentials_from(&bad_expiry),
        Err(ProviderError::LocalData(_))
    ));
}

#[test]
fn files_are_read_or_reported() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(
        load_credentials(home.path()),
        Err(ProviderError::NotSignedIn)
    );
    fs::write(home.path().join(AUTH_FILE), "not json").unwrap();
    assert!(matches!(
        load_credentials(home.path()),
        Err(ProviderError::LocalData(_))
    ));
    fs::write(home.path().join(AUTH_FILE), AUTH).unwrap();
    let file = read_auth_file(home.path()).unwrap();
    assert_eq!(file.bytes, AUTH.as_bytes());
    assert!(load_credentials(home.path()).is_ok());
}

#[test]
fn debug_output_hides_tokens() {
    let text = format!("{:?}", credentials_from(&fixture()).unwrap());
    assert!(!text.contains("fake-access-token"));
    assert!(!text.contains("fake-refresh-token"));
}
