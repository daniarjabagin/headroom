pub mod catalog;
pub mod clock;
mod commands;
pub mod config;
pub mod core;
mod daemon;
pub mod dbus;
mod dismissal;
pub mod dismissed;
pub mod error;
pub mod home;
pub mod model;
pub mod notify;
mod once;
pub mod random;
pub mod registry;
pub mod rescan;
pub mod scheduler;
pub mod settings;
pub mod state;
pub mod storage;
pub mod usage;

#[cfg(test)]
mod testing;

pub use commands::MAX_LABEL_CHARS;
pub use config::{BusTarget, DaemonConfig, Shutdown, default_db_path};
pub use daemon::run;
pub use error::DaemonError;
pub use once::{OnceContext, assemble_state_once};
pub use state::payload::StatePayload;
