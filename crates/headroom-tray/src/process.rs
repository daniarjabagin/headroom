use std::cell::Cell;
use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::gio;
use gtk::gio::prelude::*;
use gtk::glib;

use crate::i18n::Lang;
use crate::preferences::flow::exit_message;

const PROGRAM: &str = "headroom";
const SIGTERM: i32 = 15;
const KILL_GRACE_SECONDS: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProcessError {
    #[error("the headroom command was not found")]
    Missing,
    #[error("headroom exited with status {0}")]
    Status(i32),
    #[error("{0}")]
    Io(String),
}

impl ProcessError {
    #[must_use]
    pub fn message(&self, lang: Lang) -> String {
        match self {
            ProcessError::Missing => lang
                .tr("Couldn't find the headroom command. Install Headroom or add it to your PATH.")
                .to_owned(),
            ProcessError::Status(status) => exit_message(lang, *status),
            ProcessError::Io(message) => message.clone(),
        }
    }
}

fn executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[must_use]
pub fn headroom_binary() -> Option<PathBuf> {
    glib::find_program_in_path(PROGRAM).or_else(|| {
        let local = glib::home_dir().join(".local").join("bin").join(PROGRAM);
        executable(&local).then_some(local)
    })
}

pub fn spawn_headroom(
    args: &[&str],
    flags: gio::SubprocessFlags,
) -> Result<gio::Subprocess, ProcessError> {
    let binary = headroom_binary().ok_or(ProcessError::Missing)?;
    let mut command: Vec<&OsStr> = vec![binary.as_os_str()];
    command.extend(args.iter().map(OsStr::new));
    gio::Subprocess::newv(&command, flags).map_err(|error| ProcessError::Io(error.to_string()))
}

async fn read_lines(process: &gio::Subprocess, on_line: &impl Fn(&str)) {
    let Some(stdout) = process.stdout_pipe() else {
        return;
    };
    let lines = gio::DataInputStream::new(&stdout);
    while let Ok(Some(line)) = lines.read_line_utf8_future(glib::Priority::DEFAULT).await {
        on_line(&line);
    }
}

async fn outcome(process: &gio::Subprocess) -> Result<(), ProcessError> {
    process
        .wait_future()
        .await
        .map_err(|error| ProcessError::Io(error.to_string()))?;
    if process.is_successful() {
        Ok(())
    } else if process.has_exited() {
        Err(ProcessError::Status(process.exit_status()))
    } else {
        Err(ProcessError::Status(process.term_sig()))
    }
}

#[derive(Clone)]
pub struct ProgressProcess {
    process: gio::Subprocess,
    finished: Rc<Cell<bool>>,
}

impl ProgressProcess {
    pub fn start(
        args: &[&str],
        on_line: impl Fn(&str) + 'static,
        on_exit: impl FnOnce(Result<(), ProcessError>) + 'static,
    ) -> Result<Self, ProcessError> {
        let flags = gio::SubprocessFlags::STDIN_PIPE
            | gio::SubprocessFlags::STDOUT_PIPE
            | gio::SubprocessFlags::STDERR_SILENCE;
        let process = spawn_headroom(args, flags)?;
        let finished = Rc::new(Cell::new(false));
        let (reader, done) = (process.clone(), Rc::clone(&finished));
        glib::spawn_future_local(async move {
            read_lines(&reader, &on_line).await;
            let result = outcome(&reader).await;
            done.set(true);
            on_exit(result);
        });
        Ok(Self { process, finished })
    }

    pub fn send_line(&self, text: &str) {
        self.write(text, false);
    }

    pub fn send_last_line(&self, text: &str) {
        self.write(text, true);
    }

    fn write(&self, text: &str, close: bool) {
        let Some(stdin) = self.process.stdin_pipe() else {
            return;
        };
        let bytes = format!("{text}\n").into_bytes();
        glib::spawn_future_local(async move {
            if let Err((_, error)) = stdin.write_all_future(bytes, glib::Priority::DEFAULT).await {
                tracing::warn!(%error, "could not write to headroom");
            }
            if close && let Err(error) = stdin.close_future(glib::Priority::DEFAULT).await {
                tracing::debug!(%error, "could not close the input of headroom");
            }
        });
    }

    pub fn cancel(&self) {
        if self.finished.get() || self.process.identifier().is_none() {
            return;
        }
        self.process.send_signal(SIGTERM);
        let process = self.process.clone();
        glib::timeout_add_seconds_local_once(KILL_GRACE_SECONDS, move || {
            if process.identifier().is_some() {
                process.force_exit();
            }
        });
    }
}
