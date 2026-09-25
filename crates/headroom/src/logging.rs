mod file;

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use headroom_daemon::settings::LogLevel;
use headroom_daemon::{EffectiveLevel, LevelSource, LogControl, LogStatus};
use tracing::{Level, Subscriber};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::{Layered, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt, reload};

use crate::cli::Command;
use file::{LOG_CAP_BYTES, RotatingFile};

const OWN_CRATES: [&str; 5] = [
    "headroom",
    "headroom_core",
    "headroom_daemon",
    "headroom_pricing",
    "headroom_providers",
];

type Base = Layered<reload::Layer<EnvFilter, Registry>, Registry>;
type Output = Box<dyn Layer<Base> + Send + Sync>;

pub struct DaemonLogging {
    reload: reload::Handle<EnvFilter, Registry>,
    mode: Mode,
    file: Option<PathBuf>,
}

enum Mode {
    Settings(Mutex<LogLevel>),
    Env(EffectiveLevel),
}

impl LogControl for DaemonLogging {
    fn apply(&self, level: LogLevel) {
        let Mode::Settings(current) = &self.mode else {
            return;
        };
        let mut current = current.lock().unwrap_or_else(PoisonError::into_inner);
        if *current == level {
            return;
        }
        match self.reload.reload(settings_filter(level)) {
            Ok(()) => {
                *current = level;
                tracing::info!(
                    level = EffectiveLevel::from(level).as_str(),
                    "log level set"
                );
            }
            Err(error) => tracing::warn!(%error, "could not change the log level"),
        }
    }

    fn status(&self) -> LogStatus {
        let (level, source) = match &self.mode {
            Mode::Settings(current) => {
                let level = *current.lock().unwrap_or_else(PoisonError::into_inner);
                (level.into(), LevelSource::Settings)
            }
            Mode::Env(level) => (*level, LevelSource::Env),
        };
        LogStatus {
            level,
            source,
            file: self.file.clone(),
        }
    }
}

pub fn init(command: &Command) -> Option<Arc<DaemonLogging>> {
    if matches!(command, Command::Daemon(_)) {
        Some(Arc::new(init_daemon()))
    } else {
        init_cli();
        None
    }
}

fn init_cli() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .with_ansi(io::stderr().is_terminal())
        .try_init();
}

fn init_daemon() -> DaemonLogging {
    let path = daemon_log_path();
    let opened = path
        .as_deref()
        .map(|path| RotatingFile::open(path, LOG_CAP_BYTES));
    let (sink, failure) = match opened {
        Some(Ok(sink)) => (Some(sink), None),
        Some(Err(error)) => (None, Some(error)),
        None => (None, None),
    };
    let env = std::env::var("RUST_LOG").ok();
    let journal = std::env::var_os("JOURNAL_STREAM").is_some();
    let (logging, subscriber) = daemon_subscriber(env.as_deref(), stderr_layer(journal), sink);
    if let Err(error) = subscriber.try_init() {
        let _ = writeln!(io::stderr(), "headroom: could not start logging: {error}");
    }
    report_file(path.as_deref(), failure);
    logging
}

fn report_file(path: Option<&Path>, failure: Option<io::Error>) {
    match (path, failure) {
        (Some(path), Some(error)) => {
            tracing::warn!(path = %path.display(), %error, "cannot open the log file; logging to stderr only");
        }
        (None, _) => tracing::warn!("no state directory; logging to stderr only"),
        (Some(_), None) => {}
    }
}

fn daemon_subscriber(
    env: Option<&str>,
    stderr: Output,
    sink: Option<RotatingFile>,
) -> (DaemonLogging, impl Subscriber + Send + Sync + 'static) {
    let (filter, mode) = initial_filter(env);
    let (filter, reload) = reload::Layer::new(filter);
    let file = sink.as_ref().map(|sink| sink.path().to_owned());
    let mut outputs = vec![stderr];
    outputs.extend(sink.map(file_layer));
    let subscriber = Registry::default().with(filter).with(outputs);
    (DaemonLogging { reload, mode, file }, subscriber)
}

fn stderr_layer(journal: bool) -> Output {
    let layer = fmt::layer()
        .with_writer(io::stderr)
        .with_ansi(io::stderr().is_terminal());
    if journal {
        layer.without_time().boxed()
    } else {
        layer.boxed()
    }
}

fn file_layer(sink: RotatingFile) -> Output {
    fmt::layer().with_writer(sink).with_ansi(false).boxed()
}

fn initial_filter(env: Option<&str>) -> (EnvFilter, Mode) {
    let from_env = env
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .and_then(|text| EnvFilter::try_new(text).ok());
    match from_env {
        Some(filter) => {
            let level = effective_level(filter.max_level_hint());
            (filter, Mode::Env(level))
        }
        None => (
            settings_filter(LogLevel::default()),
            Mode::Settings(Mutex::new(LogLevel::default())),
        ),
    }
}

fn settings_filter(level: LogLevel) -> EnvFilter {
    EnvFilter::builder().parse_lossy(directives(level))
}

fn directives(level: LogLevel) -> String {
    let own = EffectiveLevel::from(level).as_str();
    if level != LogLevel::Debug {
        return own.to_owned();
    }
    OWN_CRATES.iter().fold(String::from("info"), |text, name| {
        format!("{text},{name}={own}")
    })
}

fn effective_level(hint: Option<LevelFilter>) -> EffectiveLevel {
    let Some(hint) = hint else {
        return EffectiveLevel::Trace;
    };
    match hint.into_level() {
        None => EffectiveLevel::Off,
        Some(Level::ERROR) => EffectiveLevel::Error,
        Some(Level::WARN) => EffectiveLevel::Warn,
        Some(Level::INFO) => EffectiveLevel::Info,
        Some(Level::DEBUG) => EffectiveLevel::Debug,
        Some(_) => EffectiveLevel::Trace,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn daemon_log_path() -> Option<PathBuf> {
    Some(log_path_in(&dirs::state_dir()?))
}

#[cfg(target_os = "macos")]
pub fn daemon_log_path() -> Option<PathBuf> {
    Some(macos_log_path(&dirs::home_dir()?))
}

#[cfg(not(target_os = "macos"))]
fn log_path_in(state_dir: &Path) -> PathBuf {
    state_dir.join("headroom").join("headroom.log")
}

#[cfg(target_os = "macos")]
fn macos_log_path(home: &Path) -> PathBuf {
    home.join("Library/Logs/Headroom/headroom.log")
}

#[cfg(test)]
#[path = "logging_tests.rs"]
mod tests;
