use std::os::unix::fs::MetadataExt;

use super::*;
use crate::antigravity::test_support::fake_process;

#[test]
fn servers_are_found_with_their_listening_ports_best_first() {
    let root = tempfile::tempdir().unwrap();
    fake_process(
        root.path(),
        40,
        &["/home/someone/.local/bin/agy"],
        &[(50_001, 7)],
    );
    fake_process(
        root.path(),
        41,
        &[
            "/opt/Antigravity/bin/language_server_linux_x64",
            "--ide_name",
            "antigravity",
            "--csrf_token",
            "tok",
            "--extension_server_port",
            "50100",
        ],
        &[(50_002, 8), (50_003, 9)],
    );
    fake_process(root.path(), 42, &["/usr/bin/bash"], &[(22, 10)]);
    fs::create_dir_all(root.path().join("self")).unwrap();
    let servers = language_servers(root.path(), owner(root.path()));
    assert_eq!(servers.len(), 2);
    assert_eq!(servers[0].rank, Rank::NamedApp);
    assert_eq!(servers[0].csrf.expose(), "tok");
    assert!(!format!("{:?}", servers[0]).contains("tok"));
    assert_eq!(servers[0].ports, BTreeSet::from([50_002, 50_003]));
    assert_eq!(servers[0].extension_port, Some(50_100));
    assert_eq!(servers[1].rank, Rank::Cli);
    assert_eq!(servers[1].ports, BTreeSet::from([50_001]));
}

fn owner(path: &Path) -> u32 {
    fs::metadata(path).unwrap().uid()
}

#[test]
fn processes_of_other_users_are_skipped() {
    let root = tempfile::tempdir().unwrap();
    fake_process(root.path(), 40, &["/usr/bin/agy"], &[(50_001, 7)]);
    let me = owner(root.path());
    assert_eq!(language_servers(root.path(), me).len(), 1);
    assert!(language_servers(root.path(), me.wrapping_add(1)).is_empty());
}

#[test]
fn a_server_without_any_port_is_skipped() {
    let root = tempfile::tempdir().unwrap();
    fake_process(root.path(), 7, &["/usr/bin/agy"], &[]);
    assert!(language_servers(root.path(), owner(root.path())).is_empty());
    assert!(language_servers(&root.path().join("missing"), owner(root.path())).is_empty());
}

#[test]
fn endpoints_try_https_then_http_then_the_extension_port() {
    let server = LanguageServer {
        rank: Rank::NamedApp,
        csrf: SecretString::new(String::new()),
        ports: BTreeSet::from([1000]),
        extension_port: Some(2000),
    };
    assert_eq!(
        server.endpoints(),
        [
            "https://127.0.0.1:1000",
            "http://127.0.0.1:1000",
            "http://127.0.0.1:2000"
        ]
    );
}

#[test]
fn an_antigravity_dir_under_gemini_means_installed() {
    let root = tempfile::tempdir().unwrap();
    let gemini = root.path().join(".gemini");
    assert!(!installed(&gemini));
    fs::create_dir_all(gemini.join("tmp")).unwrap();
    fs::write(gemini.join("antigravity.txt"), "").unwrap();
    assert!(!installed(&gemini));
    fs::create_dir_all(gemini.join("antigravity-cli")).unwrap();
    assert!(installed(&gemini));
}
