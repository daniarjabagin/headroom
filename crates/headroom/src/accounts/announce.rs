use std::io::{self, Write};

use anyhow::Result;
use headroom_core::account::{AccountRef, CredentialOwner};

use crate::client::{self, Daemon};
use crate::paths::Globals;

const NOT_DISCOVERED: &str = "The daemon did not find this account; check `headroom accounts`.";

pub async fn announce(
    globals: &Globals,
    account: &AccountRef,
    label: Option<&str>,
) -> Result<Option<String>> {
    let id = &account.id.0;
    let Ok(daemon) = client::require_daemon(globals).await else {
        if let Some(label) = label {
            writeln!(
                io::stderr(),
                "Start the daemon, then run: headroom accounts label {id} {label:?}"
            )?;
        }
        return Ok(None);
    };
    daemon.rescan().await?;
    if !daemon_knows(&daemon, id).await? {
        writeln!(io::stderr(), "{NOT_DISCOVERED}")?;
        if let Some(label) = label {
            writeln!(
                io::stderr(),
                "Once it is listed, run: headroom accounts label {id} {label:?}"
            )?;
        }
        return Ok(None);
    }
    let Some(label) = label else {
        return Ok(None);
    };
    daemon.set_account_label(id, label).await?;
    Ok(Some(label.to_owned()))
}

async fn daemon_knows(daemon: &Daemon, id: &str) -> Result<bool> {
    let (_, state) = client::fetch_state(daemon).await?;
    Ok(state.accounts.iter().any(|account| account.id == id))
}

pub async fn shown_owner(globals: &Globals, id: &str) -> Result<Option<CredentialOwner>> {
    let Ok(daemon) = client::require_daemon(globals).await else {
        return Ok(None);
    };
    let (_, state) = client::fetch_state(&daemon).await?;
    let shown = state.accounts.into_iter().find(|account| account.id == id);
    Ok(shown.map(|account| account.owner))
}

pub async fn rescan_if_running(globals: &Globals) -> Result<()> {
    if let Ok(daemon) = client::require_daemon(globals).await {
        daemon.rescan().await?;
    }
    Ok(())
}
