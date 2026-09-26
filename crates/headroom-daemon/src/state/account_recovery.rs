use headroom_core::account::{AccountRef, CredentialOwner};
use headroom_core::descriptor::ProviderDescriptor;
use headroom_core::provider::ProviderError;

use super::payload::Recovery;
use crate::catalog::ProviderCatalog;
use crate::model::RefreshFailure;

#[must_use]
pub fn recovery(
    failure: Option<&RefreshFailure>,
    account: &AccountRef,
    catalog: &ProviderCatalog,
) -> Option<Recovery> {
    match failure? {
        RefreshFailure::Timeout => Some(Recovery::Retry),
        RefreshFailure::NoProvider => None,
        RefreshFailure::Provider(error) => provider_recovery(error, account, catalog),
    }
}

fn provider_recovery(
    error: &ProviderError,
    account: &AccountRef,
    catalog: &ProviderCatalog,
) -> Option<Recovery> {
    match error {
        ProviderError::NotSignedIn | ProviderError::SignInExpired | ProviderError::ApiKeyOnly => {
            Some(sign_in(account, catalog))
        }
        ProviderError::NoSubscription { .. }
        | ProviderError::RateLimited { .. }
        | ProviderError::Unsupported(_) => None,
        ProviderError::AccountChanged(_)
        | ProviderError::Network(_)
        | ProviderError::InvalidResponse(_)
        | ProviderError::LocalData(_) => Some(Recovery::Retry),
    }
}

fn sign_in(account: &AccountRef, catalog: &ProviderCatalog) -> Recovery {
    if account.owner == CredentialOwner::Headroom {
        return Recovery::SignIn {
            account_id: account.id.0.clone(),
        };
    }
    catalog
        .descriptor(&account.provider)
        .and_then(ProviderDescriptor::cli_login)
        .map_or(Recovery::Retry, |login| Recovery::CliLogin {
            command: login.command_line(),
            account_id: account.id.0.clone(),
        })
}

#[cfg(test)]
mod tests {
    use headroom_core::account::ProviderId;
    use jiff::SignedDuration;

    use super::*;
    use crate::testing::{CLAUDE, CODEX, account, catalog};

    fn owned(provider: ProviderId, owner: CredentialOwner) -> AccountRef {
        AccountRef {
            owner,
            ..account(provider, "work")
        }
    }

    fn of(error: ProviderError, account: &AccountRef) -> Option<Recovery> {
        recovery(Some(&RefreshFailure::Provider(error)), account, &catalog())
    }

    #[test]
    fn healthy_accounts_have_no_recovery() {
        let cli = owned(CODEX, CredentialOwner::Cli);
        assert_eq!(recovery(None, &cli, &catalog()), None);
    }

    #[test]
    fn cli_owned_sign_ins_are_fixed_in_the_cli() {
        let cli = owned(CODEX, CredentialOwner::Cli);
        let login = Some(Recovery::CliLogin {
            command: "codex login".into(),
            account_id: "codex:work".into(),
        });
        assert_eq!(of(ProviderError::SignInExpired, &cli), login);
        assert_eq!(of(ProviderError::NotSignedIn, &cli), login);
        assert_eq!(of(ProviderError::ApiKeyOnly, &cli), login);
    }

    #[test]
    fn headroom_owned_sign_ins_are_fixed_by_signing_in_again() {
        let headroom = owned(CODEX, CredentialOwner::Headroom);
        assert_eq!(
            of(ProviderError::SignInExpired, &headroom),
            Some(Recovery::SignIn {
                account_id: "codex:work".into()
            })
        );
    }

    #[test]
    fn a_cli_account_without_a_login_command_falls_back_to_retry() {
        let detected = owned(CLAUDE, CredentialOwner::Cli);
        assert_eq!(
            of(ProviderError::SignInExpired, &detected),
            Some(Recovery::Retry)
        );
    }

    #[test]
    fn transient_errors_and_account_changes_are_retried() {
        let cli = owned(CODEX, CredentialOwner::Cli);
        for error in [
            ProviderError::AccountChanged("moved".into()),
            ProviderError::Network("down".into()),
            ProviderError::InvalidResponse("bad".into()),
            ProviderError::LocalData("broken".into()),
        ] {
            assert_eq!(of(error, &cli), Some(Recovery::Retry));
        }
        let timeout = recovery(Some(&RefreshFailure::Timeout), &cli, &catalog());
        assert_eq!(timeout, Some(Recovery::Retry));
    }

    #[test]
    fn waiting_states_offer_nothing_to_do() {
        let cli = owned(CODEX, CredentialOwner::Cli);
        let limited = ProviderError::RateLimited {
            retry_after: Some(SignedDuration::from_secs(60)),
        };
        assert_eq!(of(limited, &cli), None);
        let lapsed = ProviderError::NoSubscription {
            detail: "none".into(),
        };
        assert_eq!(of(lapsed, &cli), None);
        assert_eq!(of(ProviderError::Unsupported("no".into()), &cli), None);
        let unknown = recovery(Some(&RefreshFailure::NoProvider), &cli, &catalog());
        assert_eq!(unknown, None);
    }

    #[test]
    fn recovery_serializes_with_an_action_tag() {
        let cases = [
            (Recovery::Retry, serde_json::json!({ "action": "retry" })),
            (
                Recovery::SignIn {
                    account_id: "claude:a".into(),
                },
                serde_json::json!({ "action": "sign_in", "account_id": "claude:a" }),
            ),
            (
                Recovery::CliLogin {
                    command: "claude auth login --claudeai".into(),
                    account_id: "claude:a".into(),
                },
                serde_json::json!({
                    "action": "cli_login",
                    "command": "claude auth login --claudeai",
                    "account_id": "claude:a"
                }),
            ),
        ];
        for (recovery, json) in cases {
            assert_eq!(serde_json::to_value(&recovery).unwrap(), json);
        }
    }
}
