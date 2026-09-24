mod announce;
mod ansi;
mod api_key;
mod cancel;
mod discovery;
mod dismiss;
mod home;
mod login;
mod plan;
mod progress;
mod prompt;
mod pty;
mod remove;
mod stream;

use std::io::{self, IsTerminal, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use headroom_core::account::AccountRef;
use headroom_core::descriptor::{ApiKeyPrompt, ProviderDescriptor};
use headroom_core::provider::Provider;

use crate::cli::ProgressFormat;
use crate::paths::{Globals, accounts_root};
use crate::providers::{self, LocalRegistry};
use announce::announce;
use api_key::KeyTarget;
use cancel::{CANCELLED, Cancel};
use discovery::discover_local;
use home::discard_home;
use login::{Console, Launcher, LoginEvent, LoginSpec, sign_in};
use plan::{AddPlan, KeyInput, choose_add_plan};
use progress::{JsonLines, ProgressEvent};

pub use dismiss::restore;
pub use remove::remove;

pub struct AddRequest<'a> {
    pub provider: &'a str,
    pub label: Option<&'a str>,
    pub progress: Option<ProgressFormat>,
    pub api_key_stdin: bool,
}

type Added = (String, Option<String>);

struct Target {
    descriptor: &'static ProviderDescriptor,
    registry: LocalRegistry,
    plan: AddPlan,
}

impl Target {
    fn resolve(globals: &Globals, request: &AddRequest<'_>) -> Result<Target> {
        let descriptor = providers::descriptor(request.provider)?;
        let plan = choose_add_plan(descriptor, key_input(request))?;
        let registry = LocalRegistry::for_cli(globals)?;
        Ok(Target {
            descriptor,
            registry,
            plan,
        })
    }
}

