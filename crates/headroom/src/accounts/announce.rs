use std::io::{self, Write};

use anyhow::Result;
use headroom_core::account::AccountRef;

use crate::client::{self, DaemonProxy, call_error};
use crate::paths::Globals;

const NOT_DISCOVERED: &str = "The daemon did not find this account; check `headroom accounts`.";

pub async fn announce(
    globals: &Globals,
    account: &AccountRef,
    label: Option<&str>,
) -> Result<Option<String>> {
    let id = &account.id.0;
    let Ok(proxy) = client::require_daemon(&globals.bus).await else {
        if let Some(label) = label {
            writeln!(
                io::stderr(),
                "Start the daemon, then run: headroom accounts label {id} {label:?}"
            )?;
        }
        return Ok(None);
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
        return Ok(None);
    }
    let Some(label) = label else {
        return Ok(None);
    };
    proxy
        .set_account_label(id, label)
        .await
        .map_err(call_error)?;
    Ok(Some(label.to_owned()))
}

async fn daemon_knows(proxy: &DaemonProxy<'_>, id: &str) -> Result<bool> {
    let (_, state) = client::fetch_state(proxy).await?;
    Ok(state.accounts.iter().any(|account| account.id == id))
}

pub async fn rescan_if_running(globals: &Globals) -> Result<()> {
    if let Ok(proxy) = client::require_daemon(&globals.bus).await {
        proxy.rescan().await.map_err(call_error)?;
    }
    Ok(())
}
