#![cfg(test)]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

fn headroom(sandbox: &TempDir, args: &[&str], stdin: &str) -> Output {
    let bus = format!("unix:path={}", sandbox.path().join("no-bus").display());
    let mut child = Command::new(env!("CARGO_BIN_EXE_headroom"))
        .arg("--bus-address")
        .arg(&bus)
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", sandbox.path().join("home"))
        .env("XDG_DATA_HOME", sandbox.path().join("data"))
        .env("DBUS_SESSION_BUS_ADDRESS", &bus)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn events(output: &Output) -> Vec<serde_json::Value> {
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn providers_json_is_the_list_providers_payload() {
    let sandbox = tempfile::tempdir().unwrap();
    let output = headroom(&sandbox, &["providers", "--json"], "");
    assert!(output.status.success());
    let listed: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/render/fixtures/providers.json");
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture).unwrap()).unwrap();
    assert_eq!(listed, expected);
    let table = headroom(&sandbox, &["providers"], "");
    assert!(stdout(&table).starts_with("ID "), "{}", stdout(&table));
}

#[test]
fn adding_an_unknown_provider_reports_an_error_event() {
    let sandbox = tempfile::tempdir().unwrap();
    let output = headroom(
        &sandbox,
        &["accounts", "add", "cursor", "--progress", "json"],
        "",
    );
    assert!(!output.status.success());
    let lines = events(&output);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["event"], "error");
    assert_eq!(
        lines[0]["message"],
        "unknown provider \"cursor\"; known providers: codex, claude, openrouter, zai"
    );
}

#[test]
fn keys_are_refused_for_providers_that_sign_in_with_a_cli() {
    let sandbox = tempfile::tempdir().unwrap();
    let args = [
        "accounts",
        "add",
        "codex",
        "--api-key-stdin",
        "--progress",
        "json",
    ];
    let output = headroom(&sandbox, &args, "sk-should-not-leak\n");
    assert!(!output.status.success());
    let text = stdout(&output) + &String::from_utf8_lossy(&output.stderr);
    assert!(!text.contains("sk-should-not-leak"));
    let lines = events(&output);
    assert_eq!(
        lines[0]["message"],
        "Codex accounts cannot be added with an API key"
    );
    assert!(!sandbox.path().join("data/headroom/accounts").exists());
}
