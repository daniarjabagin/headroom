use std::io::{self, IsTerminal, Write};

use anyhow::Result;

use crate::cli::{RefreshArgs, StatusArgs};
use crate::client::{self, call_error};
use crate::paths::Globals;
use crate::render::accounts::render_accounts;
use crate::render::status::render_status;
use crate::render::style::Palette;
use crate::state::{self, Source};

const CACHED_NOTICE: &str = "daemon not running, showing cached data";

pub async fn status(globals: &Globals, args: &StatusArgs) -> Result<()> {
    let loaded = state::load(globals).await?;
    if loaded.source == Source::Cache {
        writeln!(io::stderr(), "{CACHED_NOTICE}")?;
    }
    let mut stdout = io::stdout().lock();
    if args.json {
        writeln!(stdout, "{}", loaded.json)?;
    } else {
        let no_color = std::env::var_os("NO_COLOR");
        let palette = Palette::detect(no_color.as_deref(), stdout.is_terminal());
        write!(stdout, "{}", render_status(&loaded.state, palette))?;
    }
    Ok(())
}

pub async fn refresh(globals: &Globals, args: &RefreshArgs) -> Result<()> {
    let proxy = client::require_daemon(&globals.bus).await?;
    let account_id = args.account_id.as_deref();
    let target = if args.now {
        proxy.refresh_now().await.map_err(call_error)?;
        "every account now"
    } else {
        proxy
            .refresh(account_id.unwrap_or(""))
            .await
            .map_err(call_error)?;
        account_id.unwrap_or("all due accounts")
    };
    writeln!(io::stdout(), "Refresh requested for {target}")?;
    Ok(())
}

pub async fn list_accounts(globals: &Globals) -> Result<()> {
    let loaded = state::load(globals).await?;
    if loaded.source == Source::Cache {
        writeln!(io::stderr(), "{CACHED_NOTICE}")?;
    }
    write!(io::stdout(), "{}", render_accounts(&loaded.state.accounts))?;
    Ok(())
}

pub async fn label_account(globals: &Globals, id: &str, label: &str) -> Result<()> {
    let proxy = client::require_daemon(&globals.bus).await?;
    proxy.set_account_label(id, label).await.map_err(call_error)
}

pub async fn hide_account(globals: &Globals, id: &str, hidden: bool) -> Result<()> {
    let proxy = client::require_daemon(&globals.bus).await?;
    proxy
        .set_account_hidden(id, hidden)
        .await
        .map_err(call_error)
}

pub async fn order_accounts(globals: &Globals, ids: &[String]) -> Result<()> {
    let proxy = client::require_daemon(&globals.bus).await?;
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    proxy.set_account_order(&ids).await.map_err(call_error)
}
