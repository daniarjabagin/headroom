use std::collections::BTreeSet;
use std::path::Path;

use headroom_core::secret::SecretString;

const LANGUAGE_SERVER: &str = "language_server";
const CLI: &str = "agy";
const MARKERS: [&str; 2] = ["antigravity", "antigravity-ide"];
const MARKER_FLAGS: [&str; 3] = ["--ide_name", "--override_ide_name", "--app_data_dir"];
const CSRF_FLAG: &str = "--csrf_token";
const EXTENSION_PORT_FLAG: &str = "--extension_server_port";
const LISTEN_STATE: &str = "0A";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Rank {
    NamedApp,
    AppPath,
    Cli,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Candidate {
    pub rank: Rank,
    pub csrf: SecretString,
    pub extension_port: Option<u16>,
}

pub(super) fn candidate(argv: &[String]) -> Option<Candidate> {
    let exe = executable_name(argv.first()?);
    let rank = if exe == CLI {
        Rank::Cli
    } else if exe == LANGUAGE_SERVER || exe.starts_with(&format!("{LANGUAGE_SERVER}_")) {
        app_rank(argv)?
    } else {
        return None;
    };
    let csrf = flag(argv, CSRF_FLAG);
    if csrf.is_none() && rank != Rank::Cli {
        return None;
    }
    Some(Candidate {
        rank,
        csrf: SecretString::new(csrf.unwrap_or_default().to_owned()),
        extension_port: flag(argv, EXTENSION_PORT_FLAG).and_then(|port| port.parse().ok()),
    })
}

fn executable_name(argv0: &str) -> String {
    Path::new(argv0)
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn app_rank(argv: &[String]) -> Option<Rank> {
    let named: Vec<String> = MARKER_FLAGS
        .iter()
        .filter_map(|marker_flag| flag(argv, marker_flag))
        .map(str::to_lowercase)
        .collect();
    if !named.is_empty() {
        return named
            .iter()
            .any(|name| MARKERS.contains(&name.as_str()))
            .then_some(Rank::NamedApp);
    }
    argv.iter()
        .any(|arg| {
            let arg = arg.to_lowercase();
            MARKERS
                .iter()
                .any(|marker| arg.contains(&format!("/{marker}/")))
        })
        .then_some(Rank::AppPath)
}

fn flag<'a>(argv: &'a [String], name: &str) -> Option<&'a str> {
    let inline = format!("{name}=");
    argv.iter().enumerate().find_map(|(index, arg)| {
        if arg == name {
            argv.get(index + 1).map(String::as_str)
        } else {
            arg.strip_prefix(&inline)
        }
    })
}

pub(super) fn argv(cmdline: &[u8]) -> Vec<String> {
    cmdline
        .split(|byte| *byte == 0)
        .filter(|arg| !arg.is_empty())
        .map(|arg| String::from_utf8_lossy(arg).into_owned())
        .collect()
}

pub(super) fn socket_inode(link: &str) -> Option<u64> {
    link.strip_prefix("socket:[")?
        .strip_suffix(']')?
        .parse()
        .ok()
}

pub(super) fn listening_ports(table: &str, inodes: &BTreeSet<u64>) -> BTreeSet<u16> {
    table
        .lines()
        .skip(1)
        .filter_map(|line| listening_port(line, inodes))
        .collect()
}

fn listening_port(line: &str, inodes: &BTreeSet<u64>) -> Option<u16> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let local = fields.get(1)?;
    let state = fields.get(3)?;
    let inode: u64 = fields.get(9)?.parse().ok()?;
    if *state != LISTEN_STATE || !inodes.contains(&inode) {
        return None;
    }
    let (_, port) = local.rsplit_once(':')?;
    u16::from_str_radix(port, 16).ok().filter(|port| *port > 0)
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
