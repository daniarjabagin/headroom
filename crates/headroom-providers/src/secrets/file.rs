use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use headroom_core::account::AccountId;
use headroom_core::secret::SecretString;

use super::SecretError;
use crate::fsio::{create_private_dir, write_private};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FileSecrets {
    dir: PathBuf,
}

impl FileSecrets {
    pub(super) fn new(dir: PathBuf) -> FileSecrets {
        FileSecrets { dir }
    }

    pub(super) fn read(&self, account: &AccountId) -> Result<Option<SecretString>, SecretError> {
        let path = self.path(account)?;
        match fs::read(&path) {
            Ok(bytes) => decode(bytes).map(Some),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io_error("read", &path, &error)),
        }
    }

    pub(super) fn write(
        &self,
        account: &AccountId,
        secret: &SecretString,
    ) -> Result<(), SecretError> {
        let path = self.path(account)?;
        create_private_dir(&self.dir).map_err(|error| io_error("create", &self.dir, &error))?;
        write_private(&path, secret.expose().as_bytes())
            .map_err(|error| io_error("write", &path, &error))
    }

    pub(super) fn delete(&self, account: &AccountId) -> Result<bool, SecretError> {
        let path = self.path(account)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(io_error("delete", &path, &error)),
        }
    }

    fn path(&self, account: &AccountId) -> Result<PathBuf, SecretError> {
        if is_safe_name(&account.0) {
            Ok(self.dir.join(&account.0))
        } else {
            Err(SecretError::InvalidAccount(account.0.clone()))
        }
    }
}

pub(super) fn decode(bytes: Vec<u8>) -> Result<SecretString, SecretError> {
    String::from_utf8(bytes)
        .map(SecretString::new)
        .map_err(|_| SecretError::NotUtf8)
}

fn is_safe_name(name: &str) -> bool {
    name.bytes().any(|b| b.is_ascii_alphanumeric())
        && name.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b':')
        })
}

fn io_error(action: &'static str, path: &Path, error: &io::Error) -> SecretError {
    SecretError::Io {
        action,
        path: path.to_path_buf(),
        kind: error.kind(),
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn account(id: &str) -> AccountId {
        AccountId(id.into())
    }

    #[test]
    fn secrets_live_in_private_files_named_by_account() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("headroom/secrets");
        let files = FileSecrets::new(dir.clone());
        let id = account("tool:0123456789ab");
        assert_eq!(files.read(&id).unwrap(), None);
        files.write(&id, &SecretString::new("sk-1".into())).unwrap();
        files.write(&id, &SecretString::new("sk-2".into())).unwrap();
        assert_eq!(files.read(&id).unwrap().unwrap().expose(), "sk-2");
        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&dir.join("tool:0123456789ab")), 0o600);
        assert!(files.delete(&id).unwrap());
        assert!(!files.delete(&id).unwrap());
        assert_eq!(files.read(&id).unwrap(), None);
    }

    #[test]
    fn account_ids_cannot_escape_the_directory() {
        let files = FileSecrets::new(PathBuf::from("/nonexistent"));
        for bad in ["../x", "a/b", "", "..", ":", "Tool:1"] {
            assert!(
                matches!(
                    files.read(&account(bad)),
                    Err(SecretError::InvalidAccount(_))
                ),
                "{bad}"
            );
        }
    }

    #[test]
    fn non_utf8_secrets_are_rejected() {
        assert!(matches!(decode(vec![0xff]), Err(SecretError::NotUtf8)));
    }
}
