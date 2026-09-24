#![cfg(test)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};
use tempfile::TempDir;

const STATE: &str = include_str!("../src/render/fixtures/state_full.json");

const PUSH_DELAY: Duration = Duration::from_millis(300);

type Answer = dyn Fn(&Value) -> Vec<Reply> + Send + Sync;

enum Reply {
    Now(String),
    Later(String),
}

struct FakeDaemon {
    path: PathBuf,
    requests: Arc<Mutex<Vec<Value>>>,
}

impl FakeDaemon {
    fn start(dir: &Path, answer: Arc<Answer>) -> FakeDaemon {
        let path = dir.join("daemon.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let (recorded, answer) = (recorded.clone(), answer.clone());
                thread::spawn(move || serve(stream.unwrap(), &recorded, answer.as_ref()));
            }
        });
        FakeDaemon { path, requests }
    }

    fn methods(&self) -> Vec<(String, Value)> {
        let requests = self.requests.lock().unwrap();
        requests
            .iter()
            .map(|r| {
                (
                    r["method"].as_str().unwrap().to_owned(),
                    r["params"].clone(),
                )
            })
            .collect()
    }
}

fn serve(stream: UnixStream, recorded: &Mutex<Vec<Value>>, answer: &Answer) {
    let mut writer = stream.try_clone().unwrap();
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { return };
        let request: Value = serde_json::from_str(&line).unwrap();
        recorded.lock().unwrap().push(request.clone());
        for reply in answer(&request) {
            let line = match reply {
                Reply::Now(line) => line,
                Reply::Later(line) => {
                    thread::sleep(PUSH_DELAY);
                    line
                }
            };
            if writeln!(writer, "{line}").is_err() {
                return;
            }
        }
    }
}

fn compact_state() -> String {
    serde_json::from_str::<Value>(STATE).unwrap().to_string()
}

fn result(request: &Value, result: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{},"result":{result}}}"#,
        request["id"]
    )
}

fn daemon_answer(request: &Value) -> Vec<Reply> {
    let line = match request["method"].as_str() {
        Some("GetState") => result(request, &compact_state()),
        Some("Refresh") if request["params"][0] == "codex:zz" => json!({
            "jsonrpc": "2.0",
            "id": request["id"],
            "error": {"code": -32602, "message": "unknown account: codex:zz"}
        })
        .to_string(),
        _ => result(request, "null"),
    };
    vec![Reply::Now(line)]
}

fn short_sandbox() -> TempDir {
    tempfile::Builder::new()
        .prefix("hr")
        .tempdir_in("/tmp")
        .unwrap()
}

fn command(sandbox: &TempDir, socket: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_headroom"));
    command
        .args(["--db", &sandbox.path().join("h.db").display().to_string()])
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", sandbox.path().join("home"))
        .env("XDG_CACHE_HOME", sandbox.path().join("cache"))
        .env("XDG_DATA_HOME", sandbox.path().join("data"))
        .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/bus")
        .env("HEADROOM_SOCKET", socket)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn headroom(sandbox: &TempDir, socket: &Path, args: &[&str]) -> Output {
    command(sandbox, socket, args).output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[test]
fn status_json_prints_the_state_exactly_as_served() {
    let sandbox = short_sandbox();
    let daemon = FakeDaemon::start(sandbox.path(), Arc::new(daemon_answer));
    let output = headroom(&sandbox, &daemon.path, &["status", "--json"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), compact_state() + "\n");
    assert_eq!(daemon.methods(), [("GetState".to_owned(), json!([]))]);
}

#[test]
fn commands_send_their_arguments_in_order() {
    let sandbox = short_sandbox();
    let daemon = FakeDaemon::start(sandbox.path(), Arc::new(daemon_answer));
    let refreshed = headroom(&sandbox, &daemon.path, &["refresh", "codex:a"]);
    assert_eq!(text(&refreshed.stdout), "Refresh requested for codex:a\n");
    let now = headroom(&sandbox, &daemon.path, &["refresh", "--now"]);
    assert!(now.status.success());
    let ordered = headroom(&sandbox, &daemon.path, &["accounts", "order", "b", "a"]);
    assert!(ordered.status.success());
    let hidden = headroom(&sandbox, &daemon.path, &["accounts", "hide", "codex:a"]);
    assert!(hidden.status.success());
    let restored = headroom(&sandbox, &daemon.path, &["accounts", "restore"]);
    assert!(restored.status.success(), "{}", text(&restored.stderr));
    assert_eq!(
        daemon.methods(),
        [
            ("Refresh".to_owned(), json!(["codex:a"])),
            ("RefreshNow".to_owned(), json!([])),
            ("SetAccountOrder".to_owned(), json!([["b", "a"]])),
            ("SetAccountHidden".to_owned(), json!(["codex:a", true])),
            ("RestoreAccounts".to_owned(), json!([""])),
        ]
    );
}

#[test]
fn daemon_errors_are_shown_as_their_message() {
    let sandbox = short_sandbox();
    let daemon = FakeDaemon::start(sandbox.path(), Arc::new(daemon_answer));
    let output = headroom(&sandbox, &daemon.path, &["refresh", "codex:zz"]);
    assert!(!output.status.success());
    assert_eq!(
        text(&output.stderr),
        "headroom: unknown account: codex:zz\n"
    );
}

#[test]
fn a_missing_socket_means_the_daemon_is_not_running() {
    let sandbox = short_sandbox();
    let socket = sandbox.path().join("absent.sock");
    let output = headroom(&sandbox, &socket, &["refresh"]);
    assert!(!output.status.success());
    assert_eq!(
        text(&output.stderr),
        format!(
            "headroom: the Headroom daemon is not listening on {}\n",
            socket.display()
        )
    );
    let status = headroom(&sandbox, &socket, &["status"]);
    assert!(text(&status.stderr).contains("no cached data yet"));
}

fn waybar_answer(request: &Value) -> Vec<Reply> {
    let mut replies = daemon_answer(request);
    if request["method"] == "GetState" {
        let mut state: Value = serde_json::from_str(STATE).unwrap();
        state["headline"] = Value::Null;
        let params = json!({"state": state});
        let pushed = json!({"jsonrpc": "2.0", "method": "StateChanged", "params": params});
        replies.push(Reply::Later(pushed.to_string()));
    }
    replies
}

fn read_lines(child: &mut Child, count: usize) -> Vec<Value> {
    let stdout = child.stdout.take().unwrap();
    BufReader::new(stdout)
        .lines()
        .take(count)
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect()
}

#[test]
fn waybar_subscribes_and_follows_state_changes() {
    let sandbox = short_sandbox();
    let daemon = FakeDaemon::start(sandbox.path(), Arc::new(waybar_answer));
    let mut child = command(&sandbox, &daemon.path, &["waybar"])
        .spawn()
        .unwrap();
    let lines = read_lines(&mut child, 2);
    child.kill().ok();
    child.wait().ok();
    assert_eq!(lines[0]["text"], "8%");
    assert_eq!(lines[1]["text"], "—");
    let methods: Vec<String> = daemon.methods().into_iter().map(|(m, _)| m).collect();
    assert_eq!(methods, ["Subscribe", "GetState"]);
}