pub async fn add(globals: &Globals, request: &AddRequest<'_>) -> Result<()> {
    let cancel = Cancel::default();
    cancel.on_signals()?;
    match request.progress {
        None => add_in_terminal(globals, request, &cancel).await,
        Some(ProgressFormat::Json) => {
            cancel.on_stdout_closed();
            let result = add_streamed(globals, request, &cancel).await;
            let done = |(id, label): &Added| ProgressEvent::Done {
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
    request: &AddRequest<'_>,
    cancel: &Cancel,
) -> Result<()> {
    let target = Target::resolve(globals, request)?;
    let mut quiet = |_: ProgressEvent| Ok(());
    let (id, label) = match target.plan {
        AddPlan::Login(spec) => login_in_terminal(globals, &target, spec, request, cancel).await?,
        AddPlan::KeyFromStdin => {
            let input = io::stdin().lock();
            add_with_key(globals, &target, input, request.label, &mut quiet, cancel).await?
        }
        AddPlan::KeyFromPrompt(prompt) => {
            let input = io::Cursor::new(ask_for_key(prompt, cancel).await?);
            add_with_key(globals, &target, input, request.label, &mut quiet, cancel).await?
        }
    };
    let name = target.descriptor.display_name;
    writeln!(io::stdout(), "Added {name} account {id}")?;
    if let Some(label) = label {
        writeln!(io::stdout(), "Labelled {id} as {label}")?;
    }
    Ok(())
}

async fn add_streamed(
    globals: &Globals,
    request: &AddRequest<'_>,
    cancel: &Cancel,
) -> Result<Added> {
    let target = Target::resolve(globals, request)?;
    match target.plan {
        AddPlan::Login(spec) => login_streamed(globals, &target, spec, request, cancel).await,
        AddPlan::KeyFromStdin | AddPlan::KeyFromPrompt(_) => {
            let mut out = JsonLines::new(io::stdout());
            let mut events = |event: ProgressEvent| out.emit(&event);
            let input = io::stdin().lock();
            add_with_key(globals, &target, input, request.label, &mut events, cancel).await
        }
    }
}

fn key_input(request: &AddRequest<'_>) -> KeyInput {
    if request.api_key_stdin {
        KeyInput::Stdin
    } else if request.progress.is_none() && io::stdin().is_terminal() {
        KeyInput::Terminal
    } else {
        KeyInput::Unavailable
    }
}

async fn ask_for_key(prompt: &ApiKeyPrompt, cancel: &Cancel) -> Result<String> {
    let mut stderr = io::stderr();
    writeln!(
        stderr,
        "Get a key ({}) at {}",
        prompt.label, prompt.console_url
    )?;
    if !prompt.hint.is_empty() {
        writeln!(stderr, "{}", prompt.hint)?;
    }
    prompt::read_hidden(io::stdin(), &mut stderr, cancel).await
}

async fn login_in_terminal(
    globals: &Globals,
    target: &Target,
    spec: LoginSpec,
    request: &AddRequest<'_>,
    cancel: &Cancel,
) -> Result<Added> {
    let provider = target.registry.provider(target.descriptor)?;
    let root = accounts_root()?;
    let preview = root.join(spec.provider.as_str()).join("<new>");
    writeln!(io::stderr(), "Running {}", spec.display(&preview))?;
    let launcher = Launcher::default();
    let login = cancel.clone();
    let home = tokio::task::spawn_blocking(move || {
        sign_in(&root, spec, &launcher, Console::Terminal, &login)
    })
    .await??;
    register(
        globals,
        target,
        provider.as_ref(),
        &home,
        request.label,
        cancel,
    )
    .await
}

async fn login_streamed(
    globals: &Globals,
    target: &Target,
    spec: LoginSpec,
    request: &AddRequest<'_>,
    cancel: &Cancel,
) -> Result<Added> {
    let provider = target.registry.provider(target.descriptor)?;
    let root = accounts_root()?;
    let login = cancel.clone();
    let home = tokio::task::spawn_blocking(move || {
        let mut out = JsonLines::new(io::stdout());
        let mut events = |event: LoginEvent| out.emit(&progress_event(spec, event));
        let console = Console::Streamed {
            input: Box::new(io::stdin()),
            events: &mut events,
        };
        sign_in(&root, spec, &Launcher::default(), console, &login)
    })
    .await??;
    register(
        globals,
        target,
        provider.as_ref(),
        &home,
        request.label,
        cancel,
    )
    .await
}

async fn register(
    globals: &Globals,
    target: &Target,
    provider: &dyn Provider,
    home: &Path,
    label: Option<&str>,
    cancel: &Cancel,
) -> Result<Added> {
    let registered = async {
        let account = signed_in_account(provider, home).await?;
        warn_if_duplicate(target, &account).await?;
        let labelled = announce(globals, &account, label).await?;
        Ok((account.id.0, labelled))
    };
    tokio::select! {
        biased;
        () = cancel.cancelled() => {
            discard_home(home);
            bail!(CANCELLED)
        }
        result = registered => result,
    }
}

async fn add_with_key(
    globals: &Globals,
    target: &Target,
    input: impl io::BufRead,
    label: Option<&str>,
    events: &mut dyn FnMut(ProgressEvent) -> Result<()>,
    cancel: &Cancel,
) -> Result<Added> {
    let provider = target.registry.provider(target.descriptor)?;
    let key_target = KeyTarget {
        provider: provider.as_ref(),
        secrets: &target.registry.secrets,
        root: &accounts_root()?,
    };
    api_key::add(globals, &key_target, input, label, events, cancel).await
}

fn progress_event(spec: LoginSpec, event: LoginEvent) -> ProgressEvent {
    match event {
        LoginEvent::Started(home) => ProgressEvent::Started {
            provider: spec.provider.clone(),
            home: home.display().to_string(),
        },
        LoginEvent::Output(line) => ProgressEvent::Output { line },
        LoginEvent::Url(url) => ProgressEvent::Url { url },
    }
}

async fn signed_in_account(provider: &dyn Provider, home: &Path) -> Result<AccountRef> {
    provider.account_at(home).await?.with_context(|| {
        format!(
            "signed in, but Headroom cannot read the account in {}",
            home.display()
        )
    })
}

async fn warn_if_duplicate(target: &Target, account: &AccountRef) -> Result<()> {
    let existing = discover_local(&target.registry.all())
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
