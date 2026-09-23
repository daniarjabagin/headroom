use super::*;

fn args(line: &str) -> Vec<String> {
    line.split(' ').map(str::to_owned).collect()
}

#[test]
fn the_app_language_server_is_matched_by_its_ide_name() {
    let found = candidate(&args(
        "/opt/Antigravity/resources/bin/language_server_linux_x64 --ide_name antigravity \
         --csrf_token abc-123 --extension_server_port 41001",
    ))
    .unwrap();
    assert_eq!(
        found,
        Candidate {
            rank: Rank::NamedApp,
            csrf: "abc-123".into(),
            extension_port: Some(41_001),
        }
    );
}

#[test]
fn inline_flags_and_app_paths_are_understood() {
    let found = candidate(&args(
        "/usr/share/antigravity/bin/language_server --csrf_token=xyz",
    ))
    .unwrap();
    assert_eq!(found.rank, Rank::AppPath);
    assert_eq!(found.csrf, "xyz");
    assert_eq!(found.extension_port, None);
}

#[test]
fn neighbouring_products_are_not_antigravity() {
    for line in [
        "/opt/windsurf/language_server --ide_name windsurf --csrf_token a",
        "/opt/antigravity/language_server --ide_name antigravity-next --csrf_token a",
        "/opt/other/language_server --csrf_token a",
        "/usr/bin/language_server_helper_antigravity",
        "/usr/bin/python3 /opt/antigravity/agy",
        "",
    ] {
        assert_eq!(candidate(&args(line)), None, "{line}");
    }
}

#[test]
fn an_app_server_without_a_csrf_token_is_skipped() {
    let line = "/opt/antigravity/language_server --ide_name antigravity";
    assert_eq!(candidate(&args(line)), None);
}

#[test]
fn the_cli_needs_no_csrf_token_and_ranks_last() {
    let found = candidate(&args("/home/someone/.local/bin/agy")).unwrap();
    assert_eq!(found.rank, Rank::Cli);
    assert_eq!(found.csrf, "");
    assert!(Rank::NamedApp < Rank::AppPath && Rank::AppPath < Rank::Cli);
}

#[test]
fn cmdline_bytes_split_on_nul() {
    assert_eq!(
        argv(b"/bin/agy\0--flag\0value\0"),
        ["/bin/agy", "--flag", "value"]
    );
    assert!(argv(b"").is_empty());
}

#[test]
fn socket_links_yield_inodes() {
    assert_eq!(socket_inode("socket:[12345]"), Some(12_345));
    assert_eq!(socket_inode("pipe:[12345]"), None);
    assert_eq!(socket_inode("/dev/null"), None);
}

#[test]
fn only_listening_sockets_of_the_process_count() {
    let table = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
   0: 0100007F:CBD8 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 12345 1\n\
   1: 0100007F:CBD9 0100007F:A000 01 00000000:00000000 00:00000000 00000000  1000        0 12346 1\n\
   2: 0100007F:CBDA 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 99999 1\n\
   3: garbage\n";
    let inodes = BTreeSet::from([12_345, 12_346]);
    assert_eq!(listening_ports(table, &inodes), BTreeSet::from([0xCBD8]));
}

#[test]
fn ipv6_tables_parse_the_same_way() {
    let table = "header\n   0: 00000000000000000000000001000000:A411 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 777 1\n";
    assert_eq!(
        listening_ports(table, &BTreeSet::from([777])),
        BTreeSet::from([0xA411])
    );
}
