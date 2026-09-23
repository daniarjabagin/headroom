use serde_json::json;

use super::super::test_support::{
    ACCOUNT_ID, EMAIL, USER_ID, access_token, at, auth_document, id_token, write_auth,
};
use super::*;

fn load(document: &Value) -> Result<Credentials, ProviderError> {
    let dir = tempfile::tempdir().unwrap();
    write_auth(dir.path(), document);
    load_credentials(dir.path())
}

#[test]
fn loads_identity_from_id_token() {
    let token = access_token(at("2026-09-24T00:00:00Z"));
    let credentials = load(&auth_document(&token)).unwrap();
    assert_eq!(credentials.access_token, token);
    assert_eq!(credentials.account_id.as_deref(), Some(ACCOUNT_ID));
    assert_eq!(
        credentials.identity,
        AccountIdentity {
            email: Some(EMAIL.into()),
            plan: Some("Plus".into()),
            stable_key: format!("{USER_ID}/{ACCOUNT_ID}"),
        }
    );
    assert_eq!(credentials.expires_at, Some(at("2026-09-24T00:00:00Z")));
}

#[test]
fn expired_access_token_is_sign_in_expired() {
    let credentials = load(&auth_document(&access_token(at("2026-09-23T09:00:00Z")))).unwrap();
    assert_eq!(
        credentials.ensure_fresh(at("2026-09-23T10:00:00Z")),
        Err(ProviderError::SignInExpired)
    );
    assert_eq!(credentials.ensure_fresh(at("2026-09-23T08:00:00Z")), Ok(()));
}

#[test]
fn opaque_access_token_is_assumed_fresh() {
    let credentials = load(&auth_document("opaque-token")).unwrap();
    assert_eq!(credentials.expires_at, None);
    assert_eq!(credentials.ensure_fresh(at("2030-01-01T00:00:00Z")), Ok(()));
}

#[test]
fn account_id_falls_back_to_stored_tokens_account_id() {
    let document = json!({ "tokens": { "access_token": "a", "account_id": "stored-acct" } });
    let credentials = load(&document).unwrap();
    assert_eq!(credentials.identity.stable_key, "/stored-acct");
    assert_eq!(credentials.account_id.as_deref(), Some("stored-acct"));
}

#[test]
fn api_key_only_and_signed_out_are_distinguished() {
    let api_key = json!({ "OPENAI_API_KEY": "sk-fake", "tokens": null });
    assert_eq!(load(&api_key), Err(ProviderError::ApiKeyOnly));
    let empty_token = json!({ "OPENAI_API_KEY": null, "tokens": { "access_token": " " } });
    assert_eq!(load(&empty_token), Err(ProviderError::NotSignedIn));
}

#[test]
fn missing_auth_file_is_not_signed_in() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        load_credentials(dir.path()),
        Err(ProviderError::NotSignedIn)
    );
}

#[test]
fn hex_encoded_document_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let document = auth_document("opaque-token").to_string();
    fs::write(
        dir.path().join("auth.json"),
        format!("{}\n", hex::encode(document)),
    )
    .unwrap();
    let credentials = load_credentials(dir.path()).unwrap();
    assert_eq!(
        credentials.identity.stable_key,
        format!("{USER_ID}/{ACCOUNT_ID}")
    );
}

#[test]
fn garbage_file_is_local_data_error_without_contents() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("auth.json"), "secret-ish garbage").unwrap();
    let Err(ProviderError::LocalData(message)) = load_credentials(dir.path()) else {
        panic!("expected a local data error");
    };
    assert!(!message.contains("secret-ish"));
}

#[test]
fn token_without_identity_is_rejected() {
    let document = json!({ "tokens": { "access_token": "a" } });
    assert!(matches!(load(&document), Err(ProviderError::LocalData(_))));
}

#[test]
fn debug_output_redacts_the_token() {
    let token = access_token(at("2026-09-24T00:00:00Z"));
    let credentials = load(&auth_document(&token)).unwrap();
    let debug = format!("{credentials:?}");
    assert!(!debug.contains(&token));
    assert!(!debug.contains(&id_token()));
    assert!(debug.contains("<redacted>"));
}
