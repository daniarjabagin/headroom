mod accounts;
mod cli;
mod client;
mod commands;
mod daemon;
mod paths;
mod pricing;
mod providers;
mod render;
mod state;
mod waybar;

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use headroom_daemon::BusTarget;
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

use cli::{AccountsAction, Cli, Command};
use paths::Globals;

const WORKER_THREADS: usize = 2;
const MAX_BLOCKING_THREADS: usize = 4;
const BLOCKING_KEEP_ALIVE: Duration = Duration::from_secs(5);

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_logging(&cli.command);
    let globals = Globals {
        bus: cli
            .bus_address
            .map_or(BusTarget::Session, BusTarget::Address),
        db: cli.db,
    };
    let outcome = runtime()
        .context("cannot start the async runtime")
        .and_then(|runtime| runtime.block_on(dispatch(&globals, cli.command)));
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

async fn dispatch(globals: &Globals, command: Command) -> Result<ExitCode> {
    match command {
        Command::Daemon => return daemon::run(globals).await,
        Command::Status(args) => commands::status(globals, &args).await?,
        Command::Refresh(args) => commands::refresh(globals, args.account_id.as_deref()).await?,
        Command::Accounts(args) => accounts_action(globals, args.action).await?,
        Command::Waybar => waybar::run(globals).await?,
        Command::Providers(args) => providers::list(&args)?,
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
        AccountsAction::Remove { id, yes, progress } => {
            accounts::remove(globals, &id, yes, progress).await
        }
    }
}

fn init_logging(command: &Command) {
    let default = if matches!(command, Command::Daemon) {
        "info"
    } else {
        "warn"
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .with_ansi(io::stderr().is_terminal());
    if std::env::var_os("JOURNAL_STREAM").is_some() {
        builder.without_time().init();
    } else {
        builder.init();
    }
}
