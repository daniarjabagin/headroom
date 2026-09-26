use std::collections::BTreeMap;
use std::path::PathBuf;

use headroom_core::account::AccountId;
use headroom_core::provider::ProviderError;

use crate::catalog::ProviderCatalog;
use crate::model::{Model, RefreshFailure};
use crate::storage::accounts::AccountRecord;

#[must_use]
pub fn credential_files(model: &Model, catalog: &ProviderCatalog) -> BTreeMap<AccountId, PathBuf> {
    model
        .active_accounts()
        .filter(|record| waits_for_sign_in(model, record))
        .filter_map(|record| {
            let login = catalog
                .descriptor(&record.reference.provider)?
                .cli_login()?;
            let reference = &record.reference;
            let path = login.account_credentials_path(&reference.home, reference.owner);
            Some((record.id().clone(), path))
        })
        .collect()
}

fn waits_for_sign_in(model: &Model, record: &AccountRecord) -> bool {
    model
        .runtime
        .get(record.id())
        .and_then(|runtime| runtime.failure.as_ref())
        .is_some_and(|failure| match failure {
            RefreshFailure::Provider(error) => {
                error.needs_sign_in() || matches!(error, ProviderError::AccountChanged(_))
            }
            RefreshFailure::Timeout | RefreshFailure::NoProvider => false,
        })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use headroom_core::account::{CredentialOwner, ProviderId};
    use headroom_core::descriptor::{
        AddAccountMethod, CliLogin, HomeVar, ProviderDescriptor, ProviderLinks,
    };
    use jiff::Timestamp;

    use super::*;
    use crate::model::AccountRuntime;
    use crate::testing::{CLAUDE, CODEX, account, catalog};

    fn record(provider: ProviderId, name: &str) -> AccountRecord {
        AccountRecord {
            reference: account(provider, name),
            label: None,
            hidden: false,
            sort_order: 0,
            email: None,
            plan: None,
            last_seen: Timestamp::UNIX_EPOCH,
            gone: false,
        }
    }

    fn failing(model: &mut Model, id: &str, error: ProviderError) {
        let runtime = AccountRuntime {
            failure: Some(RefreshFailure::Provider(error)),
            ..AccountRuntime::default()
        };
        model.runtime.insert(AccountId(id.into()), runtime);
    }

    fn model() -> Model {
        let mut gone = record(CODEX, "gone");
        gone.gone = true;
        Model {
            accounts: vec![
                record(CODEX, "expired"),
                record(CODEX, "moved"),
                record(CODEX, "offline"),
                record(CODEX, "healthy"),
                record(CLAUDE, "detected"),
                gone,
            ],
            ..Model::default()
        }
    }

    #[test]
    fn only_signed_out_accounts_with_a_cli_credential_file_are_watched() {
        let mut model = model();
        failing(&mut model, "codex:expired", ProviderError::SignInExpired);
        failing(
            &mut model,
            "codex:moved",
            ProviderError::AccountChanged("x".into()),
        );
        failing(
            &mut model,
            "codex:offline",
            ProviderError::Network("down".into()),
        );
        failing(&mut model, "claude:detected", ProviderError::NotSignedIn);
        failing(&mut model, "codex:gone", ProviderError::NotSignedIn);
        let watched = credential_files(&model, &catalog());
        let auth = Path::new("/home/ada/.codex/auth.json").to_path_buf();
        let expected = BTreeMap::from([
            (AccountId("codex:expired".into()), auth.clone()),
            (AccountId("codex:moved".into()), auth),
        ]);
        assert_eq!(watched, expected);
    }

    #[test]
    fn healthy_accounts_are_not_watched() {
        assert!(credential_files(&model(), &catalog()).is_empty());
    }

    const DATA: ProviderId = ProviderId::from_static("data");

    static XDG_DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
        id: DATA,
        display_name: "Data",
        add_account: &[AddAccountMethod::CliLogin(CliLogin {
            program: "data",
            args: &["login"],
            home_var: HomeVar::XdgBase {
                var: "XDG_DATA_HOME",
                subdir: "data",
            },
            credentials_file: "auth.json",
            default_dir: ".local/share/data",
            needs_pty: false,
            scrub_env: &[],
        })],
        multi_account: true,
        local_usage: false,
        min_poll_interval: None,
        links: ProviderLinks::NONE,
    };

    fn xdg_record(name: &str, home: &str, owner: CredentialOwner) -> AccountRecord {
        let mut record = record(DATA, name);
        record.reference.home = PathBuf::from(home);
        record.reference.owner = owner;
        record
    }

    #[test]
    fn each_owner_is_watched_where_its_login_writes() {
        let mut model = Model {
            accounts: vec![
                xdg_record("cli", "/home/ada/.local/share/data", CredentialOwner::Cli),
                xdg_record("custom", "/work/alt/data", CredentialOwner::Cli),
                xdg_record("own", "/data/accounts/data/1", CredentialOwner::Headroom),
            ],
            ..Model::default()
        };
        for id in ["data:cli", "data:custom", "data:own"] {
            failing(&mut model, id, ProviderError::SignInExpired);
        }
        let catalog = ProviderCatalog::new([&XDG_DESCRIPTOR]);
        let watched = credential_files(&model, &catalog);
        let expected = BTreeMap::from([
            (
                AccountId("data:cli".into()),
                PathBuf::from("/home/ada/.local/share/data/auth.json"),
            ),
            (
                AccountId("data:custom".into()),
                PathBuf::from("/work/alt/data/auth.json"),
            ),
            (
                AccountId("data:own".into()),
                PathBuf::from("/data/accounts/data/1/data/auth.json"),
            ),
        ]);
        assert_eq!(watched, expected);
    }
}
