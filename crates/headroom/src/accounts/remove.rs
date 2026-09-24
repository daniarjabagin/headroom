use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use headroom_core::account::{AccountId, AccountRef, CredentialOwner, ProviderId};
use headroom_core::descriptor::ProviderDescriptor;
use headroom_providers::registry;
use headroom_providers::secrets::SecretStore;

use super::announce::{rescan_if_running, shown_owner};
use super::discovery::discover_local;
use super::dismiss::Dismissal;
use super::home::headroom_home;
use super::progress::{JsonLines, ProgressEvent};
use crate::cli::ProgressFormat;
use crate::paths::{Globals, accounts_root};
use crate::providers::LocalRegistry;

struct Confirmation {
    question: String,
    refusal: String,
}

enum Removal {
    Deleted { id: String, home: PathBuf },
    Dismissed { message: String },
}

impl Removal {
    fn message(&self) -> String {
        match self {
            Removal::Deleted { id, home } => format!("Removed {id} and deleted {}", home.display()),
            Removal::Dismissed { message } => message.clone(),
        }
    }
}

pub async fn remove(
    globals: &Globals,
    id: &str,
    assume_yes: bool,
    progress: Option<ProgressFormat>,
) -> Result<()> {
    match progress {
        None => {
            let approve = |ask: &Confirmation| Ok(assume_yes || confirm(ask)?);
            let removal = remove_account(globals, id, approve).await?;
            writeln!(io::stdout(), "{}", removal.message())?;
            Ok(())
        }
        Some(ProgressFormat::Json) => {
            let approve = |ask: &Confirmation| Ok(assume_yes || refuse_without_yes(ask)?);
            let result = remove_account(globals, id, approve).await;
            let done = |_: &Removal| ProgressEvent::Done {
                account_id: id.to_owned(),
                label: None,
            };
            JsonLines::new(io::stdout()).finish(result, done)?;
            Ok(())
        }
    }
}

async fn remove_account(
    globals: &Globals,
    id: &str,
    approve: impl FnOnce(&Confirmation) -> Result<bool>,
) -> Result<Removal> {
    let registry = LocalRegistry::for_cli(globals)?;
    let accounts = discover_local(&registry.all()).await;
    let shown = shown_owner(globals, id).await?;
    let Some(account) = pick_record(&accounts, id, shown) else {
        bail!("no signed-in account {id} found");
    };
    match account.owner {
        CredentialOwner::Cli => dismiss_account(globals, account, approve).await,
        CredentialOwner::Headroom => delete_account(globals, &registry, account, approve).await,
    }
}

async fn dismiss_account(
    globals: &Globals,
    account: &AccountRef,
    approve: impl FnOnce(&Confirmation) -> Result<bool>,
) -> Result<Removal> {
    let dismissal = Dismissal::prepare(globals, account).await?;
    if !approve(&dismiss_confirmation(account))? {
        bail!("cancelled");
    }
    let message = dismissal.apply().await?;
    Ok(Removal::Dismissed { message })
}

async fn delete_account(
    globals: &Globals,
    registry: &LocalRegistry,
    account: &AccountRef,
    approve: impl FnOnce(&Confirmation) -> Result<bool>,
) -> Result<Removal> {
    let home = headroom_home(&accounts_root()?, &account.provider, &account.home)?;
    if !approve(&delete_confirmation(&account.id, &home))? {
        bail!("cancelled");
    }
    let secrets = takes_api_keys(&account.provider).then_some(registry.secrets.as_ref());
    forget(secrets, &account.id, &home).await?;
    rescan_if_running(globals).await?;
    Ok(Removal::Deleted {
        id: account.id.0.clone(),
        home,
    })
}

fn pick_record<'a>(
    accounts: &'a [AccountRef],
    id: &str,
    shown: Option<CredentialOwner>,
) -> Option<&'a AccountRef> {
    let mut records = accounts.iter().filter(|account| account.id.0 == id);
    let first = records.clone().next();
    records
        .find(|account| Some(account.owner) == shown)
        .or(first)
}

