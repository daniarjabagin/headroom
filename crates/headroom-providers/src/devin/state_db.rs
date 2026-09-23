use std::path::Path;
use std::time::Duration;

use headroom_core::provider::ProviderError;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Deserialize;
use serde_json::error::Category;

pub(super) const STATE_DB_FILE: &str = "state.vscdb";
const AUTH_STATUS_QUERY: &str = "SELECT value FROM ItemTable WHERE key = 'windsurfAuthStatus'";
const BUSY_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAuthStatus {
    api_key: Option<String>,
}

pub(super) fn read_app_key(path: &Path) -> Result<Option<String>, ProviderError> {
    let Some(value) = read_auth_status(path).map_err(|error| db_error(path, &error))? else {
        return Ok(None);
    };
    let status: RawAuthStatus = serde_json::from_slice(&value).map_err(|error| {
        ProviderError::LocalData(format!(
            "cannot parse the Devin app sign-in in {}: {}",
            path.display(),
            json_problem(&error)
        ))
    })?;
    Ok(status
        .api_key
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty()))
}

fn read_auth_status(path: &Path) -> rusqlite::Result<Option<Vec<u8>>> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connection = Connection::open_with_flags(path, flags)?;
    connection.busy_timeout(BUSY_TIMEOUT)?;
    connection
        .query_row(AUTH_STATUS_QUERY, [], |row| {
            Ok(match row.get_ref(0)? {
                ValueRef::Text(bytes) | ValueRef::Blob(bytes) => Some(bytes.to_vec()),
                _ => None,
            })
        })
        .optional()
        .map(Option::flatten)
}

fn db_error(path: &Path, error: &rusqlite::Error) -> ProviderError {
    let reason = match error {
        rusqlite::Error::SqliteFailure(failure, _) => format!("{:?}", failure.code),
        _ => "unexpected database contents".to_owned(),
    };
    ProviderError::LocalData(format!("cannot read {}: {reason}", path.display()))
}

fn json_problem(error: &serde_json::Error) -> &'static str {
    match error.classify() {
        Category::Io => "read error",
        Category::Syntax | Category::Eof => "not JSON",
        Category::Data => "unexpected fields",
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(in crate::devin) fn write_state_db(path: &Path, value: Option<&str>) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute(
                "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB)",
                [],
            )
            .unwrap();
        if let Some(value) = value {
            connection
                .execute(
                    "INSERT INTO ItemTable (key, value) VALUES ('windsurfAuthStatus', ?1)",
                    [value],
                )
                .unwrap();
        }
    }

    #[test]
    fn the_app_key_is_read_from_the_auth_status_row() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STATE_DB_FILE);
        write_state_db(&path, Some(r#"{"apiKey":" app-key-1 ","name":"x"}"#));
        assert_eq!(read_app_key(&path), Ok(Some("app-key-1".to_owned())));
    }

    #[test]
    fn a_missing_row_or_empty_key_means_signed_out() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty.vscdb");
        write_state_db(&empty, None);
        assert_eq!(read_app_key(&empty), Ok(None));
        let blank = dir.path().join("blank.vscdb");
        write_state_db(&blank, Some(r#"{"apiKey":""}"#));
        assert_eq!(read_app_key(&blank), Ok(None));
    }

    #[test]
    fn broken_values_and_non_databases_are_local_errors() {
        let dir = tempfile::tempdir().unwrap();
        let garbled = dir.path().join("garbled.vscdb");
        write_state_db(&garbled, Some("{not json"));
        assert!(matches!(
            read_app_key(&garbled),
            Err(ProviderError::LocalData(message)) if message.contains("not JSON")
        ));
        let text = dir.path().join("text.vscdb");
        std::fs::write(&text, "plain text, not sqlite").unwrap();
        assert!(matches!(
            read_app_key(&text),
            Err(ProviderError::LocalData(_))
        ));
    }
}
