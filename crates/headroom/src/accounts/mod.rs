mod ansi;
mod cancel;
mod discovery;
mod home;
mod login;
mod progress;
mod stream;

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use headroom_core::account::{AccountRef, CredentialOwner, ProviderKind};

use crate::cli::ProgressFormat;
use crate::client::{self, DaemonProxy, call_error};
use crate::paths::{Globals, accounts_root};
use crate::render::format::provider_name;
use cancel::{CANCELLED, Cancel};
use discovery::{account_at, discover_local};
use home::{discard_home, headroom_home};
use login::{Console, Launcher, LoginEvent, login_spec, sign_in};
use progress::{JsonLines, ProgressEvent};

const NOT_DISCOVERED: &str = "The daemon did not find this account; check `headroom accounts`.";

pub async fn add(
    globals: &Globals,
    provider: ProviderKind,
    label: Option<&str>,
    progress: Option<ProgressFormat>,
) -> Result<()> {
    let cancel = Cancel::default();
    cancel.on_signals()?;
    match progress {
        None => add_in_terminal(globals, provider, label, &cancel).await,
        Some(ProgressFormat::Json) => {
            cancel.on_stdout_closed();
            let result = add_streamed(globals, provider, label, &cancel).await;
            let done = |(id, label): &(String, Option<String>)| ProgressEvent::Done {
                account_id: id.clone(),
                label: label.clone(),
            };
            JsonLines::new(io::stdout()).finish(result, done)?;
            Ok(())
        }
    }
}

async fn add_in_terminal(
    globals: &Globals,
    provider: ProviderKind,
    label: Option<&str>,
    cancel: &Cancel,
) -> Result<()> {
    let root = accounts_root()?;
    let preview = root.join(provider.as_str()).join("<new>");
    writeln!(
        io::stderr(),
        "Running {}",
        login_spec(provider).display(&preview)
    )?;
    let launcher = Launcher::default();
    let login = cancel.clone();
    let home = tokio::task::spawn_blocking(move || {
        sign_in(&root, provider, &launcher, Console::Terminal, &login)
    })
    .await??;
    let registered = async {
        let account = signed_in_account(provider, &home).await?;
        writeln!(
            io::stdout(),
            "Added {} account {}",
            provider_name(provider),
            account.id.0
        )?;
        warn_if_duplicate(&account).await?;
        if let Some(label) = announce(globals, &account, label).await? {
            writeln!(io::stdout(), "Labelled {} as {label}", account.id.0)?;
        }
        Ok(())
    };
    unless_cancelled(cancel, &home, registered).await
}

async fn add_streamed(
    globals: &Globals,
    provider: ProviderKind,
    label: Option<&str>,
    cancel: &Cancel,
) -> Result<(String, Option<String>)> {
    let root = accounts_root()?;
    let login = cancel.clone();
    let home = tokio::task::spawn_blocking(move || {
        let mut out = JsonLines::new(io::stdout());
        let mut events = |event: LoginEvent| out.emit(&progress_event(provider, event));
        let console = Console::Streamed {
            input: Box::new(io::stdin()),
            events: &mut events,
        };
        sign_in(&root, provider, &Launcher::default(), console, &login)
    })
    .await??;
    let registered = async {
        let account = signed_in_account(provider, &home).await?;
        warn_if_duplicate(&account).await?;
        let labelled = announce(globals, &account, label).await?;
        Ok((account.id.0, labelled))
    };
    unless_cancelled(cancel, &home, registered).await
}

async fn unless_cancelled<T>(
    cancel: &Cancel,
    home: &Path,
    work: impl Future<Output = Result<T>>,
) -> Result<T> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => {
            discard_home(home);
            bail!(CANCELLED)
        }
        result = work => result,
    }
}

fn progress_event(provider: ProviderKind, event: LoginEvent) -> ProgressEvent {
    match event {
        LoginEvent::Started(home) => ProgressEvent::Started {
            provider,
            home: home.display().to_string(),
        },
        LoginEvent::Output(line) => ProgressEvent::Output { line },
        LoginEvent::Url(url) => ProgressEvent::Url { url },
    }
}

async fn signed_in_account(provider: ProviderKind, home: &Path) -> Result<AccountRef> {
    account_at(provider, home).await?.with_context(|| {
        format!(
            "signed in, but Headroom cannot read the account in {}",
            home.display()
        )
    })
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

async fn announce(
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
    if !approve(&home)? {
        bail!("cancelled");
    }
    std::fs::remove_dir_all(&home)
        .with_context(|| format!("could not delete {}", home.display()))?;
    if let Ok(proxy) = client::require_daemon(&globals.bus).await {
        proxy.rescan().await.map_err(call_error)?;
    }
    Ok(home)
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
