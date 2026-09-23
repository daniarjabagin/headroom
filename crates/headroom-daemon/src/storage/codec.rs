use std::path::{Path, PathBuf};

use headroom_core::units::Tokens;
use jiff::Timestamp;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::StorageError;

pub fn timestamp_to_sql(at: Timestamp) -> Result<i64, StorageError> {
    i64::try_from(at.as_nanosecond()).map_err(|_| StorageError::OutOfRange("timestamp"))
}

pub fn timestamp_from_sql(nanos: i64) -> Result<Timestamp, StorageError> {
    Ok(Timestamp::from_nanosecond(i128::from(nanos))?)
}

pub fn tokens_to_sql(tokens: Tokens) -> Result<i64, StorageError> {
    i64::try_from(tokens.0).map_err(|_| StorageError::OutOfRange("token count"))
}

pub fn tokens_from_sql(value: i64) -> Result<Tokens, StorageError> {
    u64::try_from(value)
        .map(Tokens)
        .map_err(|_| StorageError::OutOfRange("token count"))
}

pub fn path_to_sql(path: &Path) -> Result<&str, StorageError> {
    path.to_str()
        .ok_or_else(|| StorageError::NonUtf8Path(path.to_path_buf()))
}

pub fn path_from_sql(text: String) -> PathBuf {
    PathBuf::from(text)
}

pub fn enum_to_sql<T: Serialize>(value: &T) -> Result<String, StorageError> {
    match serde_json::to_value(value)? {
        serde_json::Value::String(text) => Ok(text),
        other => Err(StorageError::UnknownValue {
            field: "enum",
            value: other.to_string(),
        }),
    }
}

pub fn enum_from_sql<T: DeserializeOwned>(
    field: &'static str,
    text: String,
) -> Result<T, StorageError> {
    serde_json::from_value(serde_json::Value::String(text.clone()))
        .map_err(|_| StorageError::UnknownValue { field, value: text })
}

#[cfg(test)]
mod tests {
    use headroom_core::account::ProviderId;

    use super::*;
    use crate::testing::{CLAUDE, CODEX};

    #[test]
    fn timestamps_round_trip_with_nanoseconds() {
        let at: Timestamp = "2026-09-23T10:00:00.123456789Z".parse().unwrap();
        assert_eq!(
            timestamp_from_sql(timestamp_to_sql(at).unwrap()).unwrap(),
            at
        );
    }

    #[test]
    fn tokens_above_i64_are_rejected() {
        assert!(matches!(
            tokens_to_sql(Tokens(u64::MAX)),
            Err(StorageError::OutOfRange(_))
        ));
        assert!(tokens_from_sql(-1).is_err());
    }

    #[test]
    fn enums_use_their_serde_names() {
        assert_eq!(enum_to_sql(&CLAUDE).unwrap(), "claude");
        let id: ProviderId = enum_from_sql("provider", "codex".into()).unwrap();
        assert_eq!(id, CODEX);
        let unknown: ProviderId = enum_from_sql("provider", "cursor".into()).unwrap();
        assert_eq!(unknown.as_str(), "cursor");
        assert!(matches!(
            enum_from_sql::<ProviderId>("provider", "Not An Id".into()),
            Err(StorageError::UnknownValue {
                field: "provider",
                ..
            })
        ));
    }
}
