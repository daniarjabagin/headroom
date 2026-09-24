use std::ffi::OsStr;

use gtk::gio;
use gtk::gio::prelude::*;
use gtk::glib;

use crate::update::{ProgressEvent, parse_progress};

const UPDATE_ARGS: [&str; 5] = ["headroom", "update", "--yes", "--progress", "json"];

fn spawn(command: &[&str], flags: gio::SubprocessFlags) -> Result<gio::Subprocess, glib::Error> {
    let argv: Vec<&OsStr> = command.iter().map(OsStr::new).collect();
    gio::Subprocess::newv(&argv, flags)
}

pub fn launch(command: &[String]) {
    let argv: Vec<&str> = command.iter().map(String::as_str).collect();
    match spawn(&argv, gio::SubprocessFlags::NONE) {
        Ok(process) => process.wait_async(gio::Cancellable::NONE, |result| {
            if let Err(error) = result {
                tracing::warn!(%error, "the sign-in terminal failed");
            }
        }),
        Err(error) => tracing::warn!(%error, "could not open a terminal for signing in"),
    }
}

async fn read_events(process: &gio::Subprocess, on_event: &impl Fn(ProgressEvent)) {
    let Some(stdout) = process.stdout_pipe() else {
        return;
    };
    let lines = gio::DataInputStream::new(&stdout);
    while let Ok(Some(line)) = lines.read_line_utf8_future(glib::Priority::DEFAULT).await {
        if let Some(event) = parse_progress(&line) {
            on_event(event);
        }
    }
}

pub fn run_update(
    on_event: impl Fn(ProgressEvent) + 'static,
    on_exit: impl FnOnce(Option<String>) + 'static,
) {
    let process = match spawn(&UPDATE_ARGS, gio::SubprocessFlags::STDOUT_PIPE) {
        Ok(process) => process,
        Err(error) => {
            on_exit(Some(error.to_string()));
            return;
        }
    };
    glib::spawn_future_local(async move {
        read_events(&process, &on_event).await;
        let failure = process
            .wait_check_future()
            .await
            .err()
            .map(|error| error.to_string());
        on_exit(failure);
    });
}
