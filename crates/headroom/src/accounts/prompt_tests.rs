use std::fs::{File, OpenOptions};
use std::os::fd::OwnedFd;
use std::time::Duration;

use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};

use super::*;

const LIMIT: Duration = Duration::from_secs(10);

struct Pty {
    master: OwnedFd,
    slave: File,
}

fn pty() -> Pty {
    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).unwrap();
    grantpt(&master).unwrap();
    unlockpt(&master).unwrap();
    let name = ptsname(&master, Vec::new()).unwrap();
    let slave = OpenOptions::new()
        .read(true)
        .write(true)
        .open(name.to_str().unwrap())
        .unwrap();
    Pty { master, slave }
}

fn echoes(tty: &File) -> bool {
    termios::tcgetattr(tty)
        .unwrap()
        .local_modes
        .contains(LocalModes::ECHO)
}

async fn echo_turned_off(tty: &File) {
    while echoes(tty) {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

fn type_once_echo_is_off(pty: &Pty, text: &'static [u8]) -> tokio::task::JoinHandle<OwnedFd> {
    let watcher = pty.slave.try_clone().unwrap();
    let master = pty.master.try_clone().unwrap();
    tokio::spawn(async move {
        echo_turned_off(&watcher).await;
        rustix::io::write(&master, text).unwrap();
        master
    })
}

async fn prompt(pty: &Pty, out: &mut Vec<u8>, cancel: &Cancel) -> Result<String> {
    let tty = pty.slave.try_clone().unwrap();
    tokio::time::timeout(LIMIT, read_hidden(tty, out, cancel))
        .await
        .unwrap()
}

#[tokio::test]
async fn the_key_is_read_without_echo_and_the_terminal_is_restored() {
    let pty = pty();
    assert!(echoes(&pty.slave));
    let typist = type_once_echo_is_off(&pty, b"oc_sk_fake_secret\n");
    let mut out = Vec::new();
    let key = prompt(&pty, &mut out, &Cancel::default()).await.unwrap();
    assert_eq!(key, "oc_sk_fake_secret");
    assert_eq!(out, b"Paste the API key: ");
    assert!(echoes(&pty.slave));
    let master = typist.await.unwrap();
    let mut echoed = [0_u8; 256];
    let read = rustix::io::read(&master, &mut echoed).unwrap();
    let echoed = String::from_utf8_lossy(&echoed[..read]);
    assert!(echoed.contains('\n'), "{echoed:?}");
    assert!(!echoed.contains("secret"), "{echoed:?}");
}

#[tokio::test]
async fn cancelling_restores_echo_and_ends_the_prompt_line() {
    let pty = pty();
    let cancel = Cancel::default();
    let remote = cancel.clone();
    let watcher = pty.slave.try_clone().unwrap();
    tokio::spawn(async move {
        echo_turned_off(&watcher).await;
        remote.cancel();
    });
    let mut out = Vec::new();
    let error = prompt(&pty, &mut out, &cancel).await.unwrap_err();
    assert_eq!(error.to_string(), CANCELLED);
    assert_eq!(out, b"Paste the API key: \n");
    assert!(echoes(&pty.slave));
}

#[tokio::test]
async fn an_empty_line_is_refused() {
    let pty = pty();
    let typist = type_once_echo_is_off(&pty, b"   \n");
    let mut out = Vec::new();
    let error = prompt(&pty, &mut out, &Cancel::default())
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "no API key entered");
    assert!(echoes(&pty.slave));
    typist.await.unwrap();
}

#[tokio::test]
async fn a_regular_file_is_not_a_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("key");
    std::fs::write(&path, "oc_sk_fake\n").unwrap();
    let mut out = Vec::new();
    let file = File::open(&path).unwrap();
    assert!(
        read_hidden(file, &mut out, &Cancel::default())
            .await
            .is_err()
    );
}
