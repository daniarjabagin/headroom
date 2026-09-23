mod discovery;
mod home;
mod login;

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use headroom_core::account::{AccountRef, CredentialOwner, ProviderKind};

use crate::client::{self, DaemonProxy, call_error};
use crate::paths::{Globals, accounts_root};
use crate::render::format::provider_name;
use discovery::{account_at, discover_local};
use home::headroom_home;
use login::{Launcher, login_spec, sign_in};

const NOT_DISCOVERED: &str = "The daemon did not find this account; check `headroom accounts`.";

pub async fn add(globals: &Globals, provider: ProviderKind, label: Option<&str>) -> Result<()> {
    let root = accounts_root()?;
    let preview = root.join(provider.as_str()).join("<new>");
    writeln!(
        io::stderr(),
        "Running {}",
        login_spec(provider).display(&preview)
    )?;
    let home = tokio::task::spawn_blocking(move || sign_in(&root, provider, &Launcher::default()))
        .await??;
    let account = account_at(provider, &home).await?.with_context(|| {
        format!(
            "signed in, but Headroom cannot read the account in {}",
            home.display()
        )
    })?;
    writeln!(
        io::stdout(),
        "Added {} account {}",
        provider_name(provider),
        account.id.0
    )?;
    warn_if_duplicate(&account).await?;
    announce(globals, &account, label).await
}

async fn warn_if_duplicate(account: &AccountRef) -> Result<()> {
    let existing = discover_local()
        .await
        .into_iter()
        .find(|known| known.id == account.id && known.home != account.home);
    if let Some(existing) = existing {
        let path = existing.home.display();
        writeln!(
            io::stderr(),
            "This account is already tracked from {path}; Headroom keeps using that one."
        )?;
    }
    Ok(())
}

async fn announce(globals: &Globals, account: &AccountRef, label: Option<&str>) -> Result<()> {
    let id = &account.id.0;
    let Ok(proxy) = client::require_daemon(&globals.bus).await else {
        if let Some(label) = label {
            writeln!(
                io::stderr(),
                "Start the daemon, then run: headroom accounts label {id} {label:?}"
            )?;
        }
        return Ok(());
    };
    proxy.rescan().await.map_err(call_error)?;
    if !daemon_knows(&proxy, id).await? {
        writeln!(io::stderr(), "{NOT_DISCOVERED}")?;
        if let Some(label) = label {
            writeln!(
                io::stderr(),
                "Once it is listed, run: headroom accounts label {id} {label:?}"
            )?;
        }
        return Ok(());
    }
    if let Some(label) = label {
        proxy
            .set_account_label(id, label)
            .await
            .map_err(call_error)?;
        writeln!(io::stdout(), "Labelled {id} as {label}")?;
    }
    Ok(())
}

async fn daemon_knows(proxy: &DaemonProxy<'_>, id: &str) -> Result<bool> {
    let (_, state) = client::fetch_state(proxy).await?;
    Ok(state.accounts.iter().any(|account| account.id == id))
}

pub async fn remove(globals: &Globals, id: &str, assume_yes: bool) -> Result<()> {
    let accounts = discover_local().await;
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
    let home = headroom_home(&accounts_root()?, account.provider, &account.home)?;
    if !assume_yes && !confirm(id, &home)? {
        bail!("cancelled");
    }
    std::fs::remove_dir_all(&home)
        .with_context(|| format!("could not delete {}", home.display()))?;
    writeln!(io::stdout(), "Removed {id} and deleted {}", home.display())?;
    if let Ok(proxy) = client::require_daemon(&globals.bus).await {
        proxy.rescan().await.map_err(call_error)?;
    }
    Ok(())
}

fn confirm(id: &str, home: &Path) -> Result<bool> {
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        bail!("refusing to delete {} without --yes", home.display());
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
