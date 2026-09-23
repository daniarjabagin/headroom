use std::fs;
use std::path::Path;

use jiff::Timestamp;
use serde_json::{Value, json};

use super::jwt::unsigned_token;

pub(super) const USER_ID: &str = "user-fake0001";
pub(super) const ACCOUNT_ID: &str = "acct-fake0001";
pub(super) const EMAIL: &str = "someone@example.com";

pub(super) fn at(text: &str) -> Timestamp {
    text.parse().unwrap()
}

pub(super) fn id_token() -> String {
    unsigned_token(&json!({
        "email": EMAIL,
        "sub": "auth0|fake",
        "https://api.openai.com/auth": {
            "chatgpt_account_id": ACCOUNT_ID,
            "chatgpt_user_id": USER_ID,
            "chatgpt_plan_type": "plus",
        }
    }))
}

pub(super) fn access_token(expires_at: Timestamp) -> String {
    unsigned_token(&json!({ "exp": expires_at.as_second() }))
}

pub(super) fn auth_document(access_token: &str) -> Value {
    json!({
        "OPENAI_API_KEY": null,
        "tokens": {
            "id_token": id_token(),
            "access_token": access_token,
            "refresh_token": "rt-fake",
            "account_id": ACCOUNT_ID,
        },
        "last_refresh": "2026-09-20T10:00:00Z",
    })
}

pub(super) fn write_auth(home: &Path, document: &Value) {
    fs::create_dir_all(home).unwrap();
    fs::write(home.join("auth.json"), document.to_string()).unwrap();
}

pub(super) fn write_rollout(home: &Path, relative: &str, content: &str) -> std::path::PathBuf {
    let path = home.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}
