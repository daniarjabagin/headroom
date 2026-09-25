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
    #[error("could not restrict the permissions of {path}: {source}")]
    Restrict {
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
    #[error("panel limits need non-empty account and window ids")]
    BlankPanelLimit,
    #[error("at most {max} panel limits are allowed, got {found}")]
    TooManyPanelLimits { found: usize, max: usize },
    #[error("starred accounts need non-empty ids")]
    BlankStarredAccount,
    #[error("notification threshold {0}% is outside 1..=50")]
    ThresholdPercent(u8),
    #[error("provider thresholds need non-empty provider ids")]
    BlankProviderThreshold,
    #[error("notification threshold {value}% for {provider} is outside 0..=50")]
    ProviderThreshold { provider: String, value: u8 },
    #[error("quiet hours time {0:?} is not HH:MM between 00:00 and 23:59")]
    ClockTime(String),
    #[error("enabled quiet hours need different start and end times")]
    EmptyQuietHours,
    #[error("shortcut is longer than {0} characters")]
    ShortcutTooLong(usize),
    #[error("shortcut {0:?} is not a GTK accelerator such as <Super>u")]
    InvalidShortcut(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SpendQueryError {
    #[error("spend query is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("a spend query needs either a period or a since date")]
    MissingRange,
    #[error("a spend query takes a period or a since date, not both")]
    PeriodAndSince,
    #[error("until needs a since date")]
    UntilWithoutSince,
    #[error("until {until} is before since {since}")]
    UntilBeforeSince {
        since: jiff::civil::Date,
        until: jiff::civil::Date,
    },
    #[error("{0} is in the future")]
    Future(jiff::civil::Date),
    #[error("Headroom keeps usage for recent days only; the earliest since date is {0}")]
    BeyondRetention(jiff::civil::Date),
    #[error("date range is out of bounds: {0}")]
    Range(#[from] jiff::Error),
    #[error("unknown provider: {0}")]
    UnknownProvider(String),
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
    #[error("label contains control characters")]
    LabelControlCharacters,
    #[error(transparent)]
    Settings(#[from] SettingsError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("could not encode JSON: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("the daemon is shutting down")]
    Stopping,
    #[error("update checks are turned off for this daemon (--no-update-check)")]
    UpdateChecksUnavailable,
    #[error(transparent)]
    SpendQuery(#[from] SpendQueryError),
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
            | CommandError::LabelControlCharacters
            | CommandError::Settings(_)
            | CommandError::SpendQuery(_) => true,
            CommandError::Storage(_)
            | CommandError::Encode(_)
            | CommandError::Stopping
            | CommandError::UpdateChecksUnavailable => false,
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