fn dismiss_confirmation(account: &AccountRef) -> Confirmation {
    Confirmation {
        question: format!(
            "Stop showing {} in Headroom? The {} CLI stays signed in.",
            account.id, account.provider
        ),
        refusal: format!("refusing to dismiss {} without --yes", account.id),
    }
}

fn delete_confirmation(id: &AccountId, home: &Path) -> Confirmation {
    Confirmation {
        question: format!("Sign out {id} and delete {}?", home.display()),
        refusal: format!("refusing to delete {} without --yes", home.display()),
    }
}

async fn forget(secrets: Option<&SecretStore>, id: &AccountId, home: &Path) -> Result<()> {
    if let Some(secrets) = secrets {
        secrets
            .delete(id)
            .await
            .context("could not delete the stored API key; the account is kept, try again")?;
    }
    std::fs::remove_dir_all(home).with_context(|| format!("could not delete {}", home.display()))
}

fn takes_api_keys(provider: &ProviderId) -> bool {
    registry::descriptor(provider.as_str()).is_some_and(ProviderDescriptor::accepts_api_key)
}

fn refuse_without_yes(ask: &Confirmation) -> Result<bool> {
    bail!("{}", ask.refusal)
}

fn confirm(ask: &Confirmation) -> Result<bool> {
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return refuse_without_yes(ask);
    }
    write!(io::stderr(), "{} [y/N] ", ask.question)?;
    io::stderr().flush()?;
    let mut answer = String::new();
    stdin.lock().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use headroom_providers::keychain::Security;
    use headroom_providers::secrets::SecretBus;

    use super::*;

    #[test]
    fn confirmations_say_what_happens_to_the_account() {
        let account = AccountRef {
            id: AccountId("grok:0123456789ab".into()),
            provider: ProviderId::parse("grok").unwrap(),
            home: PathBuf::from("/home/ada/.grok"),
            owner: CredentialOwner::Cli,
        };
        let dismiss = dismiss_confirmation(&account);
        assert!(dismiss.question.contains("The grok CLI stays signed in"));
        assert_eq!(
            dismiss.refusal,
            "refusing to dismiss grok:0123456789ab without --yes"
        );
        let delete = delete_confirmation(&account.id, Path::new("/x/home"));
        assert_eq!(delete.refusal, "refusing to delete /x/home without --yes");
    }

    #[test]
    fn removal_targets_the_home_the_daemon_shows() {
        let record = |home: &str, owner| AccountRef {
            id: AccountId("grok:0123456789ab".into()),
            provider: ProviderId::parse("grok").unwrap(),
            home: PathBuf::from(home),
            owner,
        };
        let cli = record("/home/ada/.grok", CredentialOwner::Cli);
        let own = record("/data/grok/1", CredentialOwner::Headroom);
        let both = [cli.clone(), own.clone()];
        let id = "grok:0123456789ab";
        assert_eq!(
            pick_record(&both, id, Some(CredentialOwner::Headroom)),
            Some(&own)
        );
        assert_eq!(
            pick_record(&both, id, Some(CredentialOwner::Cli)),
            Some(&cli)
        );
        assert_eq!(pick_record(&both, id, None), Some(&cli));
        assert_eq!(
            pick_record(
                std::slice::from_ref(&cli),
                id,
                Some(CredentialOwner::Headroom)
            ),
            Some(&cli)
        );
        assert_eq!(pick_record(&both, "grok:other", None), None);
    }

    #[tokio::test]
    async fn a_key_that_cannot_be_deleted_keeps_the_home() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir(&home).unwrap();
        let keychain = Security::new(dir.path().join("no-security"), Duration::from_secs(1));
        let secrets = SecretStore::with_keychain(keychain, dir.path().join("secrets"));
        let id = AccountId("keyed:0123456789ab".into());
        assert!(forget(Some(&secrets), &id, &home).await.is_err());
        assert!(home.exists());
    }

    #[tokio::test]
    async fn a_deleted_key_is_followed_by_the_home() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir(&home).unwrap();
        let secrets = SecretStore::new(SecretBus::Disabled, dir.path().join("secrets"));
        let id = AccountId("keyed:0123456789ab".into());
        forget(Some(&secrets), &id, &home).await.unwrap();
        assert!(!home.exists());
    }
}
