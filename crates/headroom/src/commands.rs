use std::io::{self, IsTerminal, Write};

use anyhow::Result;

use crate::cli::{RefreshArgs, StatusArgs};
use crate::client;
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
    let daemon = client::require_daemon(globals).await?;
    let account_id = args.account_id.as_deref();
    let target = if args.now {
        daemon.refresh_now().await?;
        "every account now"
    } else {
        daemon.refresh(account_id.unwrap_or("")).await?;
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
    let daemon = client::require_daemon(globals).await?;
    daemon.set_account_label(id, label).await
}

pub async fn hide_account(globals: &Globals, id: &str, hidden: bool) -> Result<()> {
    let daemon = client::require_daemon(globals).await?;
    daemon.set_account_hidden(id, hidden).await
}

pub async fn order_accounts(globals: &Globals, ids: &[String]) -> Result<()> {
    let daemon = client::require_daemon(globals).await?;
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    daemon.set_account_order(&ids).await
}
