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
    #[error("a settings patch must be a JSON object")]
    PatchNotObject,
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("unknown account: {0}")]
    UnknownAccount(String),
    #[error("duplicate account in order: {0}")]
    DuplicateAccount(String),
    #[error("unknown provider: {0}")]
    UnknownProvider(String),
    #[error("{0} is signed in through Headroom; remove it to delete its home instead")]
    NotDismissable(String),
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

impl CommandError {
    #[must_use]
    pub fn is_invalid_argument(&self) -> bool {
        match self {
            CommandError::UnknownAccount(_)
            | CommandError::DuplicateAccount(_)
            | CommandError::UnknownProvider(_)
            | CommandError::NotDismissable(_)
            | CommandError::LabelTooLong(_)
            | CommandError::Settings(_) => true,
            CommandError::Storage(_) | CommandError::Encode(_) | CommandError::Stopping => false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[cfg(target_os = "linux")]
    #[error("D-Bus error: {0}")]
    Bus(#[from] zbus::Error),
    #[error("another Headroom daemon already owns the bus name")]
    AlreadyRunning,
    #[error(transparent)]
    Socket(#[from] SocketError),
    #[error("no state or data directory is available")]
    NoStateDir,
    #[error("could not encode JSON: {0}")]
    Encode(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum SocketError {
    #[error("another Headroom daemon is already listening on {0}")]
    AlreadyListening(PathBuf),
    #[error("{0} exists and is not a socket")]
    NotASocket(PathBuf),
    #[error("refusing to use {path}: {issue}")]
    NotPrivate { path: PathBuf, issue: PrivacyIssue },
    #[error("could not {action} {path}: {source}")]
    Io {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrivacyIssue {
    #[error("it is not a directory")]
    NotADirectory,
    #[error("it is owned by uid {owner}, not by uid {uid}")]
    Owner { owner: u32, uid: u32 },
    #[error("its mode is {0:04o}, not 0700")]
    Mode(u32),
}
