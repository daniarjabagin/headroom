use std::io::{self, Write};

use anyhow::{Result, anyhow, bail};
use headroom_core::account::{AccountRef, CredentialOwner, ProviderId};
use headroom_core::descriptor::ProviderDescriptor;
use headroom_providers::{key_accounts, registry};

use super::announce::{refresh_signed_in, shown_owner};
use super::api_key::{self, KeyTarget};
use super::cancel::{CANCELLED, Cancel};
use super::discovery::discover_local;
use super::home::headroom_home;
use super::login::{
    Console, CredentialsStamp, Launcher, LoginEvent, LoginSpec, confirm_renewal, sign_in_at,
};
use super::plan::{AddPlan, choose_login_plan};
use super::progress::{JsonLines, ProgressEvent};
use super::{ask_for_key, key_input, progress_event, signed_in_account};
use crate::cli::ProgressFormat;
use crate::paths::{Globals, accounts_root};
use crate::providers::{self, LocalRegistry};

pub struct LoginRequest<'a> {
    pub account_id: &'a str,
    pub progress: Option<ProgressFormat>,
    pub api_key_stdin: bool,
}

struct Relogin {
    account: AccountRef,
    descriptor: &'static ProviderDescriptor,
    registry: LocalRegistry,
    plan: AddPlan,
}

impl Relogin {
    async fn resolve(globals: &Globals, request: &LoginRequest<'_>) -> Result<Relogin> {
        let id = request.account_id;
        let registry = LocalRegistry::for_cli(globals)?;
        let accounts = discover_local(&registry.all()).await;
        let shown = shown_owner(globals, id).await?;
        let mut account = owned_record(&accounts, id, shown)?.clone();
        account.home = headroom_home(&accounts_root()?, &account.provider, &account.home)?;
        let descriptor = providers::descriptor(account.provider.as_str())?;
        let holds_key = key_accounts::load_record(&account.home)?.is_some();
        let keys = key_input(request.api_key_stdin, request.progress);
        let plan = choose_login_plan(descriptor, holds_key, keys)?;
        Ok(Relogin {
            account,
            descriptor,
            registry,
            plan,
        })
    }
}

pub async fn login_again(globals: &Globals, request: &LoginRequest<'_>) -> Result<()> {
    let cancel = Cancel::default();
    cancel.on_signals()?;
    match request.progress {
        None => {
            let (name, id) = relogin(globals, request, &cancel).await?;
            writeln!(io::stdout(), "Signed in to {name} account {id} again")?;
            Ok(())
        }
        Some(ProgressFormat::Json) => {
            cancel.on_stdout_closed();
            let result = relogin(globals, request, &cancel).await;
            let done = |(_, id): &(&str, String)| ProgressEvent::Done {
                account_id: id.clone(),
                label: None,
            };
            JsonLines::new(io::stdout()).finish(result, done)?;
            Ok(())
        }
    }
}

async fn relogin(
    globals: &Globals,
    request: &LoginRequest<'_>,
    cancel: &Cancel,
) -> Result<(&'static str, String)> {
    let target = Relogin::resolve(globals, request).await?;
    let renewed = match target.plan {
        AddPlan::Login(spec) => renew_with_cli(&target, spec, request.progress, cancel).await?,
        AddPlan::KeyFromStdin => {
            renew_with_key(&target, io::stdin().lock(), request, cancel).await?
        }
        AddPlan::KeyFromPrompt(prompt) => {
            let input = io::Cursor::new(ask_for_key(prompt, cancel).await?);
            renew_with_key(&target, input, request, cancel).await?
        }
    };
    if let Some(note) = identity_note(&target.account, &renewed) {
        writeln!(io::stderr(), "{note}")?;
    }
    tokio::select! {
        biased;
        () = cancel.cancelled() => bail!(CANCELLED),
        result = refresh_signed_in(globals, &renewed.id.0) => result?,
    }
    Ok((target.descriptor.display_name, renewed.id.0))
}

async fn renew_with_cli(
    target: &Relogin,
    spec: LoginSpec,
    progress: Option<ProgressFormat>,
    cancel: &Cancel,
) -> Result<AccountRef> {
    let provider = target.registry.provider(target.descriptor)?;
    let home = target.account.home.clone();
    let before = CredentialsStamp::read(&spec, &home)?;
    if progress.is_none() {
        writeln!(io::stderr(), "Running {}", spec.display(&home))?;
    }
    let (at, login) = (home.clone(), cancel.clone());
    tokio::task::spawn_blocking(move || {
        let launcher = Launcher::default();
        if progress.is_none() {
            return sign_in_at(&at, spec, &launcher, Console::Terminal, &login);
        }
        let mut out = JsonLines::new(io::stdout());
        let mut events = |event: LoginEvent| out.emit(&progress_event(spec, event));
        let console = Console::Streamed {
            input: Box::new(io::stdin()),
            events: &mut events,
        };
        sign_in_at(&at, spec, &launcher, console, &login)
    })
    .await??;
    confirm_renewal(provider.as_ref(), &spec, &home, &before).await?;
    signed_in_account(provider.as_ref(), &home).await
}

async fn renew_with_key(
    target: &Relogin,
    input: impl io::BufRead,
    request: &LoginRequest<'_>,
    cancel: &Cancel,
) -> Result<AccountRef> {
    let provider = target.registry.provider(target.descriptor)?;
    let key_target = KeyTarget {
        provider: provider.as_ref(),
        secrets: &target.registry.secrets,
        root: &accounts_root()?,
    };
    let mut out = JsonLines::new(io::stdout());
    let mut events = |event: ProgressEvent| match request.progress {
        Some(ProgressFormat::Json) => out.emit(&event),
        None => Ok(()),
    };
    api_key::renew(&key_target, &target.account, input, &mut events, cancel).await
}

fn owned_record<'a>(
    accounts: &'a [AccountRef],
    id: &str,
    shown: Option<CredentialOwner>,
) -> Result<&'a AccountRef> {
    let mut records = accounts.iter().filter(|account| account.id.0 == id);
    let first = records.clone().next();
    let preferred = shown.unwrap_or(CredentialOwner::Headroom);
    let account = records
        .find(|account| account.owner == preferred)
        .or(first)
        .ok_or_else(|| anyhow!("no signed-in account {id} found"))?;
    match account.owner {
        CredentialOwner::Headroom => Ok(account),
        CredentialOwner::Cli => bail!(cli_owned(&account.provider)),
    }
}

fn cli_owned(provider: &ProviderId) -> String {
    let Some(descriptor) = registry::descriptor(provider.as_str()) else {
        return format!("This account belongs to the {provider} CLI — sign in there again");
    };
    let name = descriptor.display_name;
    match descriptor.cli_login() {
        Some(login) => format!(
            "This account belongs to the {name} CLI — run `{}` instead",
            login.command_line()
        ),
        None => format!("This account belongs to {name} outside Headroom — sign in there again"),
    }
}

fn identity_note(previous: &AccountRef, renewed: &AccountRef) -> Option<String> {
    (previous.id != renewed.id).then(|| {
        format!(
            "Signed in with another account: {} now holds {} instead of {}; Headroom shows it as \
             a new account.",
            renewed.home.display(),
            renewed.id,
            previous.id
        )
    })
}

#[cfg(test)]
#[path = "relogin_tests.rs"]
mod tests;
