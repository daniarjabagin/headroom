use std::fs;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use headroom_core::account::AccountRef;
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::quota::LimitsSnapshot;
use headroom_providers::test_support::install_script;

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
        install_script(&path, &format!("#!/bin/sh\n{body}\n")).unwrap();
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

pub(super) struct FixedAccounts(pub Vec<AccountRef>);

#[async_trait]
impl Provider for FixedAccounts {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        headroom_providers::registry::descriptor("claude").unwrap()
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        Ok(self.0.clone())
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, _account: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        Err(ProviderError::NotSignedIn)
    }

    fn read_usage(
        &self,
        _home: &Path,
        _cursors: &mut LogCursors,
    ) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }
}
