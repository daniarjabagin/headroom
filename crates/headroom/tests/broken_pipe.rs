#![cfg(test)]

use std::io::Read;
use std::process::{Command, Stdio};

#[test]
fn a_closed_stdout_is_an_error_not_a_panic() {
    let sandbox = tempfile::tempdir().unwrap();
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let mut child = Command::new(env!("CARGO_BIN_EXE_headroom"))
        .args(["providers", "--json"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", sandbox.path().join("home"))
        .env("XDG_DATA_HOME", sandbox.path().join("data"))
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
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(1), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}
