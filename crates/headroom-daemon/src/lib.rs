mod activity;
pub mod catalog;
pub mod clock;
mod commands;
pub mod config;
pub mod core;
mod credentials;
mod daemon;
#[cfg(target_os = "linux")]
pub mod dbus;
mod dismissal;
pub mod dismissed;
pub mod error;
pub mod events;
pub mod home;
pub mod ipc;
pub mod model;
pub mod notify;
mod once;
pub mod quota_history;
pub mod random;
pub mod registry;
pub mod rescan;
pub mod scheduler;
pub mod service;
pub mod settings;
pub mod state;
pub mod status;
pub mod storage;
pub mod update;
pub mod usage;

#[cfg(test)]
mod testing;

pub use commands::MAX_LABEL_CHARS;
#[cfg(target_os = "linux")]
pub use config::BusTarget;
pub use config::{DaemonConfig, Shutdown, app_dir, default_db_path};
pub use daemon::diagnostics::{self, Diagnostics};
pub use daemon::log_level::{EffectiveLevel, LevelSource, LogControl, LogStatus};
pub use daemon::run;
pub use daemon::system_info::{self, SystemInfo};
pub use error::{DaemonError, SocketError};
pub use ipc::{SOCKET_ENV, default_socket_path};
pub use once::{OnceContext, assemble_state_once};
pub use state::payload::StatePayload;
