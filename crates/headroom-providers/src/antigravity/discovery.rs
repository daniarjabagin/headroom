use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::process::{self, Candidate, Rank};

const MAX_SERVERS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LanguageServer {
    pub rank: Rank,
    pub csrf: String,
    pub ports: BTreeSet<u16>,
    pub extension_port: Option<u16>,
}

impl LanguageServer {
    pub(super) fn endpoints(&self) -> Vec<String> {
        let mut endpoints: Vec<String> = self
            .ports
            .iter()
            .flat_map(|port| {
                [
                    format!("https://127.0.0.1:{port}"),
                    format!("http://127.0.0.1:{port}"),
                ]
            })
            .collect();
        if let Some(port) = self
            .extension_port
            .filter(|port| !self.ports.contains(port))
        {
            endpoints.push(format!("http://127.0.0.1:{port}"));
        }
        endpoints
    }
}

pub(super) fn language_servers(proc_root: &Path, owner: u32) -> Vec<LanguageServer> {
    let mut servers: Vec<LanguageServer> = process_dirs(proc_root, owner)
        .into_iter()
        .filter_map(|dir| server_at(&dir))
        .collect();
    servers.sort_by(|a, b| a.rank.cmp(&b.rank));
    servers.truncate(MAX_SERVERS);
    servers
}

fn process_dirs(proc_root: &Path, owner: u32) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(proc_root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_digit())
        })
        .filter(|entry| entry.metadata().is_ok_and(|meta| meta.uid() == owner))
        .map(|entry| entry.path())
        .collect()
}

fn server_at(dir: &Path) -> Option<LanguageServer> {
    let cmdline = fs::read(dir.join("cmdline")).ok()?;
    let Candidate {
        rank,
        csrf,
        extension_port,
    } = process::candidate(&process::argv(&cmdline))?;
    let ports = ports_of(dir);
    if ports.is_empty() && extension_port.is_none() {
        return None;
    }
    Some(LanguageServer {
        rank,
        csrf,
        ports,
        extension_port,
    })
}

fn ports_of(dir: &Path) -> BTreeSet<u16> {
    let inodes = socket_inodes(&dir.join("fd"));
    if inodes.is_empty() {
        return BTreeSet::new();
    }
    ["net/tcp", "net/tcp6"]
        .iter()
        .filter_map(|table| fs::read_to_string(dir.join(table)).ok())
        .flat_map(|table| process::listening_ports(&table, &inodes))
        .collect()
}

fn socket_inodes(fd_dir: &Path) -> BTreeSet<u64> {
    let Ok(entries) = fs::read_dir(fd_dir) else {
        return BTreeSet::new();
    };
    entries
        .flatten()
        .filter_map(|entry| fs::read_link(entry.path()).ok())
        .filter_map(|link| process::socket_inode(&link.to_string_lossy()))
        .collect()
}

pub(super) fn installed(gemini_dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(gemini_dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .starts_with("antigravity")
            && entry.file_type().is_ok_and(|kind| kind.is_dir())
    })
}

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod tests;
