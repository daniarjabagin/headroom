use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use headroom_core::account::{AccountIdentity, AccountRef, CredentialOwner, ProviderId};
use headroom_core::provider::ProviderError;

use crate::fsio::write_private;

/// Identity of an API-key account, stored in its Headroom-owned home; the key itself is a secret.
pub const RECORD_FILE: &str = "account.json";

pub fn save_record(home: &Path, identity: &AccountIdentity) -> Result<(), ProviderError> {
    let path = home.join(RECORD_FILE);
    let bytes = serde_json::to_vec_pretty(identity)
        .map_err(|error| ProviderError::LocalData(format!("cannot encode account: {error}")))?;
    write_private(&path, &bytes).map_err(|error| local_error("write", &path, &error))
}

pub fn load_record(home: &Path) -> Result<Option<AccountIdentity>, ProviderError> {
    let path = home.join(RECORD_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(local_error("read", &path, &error)),
    };
    serde_json::from_str(&text).map(Some).map_err(|error| {
        ProviderError::LocalData(format!("cannot parse {}: {error}", path.display()))
    })
}

/// Accounts added with an API key below `root` (`…/headroom/accounts/<provider>`).
pub fn discover(root: &Path, provider: &ProviderId) -> Result<Vec<AccountRef>, ProviderError> {
    let mut accounts = Vec::new();
    for home in account_homes(root)? {
        match load_record(&home) {
            Ok(Some(identity)) => accounts.push(AccountRef {
                id: identity.account_id(provider),
                provider: provider.clone(),
                home,
                owner: CredentialOwner::Headroom,
            }),
            Ok(None) => {}
            Err(error) => tracing::warn!(home = %home.display(), %error, "skipping account"),
        }
    }
    Ok(accounts)
}

fn account_homes(root: &Path) -> Result<Vec<PathBuf>, ProviderError> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(local_error("read", root, &error)),
    };
    let mut homes: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    homes.sort();
    Ok(homes)
}

fn local_error(action: &str, path: &Path, error: &io::Error) -> ProviderError {
    ProviderError::LocalData(format!(
        "cannot {action} {}: {}",
        path.display(),
        error.kind()
    ))
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    const TOOL: ProviderId = ProviderId::from_static("tool");

    fn identity(key: &str) -> AccountIdentity {
        AccountIdentity {
            email: Some(format!("{key}@example.com")),
            plan: None,
            stable_key: key.into(),
        }
    }

    #[test]
    fn records_round_trip_privately() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(load_record(home.path()), Ok(None));
        save_record(home.path(), &identity("k1")).unwrap();
        assert_eq!(load_record(home.path()), Ok(Some(identity("k1"))));
        let mode = fs::metadata(home.path().join(RECORD_FILE))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn discovery_lists_homes_with_a_record_in_order() {
        let root = tempfile::tempdir().unwrap();
        for (name, key) in [("b", "k2"), ("a", "k1")] {
            let home = root.path().join(name);
            fs::create_dir(&home).unwrap();
            save_record(&home, &identity(key)).unwrap();
        }
        fs::create_dir(root.path().join("pending")).unwrap();
        let broken = root.path().join("broken");
        fs::create_dir(&broken).unwrap();
        fs::write(broken.join(RECORD_FILE), "{").unwrap();
        let found = discover(root.path(), &TOOL).unwrap();
        let summary: Vec<_> = found
            .iter()
            .map(|a| (a.id.clone(), a.home.clone()))
            .collect();
        assert_eq!(
            summary,
            [
                (identity("k1").account_id(&TOOL), root.path().join("a")),
                (identity("k2").account_id(&TOOL), root.path().join("b")),
            ]
        );
        assert!(found.iter().all(|a| a.owner == CredentialOwner::Headroom));
        assert_eq!(discover(&root.path().join("none"), &TOOL), Ok(Vec::new()));
    }
}
