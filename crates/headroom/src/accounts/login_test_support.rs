use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use headroom_core::descriptor::AddAccountMethod;

use super::{Launcher, LoginSpec};

pub(super) fn spec(id: &str) -> LoginSpec {
    let descriptor = headroom_providers::registry::descriptor(id).unwrap();
    match descriptor.default_method() {
        Some(AddAccountMethod::CliLogin(login)) => LoginSpec::new(descriptor, login),
        other => panic!("{id} has no CLI login: {other:?}"),
    }
}

pub(super) struct FakeBin {
    dir: tempfile::TempDir,
}

impl FakeBin {
    pub(super) fn new() -> FakeBin {
        FakeBin {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    pub(super) fn install(&self, name: &str, body: &str) {
        let path = self.dir.path().join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, Permissions::from_mode(0o755)).unwrap();
    }

    pub(super) fn launcher(&self) -> Launcher {
        Launcher {
            search_path: Some(self.dir.path().as_os_str().to_owned()),
        }
    }
}

pub(super) fn homes(root: &Path, provider: &str) -> Vec<PathBuf> {
    match fs::read_dir(root.join(provider)) {
        Ok(entries) => entries.map(|entry| entry.unwrap().path()).collect(),
        Err(_) => Vec::new(),
    }
}
