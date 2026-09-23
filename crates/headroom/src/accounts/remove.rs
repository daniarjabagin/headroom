use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use headroom_core::account::{CredentialOwner, ProviderId};
use headroom_core::descriptor::ProviderDescriptor;
use headroom_providers::registry;

use super::announce::rescan_if_running;
use super::discovery::discover_local;
use super::home::headroom_home;
use super::progress::{JsonLines, ProgressEvent};
use crate::cli::ProgressFormat;
use crate::paths::{Globals, accounts_root};
use crate::providers::LocalRegistry;

pub async fn remove(
    globals: &Globals,
    id: &str,
    assume_yes: bool,
    progress: Option<ProgressFormat>,
) -> Result<()> {
    match progress {
        None => {
            let approve = |home: &Path| Ok(assume_yes || confirm(id, home)?);
            let home = delete_account(globals, id, approve).await?;
            writeln!(io::stdout(), "Removed {id} and deleted {}", home.display())?;
            Ok(())
        }
        Some(ProgressFormat::Json) => {
            let approve = |home: &Path| Ok(assume_yes || refuse_without_yes(home)?);
            let result = delete_account(globals, id, approve).await;
            let done = |_: &PathBuf| ProgressEvent::Done {
                account_id: id.to_owned(),
                label: None,
            };
            JsonLines::new(io::stdout()).finish(result, done)?;
            Ok(())
        }
    }
}

async fn delete_account(
    globals: &Globals,
    id: &str,
    approve: impl FnOnce(&Path) -> Result<bool>,
) -> Result<PathBuf> {
    let registry = LocalRegistry::for_cli(globals)?;
    let accounts = discover_local(&registry.all()).await;
    let Some(account) = accounts.iter().find(|account| account.id.0 == id) else {
        bail!("no signed-in account {id} found");
    };
    if account.owner == CredentialOwner::Cli {
        bail!(
            "{id} belongs to the {} CLI ({}); Headroom never deletes CLI homes, sign out with the CLI",
            account.provider,
            account.home.display()
        );
    }
    let home = headroom_home(&accounts_root()?, &account.provider, &account.home)?;
    if !approve(&home)? {
        bail!("cancelled");
    }
    std::fs::remove_dir_all(&home)
        .with_context(|| format!("could not delete {}", home.display()))?;
    if takes_api_keys(&account.provider) {
        registry
            .secrets
            .delete(&account.id)
            .await
            .context("deleted the account home, but not its stored API key")?;
    }
    rescan_if_running(globals).await?;
    Ok(home)
}

fn takes_api_keys(provider: &ProviderId) -> bool {
    registry::descriptor(provider.as_str()).is_some_and(ProviderDescriptor::accepts_api_key)
}

fn refuse_without_yes(home: &Path) -> Result<bool> {
    bail!("refusing to delete {} without --yes", home.display())
}

fn confirm(id: &str, home: &Path) -> Result<bool> {
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return refuse_without_yes(home);
    }
    write!(
        io::stderr(),
        "Sign out {id} and delete {}? [y/N] ",
        home.display()
    )?;
    io::stderr().flush()?;
    let mut answer = String::new();
    stdin.lock().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}
