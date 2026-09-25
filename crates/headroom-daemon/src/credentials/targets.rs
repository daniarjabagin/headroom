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
            let path = login.credentials_path(&record.reference.home);
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

    use jiff::Timestamp;

    use super::*;
    use crate::model::AccountRuntime;
    use crate::testing::{CLAUDE, CODEX, account, catalog};

    fn record(provider: headroom_core::account::ProviderId, name: &str) -> AccountRecord {
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
}
