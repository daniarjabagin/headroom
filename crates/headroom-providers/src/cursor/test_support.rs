use std::fs;
use std::path::Path;

use jiff::Timestamp;
use rusqlite::Connection;
use serde_json::json;

use super::config::CursorConfig;
use super::jwt::unsigned_token;

pub(super) fn fixed_now() -> Timestamp {
    "2026-09-23T10:00:00Z".parse().unwrap()
}

pub(super) fn token(subject: &str, expires_at: Timestamp) -> String {
    unsigned_token(&json!({ "sub": subject, "exp": expires_at.as_second() }))
}

pub(super) fn valid_token(subject: &str) -> String {
    token(subject, fixed_now() + jiff::SignedDuration::from_hours(24))
}

pub(super) fn write_state_db(config: &CursorConfig, rows: &[(&str, &str)]) {
    let path = config.state_db();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)",
        )
        .unwrap();
    for (key, value) in rows {
        connection
            .execute("INSERT INTO ItemTable VALUES (?1, ?2)", [key, value])
            .unwrap();
    }
}

pub(super) fn sign_in_ide(config: &CursorConfig, token: &str, membership: &str) {
    write_state_db(
        config,
        &[
            ("cursorAuth/accessToken", token),
            ("cursorAuth/refreshToken", "refresh-not-used"),
            ("cursorAuth/stripeMembershipType", membership),
            ("cursorAuth/cachedEmail", "someone@example.com"),
        ],
    );
}

pub(super) fn sign_in_agent(config: &CursorConfig, token: &str) {
    write_file(
        &config.agent_auth_file(),
        &json!({ "accessToken": token, "refreshToken": "refresh-not-used" }).to_string(),
    );
}

pub(super) fn write_file(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

pub(super) fn config_in(home: &Path) -> CursorConfig {
    CursorConfig::for_home(home)
}
