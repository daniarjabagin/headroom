use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("could not create database directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("stored timestamp is invalid: {0}")]
    Timestamp(#[from] jiff::Error),
    #[error("value does not fit in the database: {0}")]
    OutOfRange(&'static str),
    #[error("unknown stored {field}: {value}")]
    UnknownValue { field: &'static str, value: String },
    #[error("path is not valid UTF-8: {0}")]
    NonUtf8Path(PathBuf),
    #[error("database schema version {found} is newer than supported version {supported}")]
    FutureSchema { found: usize, supported: usize },
    #[error("database lock poisoned")]
    Poisoned,
    #[error("database task failed: {0}")]
    Task(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("settings JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("refresh interval {0}s is outside 60..=3600")]
    RefreshInterval(u64),
    #[error("a pinned headline needs an account id and a window")]
    EmptyHeadlineTarget,
    #[error("hidden windows need non-empty account and window ids")]
    BlankHiddenWindow,
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("unknown account: {0}")]
    UnknownAccount(String),
    #[error("duplicate account in order: {0}")]
    DuplicateAccount(String),
    #[error("label is longer than {0} characters")]
    LabelTooLong(usize),
    #[error(transparent)]
    Settings(#[from] SettingsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("could not encode JSON: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("the daemon is shutting down")]
    Stopping,
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("D-Bus error: {0}")]
    Bus(#[from] zbus::Error),
    #[error("another Headroom daemon already owns the bus name")]
    AlreadyRunning,
    #[error("no XDG state directory is available")]
    NoStateDir,
    #[error("could not encode JSON: {0}")]
    Encode(#[from] serde_json::Error),
}
