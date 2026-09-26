#![cfg(test)]

use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

fn run_with_closed_stdout(sandbox: &Path, args: &[&str]) -> (ExitStatus, String) {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let mut child = Command::new(env!("CARGO_BIN_EXE_headroom"))
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", sandbox.join("home"))
        .env("XDG_DATA_HOME", sandbox.join("data"))
        .env("HEADROOM_SOCKET", sandbox.join("absent.sock"))
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    (child.wait().unwrap(), stderr)
}

#[test]
fn a_closed_stdout_ends_a_command_quietly() {
    let sandbox = tempfile::tempdir().unwrap();
    let (status, stderr) = run_with_closed_stdout(sandbox.path(), &["providers", "--json"]);
    assert!(status.success(), "{stderr}");
    assert_eq!(stderr, "");
}

#[test]
fn a_closed_stdout_ends_the_waybar_stream_quietly() {
    let sandbox = tempfile::tempdir().unwrap();
    let (status, stderr) = run_with_closed_stdout(sandbox.path(), &["waybar"]);
    assert!(status.success(), "{stderr}");
    assert_eq!(stderr, "");
}

fn wait_briefly(child: &mut Child) -> ExitStatus {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "waybar kept running"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn waybar_exits_once_its_reader_leaves_after_the_first_line() {
    let sandbox = tempfile::tempdir().unwrap();
    let (reader, writer) = std::io::pipe().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_headroom"))
        .arg("waybar")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", sandbox.path().join("home"))
        .env("HEADROOM_SOCKET", sandbox.path().join("absent.sock"))
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = String::new();
    BufReader::new(reader).read_line(&mut first).unwrap();
    assert!(first.starts_with('{'), "{first}");
    let status = wait_briefly(&mut child);
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(status.success(), "{stderr}");
    assert_eq!(stderr, "");
}
