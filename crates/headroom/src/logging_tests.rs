use std::fs;

use super::*;

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Captured {
    fn layer(&self) -> Output {
        let target = self.clone();
        fmt::layer()
            .with_writer(move || target.clone())
            .with_ansi(false)
            .boxed()
    }

    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

#[test]
fn the_settings_level_is_applied_without_a_restart() {
    let captured = Captured::default();
    let (logging, subscriber) = daemon_subscriber(None, captured.layer(), None);
    tracing::subscriber::with_default(subscriber, || {
        tracing::debug!(target: "headroom_daemon", "before-debug");
        tracing::info!(target: "headroom_daemon", "default-info");
        logging.apply(LogLevel::Debug);
        tracing::debug!(target: "headroom_daemon", "after-debug");
        tracing::debug!(target: "hyper::proto", "dependency-debug");
        logging.apply(LogLevel::Error);
        tracing::warn!(target: "headroom_daemon", "quiet-warn");
        tracing::error!(target: "headroom_daemon", "loud-error");
    });
    let text = captured.text();
    for shown in ["default-info", "after-debug", "loud-error"] {
        assert!(text.contains(shown), "{shown} missing from {text}");
    }
    for hidden in ["before-debug", "dependency-debug", "quiet-warn"] {
        assert!(!text.contains(hidden), "{hidden} leaked into {text}");
    }
    let status = logging.status();
    assert_eq!(status.level, EffectiveLevel::Error);
    assert_eq!(status.source, LevelSource::Settings);
    assert_eq!(status.file, None);
}

#[test]
fn rust_log_wins_over_the_setting() {
    let captured = Captured::default();
    let (logging, subscriber) =
        daemon_subscriber(Some("headroom_daemon=trace"), captured.layer(), None);
    tracing::subscriber::with_default(subscriber, || {
        logging.apply(LogLevel::Error);
        tracing::trace!(target: "headroom_daemon", "env-trace");
    });
    assert!(captured.text().contains("env-trace"));
    let status = logging.status();
    assert_eq!(status.level, EffectiveLevel::Trace);
    assert_eq!(status.source, LevelSource::Env);
}

#[test]
fn an_empty_or_invalid_rust_log_leaves_the_setting_in_charge() {
    for env in [None, Some(""), Some("  "), Some("headroom=[")] {
        let (_, mode) = initial_filter(env);
        assert!(matches!(mode, Mode::Settings(_)), "{env:?}");
    }
}

#[test]
fn records_also_go_to_the_log_file() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("headroom").join("headroom.log");
    let sink = RotatingFile::open(&path, LOG_CAP_BYTES).unwrap();
    let captured = Captured::default();
    let (logging, subscriber) = daemon_subscriber(None, captured.layer(), Some(sink));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(target: "headroom_daemon", "to-both");
    });
    assert!(captured.text().contains("to-both"));
    let written = fs::read_to_string(&path).unwrap();
    assert!(written.contains("to-both"));
    assert!(!written.contains('\u{1b}'));
    assert_eq!(logging.status().file, Some(path));
}

#[test]
fn debug_is_limited_to_headroom_crates() {
    assert_eq!(directives(LogLevel::Info), "info");
    assert_eq!(directives(LogLevel::Error), "error");
    assert_eq!(
        directives(LogLevel::Debug),
        "info,headroom=debug,headroom_core=debug,headroom_daemon=debug,\
         headroom_pricing=debug,headroom_providers=debug"
    );
}

#[test]
fn filter_hints_map_to_levels() {
    assert_eq!(effective_level(None), EffectiveLevel::Trace);
    assert_eq!(effective_level(Some(LevelFilter::OFF)), EffectiveLevel::Off);
    assert_eq!(
        effective_level(Some(LevelFilter::WARN)),
        EffectiveLevel::Warn
    );
    assert_eq!(
        effective_level(Some(LevelFilter::TRACE)),
        EffectiveLevel::Trace
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn the_log_lives_in_the_state_directory() {
    assert_eq!(
        log_path_in(Path::new("/home/ada/.local/state")),
        PathBuf::from("/home/ada/.local/state/headroom/headroom.log")
    );
}

#[cfg(target_os = "macos")]
#[test]
fn the_log_lives_in_the_user_logs_folder() {
    assert_eq!(
        macos_log_path(Path::new("/Users/ada")),
        PathBuf::from("/Users/ada/Library/Logs/Headroom/headroom.log")
    );
}
