use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;

use crate::core::Core;
use crate::settings::LogLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectiveLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LevelSource {
    Settings,
    Env,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogStatus {
    pub level: EffectiveLevel,
    pub source: LevelSource,
    pub file: Option<PathBuf>,
}

pub trait LogControl: Send + Sync {
    fn apply(&self, level: LogLevel);
    fn status(&self) -> LogStatus;
}

impl From<LogLevel> for EffectiveLevel {
    fn from(level: LogLevel) -> EffectiveLevel {
        match level {
            LogLevel::Error => EffectiveLevel::Error,
            LogLevel::Warn => EffectiveLevel::Warn,
            LogLevel::Info => EffectiveLevel::Info,
            LogLevel::Debug => EffectiveLevel::Debug,
        }
    }
}

impl EffectiveLevel {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            EffectiveLevel::Off => "off",
            EffectiveLevel::Error => "error",
            EffectiveLevel::Warn => "warn",
            EffectiveLevel::Info => "info",
            EffectiveLevel::Debug => "debug",
            EffectiveLevel::Trace => "trace",
        }
    }
}

impl LevelSource {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            LevelSource::Settings => "settings",
            LevelSource::Env => "RUST_LOG",
        }
    }
}

pub async fn follow(core: Arc<Core>, control: Arc<dyn LogControl>) {
    let mut changes = core.settings_changes();
    loop {
        let level = core.model().settings.logging.level;
        control.apply(level);
        if changes.changed().await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::testing::{eventually, harness};

    #[derive(Default)]
    struct RecordingControl {
        applied: Mutex<Vec<LogLevel>>,
    }

    impl LogControl for RecordingControl {
        fn apply(&self, level: LogLevel) {
            self.applied.lock().unwrap().push(level);
        }

        fn status(&self) -> LogStatus {
            LogStatus {
                level: EffectiveLevel::Info,
                source: LevelSource::Settings,
                file: None,
            }
        }
    }

    impl RecordingControl {
        fn last(&self) -> Option<LogLevel> {
            self.applied.lock().unwrap().last().copied()
        }
    }

    #[tokio::test]
    async fn the_stored_level_is_applied_at_start_and_after_every_change() {
        let harness = harness(Vec::new()).await;
        let control = Arc::new(RecordingControl::default());
        tokio::spawn(follow(harness.core.clone(), control.clone()));
        eventually(|| control.last() == Some(LogLevel::Info)).await;
        harness
            .core
            .update_settings(r#"{"logging":{"level":"debug"}}"#)
            .await
            .unwrap();
        eventually(|| control.last() == Some(LogLevel::Debug)).await;
        harness.core.reset_settings().await.unwrap();
        eventually(|| control.last() == Some(LogLevel::Info)).await;
    }

    #[test]
    fn levels_name_themselves_like_the_settings() {
        assert_eq!(EffectiveLevel::from(LogLevel::Warn).as_str(), "warn");
        assert_eq!(EffectiveLevel::from(LogLevel::Debug).as_str(), "debug");
        assert_eq!(
            serde_json::to_value(LevelSource::Env).unwrap(),
            serde_json::json!("env")
        );
    }
}
