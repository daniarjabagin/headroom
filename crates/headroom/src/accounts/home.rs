use std::fs::{self, DirBuilder, Permissions};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use headroom_core::account::ProviderId;
use uuid::Uuid;

const PRIVATE_DIR: u32 = 0o700;

pub fn create_home(root: &Path, provider: &ProviderId) -> Result<PathBuf> {
    let parent = root.join(provider.as_str());
    DirBuilder::new()
        .recursive(true)
        .mode(PRIVATE_DIR)
        .create(&parent)
        .with_context(|| format!("could not create {}", parent.display()))?;
    let home = parent.join(Uuid::new_v4().to_string());
    DirBuilder::new()
        .mode(PRIVATE_DIR)
        .create(&home)
        .with_context(|| format!("could not create {}", home.display()))?;
    fs::set_permissions(&home, Permissions::from_mode(PRIVATE_DIR))
        .with_context(|| format!("could not restrict {}", home.display()))?;
    Ok(home)
}

pub fn discard_home(home: &Path) {
    if let Err(error) = fs::remove_dir_all(home) {
        tracing::warn!(home = %home.display(), %error, "could not remove the new home");
    }
}

pub fn headroom_home(root: &Path, provider: &ProviderId, home: &Path) -> Result<PathBuf> {
    let parent = root.join(provider.as_str());
    let refuse = || format!("{} is not a Headroom-owned account home", home.display());
    let metadata = fs::symlink_metadata(home).with_context(refuse)?;
    if !metadata.is_dir() {
        bail!(refuse());
    }
    let home = fs::canonicalize(home).with_context(refuse)?;
    let parent = fs::canonicalize(&parent).with_context(refuse)?;
    let named_by_uuid = home
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| Uuid::parse_str(name).is_ok());
    if home.parent() != Some(parent.as_path()) || !named_by_uuid {
        bail!(refuse());
    }
    Ok(home)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use headroom_providers::{claude, codex};

    use super::*;

    #[test]
    fn creates_a_private_uuid_home_per_provider() {
        let root = tempfile::tempdir().unwrap();
        let home = create_home(root.path(), &claude::ID).unwrap();
        assert_eq!(home.parent().unwrap(), root.path().join("claude"));
        let name = home.file_name().unwrap().to_str().unwrap();
        assert!(Uuid::parse_str(name).is_ok());
        let mode = fs::metadata(&home).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
        assert_ne!(create_home(root.path(), &claude::ID).unwrap(), home);
    }

    #[test]
    fn accepts_only_direct_uuid_children_of_the_provider_root() {
        let root = tempfile::tempdir().unwrap();
        let home = create_home(root.path(), &codex::ID).unwrap();
        assert_eq!(
            headroom_home(root.path(), &codex::ID, &home).unwrap(),
            fs::canonicalize(&home).unwrap()
        );
        assert!(headroom_home(root.path(), &claude::ID, &home).is_err());
        let nested = home.join(Uuid::new_v4().to_string());
        fs::create_dir(&nested).unwrap();
        assert!(headroom_home(root.path(), &codex::ID, &nested).is_err());
        let named = root.path().join("codex/not-a-uuid");
        fs::create_dir(&named).unwrap();
        assert!(headroom_home(root.path(), &codex::ID, &named).is_err());
    }

    #[test]
    fn refuses_cli_homes_and_symlinks_out_of_the_root() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let cli_home = outside.path().join(".codex");
        fs::create_dir(&cli_home).unwrap();
        create_home(root.path(), &codex::ID).unwrap();
        assert!(headroom_home(root.path(), &codex::ID, &cli_home).is_err());
        let link = root.path().join("codex").join(Uuid::new_v4().to_string());
        symlink(&cli_home, &link).unwrap();
        assert!(headroom_home(root.path(), &codex::ID, &link).is_err());
        assert!(headroom_home(root.path(), &codex::ID, &root.path().join("x")).is_err());
    }
}
