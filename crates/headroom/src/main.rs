mod accounts;
mod cli;
mod client;
mod commands;
mod daemon;
mod diagnostics;
mod guard;
mod logging;
mod paths;
mod pricing;
mod providers;
mod render;
mod spend;
mod state;
mod status_pages;
mod terminal;
mod update;
mod waybar;

use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
#[cfg(target_os = "linux")]
use headroom_daemon::BusTarget;
use tokio::runtime::Runtime;

use cli::{AccountsAction, Cli, Command};
use logging::DaemonLogging;
use paths::Globals;

const WORKER_THREADS: usize = 2;
const MAX_BLOCKING_THREADS: usize = 4;
const BLOCKING_KEEP_ALIVE: Duration = Duration::from_secs(5);

fn main() -> ExitCode {
    let cli = Cli::parse();
    let logging = logging::init(&cli.command);
    let globals = Globals {
        #[cfg(target_os = "linux")]
        bus: cli
            .bus_address
            .map_or(BusTarget::Session, BusTarget::Address),
        db: cli.db,
        socket: paths::socket_override(std::env::var_os(headroom_daemon::SOCKET_ENV)),
    };
    let outcome = runtime()
        .context("cannot start the async runtime")
        .and_then(|runtime| runtime.block_on(dispatch(&globals, cli.command, logging)));
    match outcome {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(io::stderr(), "headroom: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn runtime() -> io::Result<Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(WORKER_THREADS)
        .max_blocking_threads(MAX_BLOCKING_THREADS)
        .thread_keep_alive(BLOCKING_KEEP_ALIVE)
        .enable_all()
        .build()
}

async fn dispatch(
    globals: &Globals,
    command: Command,
    logging: Option<Arc<DaemonLogging>>,
) -> Result<ExitCode> {
    match command {
        Command::Daemon(args) => return daemon::run(globals, args, logging).await,
        Command::Status(args) => commands::status(globals, &args).await?,
        Command::Refresh(args) => commands::refresh(globals, &args).await?,
        Command::Accounts(args) => accounts_action(globals, args.action).await?,
        Command::Waybar(args) => waybar::run(globals, &args).await?,
        Command::Guard(args) => return guard::run(globals, &args).await,
        Command::Providers(args) => providers::list(&args)?,
        Command::Update(args) => update::run(&args).await?,
        Command::Diagnostics => diagnostics::print(globals).await?,
        Command::Spend(args) => spend::run(globals, &args).await?,
    }
    Ok(ExitCode::SUCCESS)
}

async fn accounts_action(globals: &Globals, action: Option<AccountsAction>) -> Result<()> {
    match action.unwrap_or(AccountsAction::List) {
        AccountsAction::List => commands::list_accounts(globals).await,
        AccountsAction::Label { id, label } => commands::label_account(globals, &id, &label).await,
        AccountsAction::Hide { id } => commands::hide_account(globals, &id, true).await,
        AccountsAction::Show { id } => commands::hide_account(globals, &id, false).await,
        AccountsAction::Order { ids } => commands::order_accounts(globals, &ids).await,
        AccountsAction::Add {
            provider,
            label,
            progress,
            api_key_stdin,
        } => {
            let request = accounts::AddRequest {
                provider: &provider,
                label: label.as_deref(),
                progress,
                api_key_stdin,
            };
            accounts::add(globals, &request).await
        }
        AccountsAction::Login {
            id,
            progress,
            api_key_stdin,
        } => {
            let request = accounts::LoginRequest {
                account_id: &id,
                progress,
                api_key_stdin,
            };
            accounts::login_again(globals, &request).await
        }
        AccountsAction::Remove { id, yes, progress } => {
            accounts::remove(globals, &id, yes, progress).await
        }
        AccountsAction::Restore { provider } => {
            accounts::restore(globals, provider.as_deref()).await
        }
    }
}
