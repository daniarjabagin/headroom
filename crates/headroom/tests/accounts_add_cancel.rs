#![cfg(test)]

use std::fs::{self, Permissions};
use std::io::{BufRead, BufReader, Lines};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use headroom_providers::paths::HeadroomDirs;
use rustix::process::{Pid, Signal, kill_process};
use tempfile::TempDir;

const HANGING_CODEX: &str = "#!/bin/sh
PATH=/usr/bin:/bin
sleep 300 &
echo $! > \"$PIDS/grandchild\"
echo $$ > \"$PIDS/child\"
wait
";

struct Sandbox {
    dir: TempDir,
}

impl Sandbox {
    fn new() -> Sandbox {
        let sandbox = Sandbox {
            dir: tempfile::tempdir().unwrap(),
        };
        for sub in ["bin", "pids", "data", "home"] {
            fs::create_dir(sandbox.path(sub)).unwrap();
        }
        let codex = sandbox.path("bin").join("codex");
        fs::write(&codex, HANGING_CODEX).unwrap();
        fs::set_permissions(&codex, Permissions::from_mode(0o755)).unwrap();
        sandbox
    }

    fn path(&self, sub: &str) -> PathBuf {
        self.dir.path().join(sub)
    }

    fn spawn_add(&self) -> Child {
        let bus = format!("unix:path={}", self.path("no-bus").display());
        Command::new(env!("CARGO_BIN_EXE_headroom"))
            .args(["accounts", "add", "codex"])
            .args(["--progress", "json"])
            .env_clear()
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.path("bin").display()),
            )
            .env("HOME", self.path("home"))
            .env("XDG_DATA_HOME", self.path("data"))
            .env("DBUS_SESSION_BUS_ADDRESS", &bus)
            .env("PIDS", self.path("pids"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }

    fn pid(&self, name: &str) -> i32 {
        let path = self.path("pids").join(name);
        let text = wait_until(|| fs::read_to_string(&path).ok().filter(|t| t.ends_with('\n')));
        text.trim().parse().unwrap()
    }

    fn account_homes(&self) -> Vec<PathBuf> {
        let data = self.path("data");
        let dirs = HeadroomDirs::from_vars(&self.path("home"), |name| {
            (name == "XDG_DATA_HOME").then(|| data.clone().into_os_string())
        });
        match fs::read_dir(dirs.accounts("codex")) {
            Ok(entries) => entries.map(|entry| entry.unwrap().path()).collect(),
            Err(_) => Vec::new(),
        }
    }
}

fn wait_until<T>(mut probe: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(value) = probe() {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out");
        thread::sleep(Duration::from_millis(25));
    }
}

fn running(pid: i32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            stat.rsplit_once(") ")
                .map(|(_, rest)| !rest.starts_with('Z'))
        })
        .unwrap_or(false)
}

fn read_until_started(lines: &mut Lines<BufReader<ChildStdout>>) -> PathBuf {
    for line in lines.by_ref() {
        let event: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        if event["event"] == "started" {
            return PathBuf::from(event["home"].as_str().unwrap());
        }
    }
    panic!("no started event");
}

fn signal(child: &Child, signal: Signal) {
    let pid = Pid::from_raw(i32::try_from(child.id()).unwrap()).unwrap();
    kill_process(pid, signal).unwrap();
}

fn assert_cleaned_up(sandbox: &Sandbox, home: &Path) {
    let child = sandbox.pid("child");
    let grandchild = sandbox.pid("grandchild");
    wait_until(|| (!running(child) && !running(grandchild)).then_some(()));
    assert!(!home.exists(), "{} was left behind", home.display());
    assert!(sandbox.account_homes().is_empty());
}

fn cancelled_by(kind: Signal) {
    let sandbox = Sandbox::new();
    let mut headroom = sandbox.spawn_add();
    let mut lines = BufReader::new(headroom.stdout.take().unwrap()).lines();
    let home = read_until_started(&mut lines);
    assert!(home.is_dir());
    sandbox.pid("grandchild");
    signal(&headroom, kind);
    let rest: Vec<String> = lines.map(Result::unwrap).collect();
    let status = headroom.wait().unwrap();
    assert!(!status.success());
    assert_eq!(
        rest.last().map(String::as_str),
        Some(r#"{"event":"error","message":"cancelled"}"#)
    );
    assert_cleaned_up(&sandbox, &home);
}

#[test]
fn sigterm_kills_the_login_tree_and_removes_the_home() {
    cancelled_by(Signal::TERM);
}

#[test]
fn sigint_kills_the_login_tree_and_removes_the_home() {
    cancelled_by(Signal::INT);
}

#[test]
fn closing_stdout_cancels_the_login() {
    let sandbox = Sandbox::new();
    let mut headroom = sandbox.spawn_add();
    let mut lines = BufReader::new(headroom.stdout.take().unwrap()).lines();
    let home = read_until_started(&mut lines);
    sandbox.pid("grandchild");
    drop(lines);
    let status = wait_until(|| headroom.try_wait().unwrap());
    assert!(!status.success());
    assert_cleaned_up(&sandbox, &home);
}
