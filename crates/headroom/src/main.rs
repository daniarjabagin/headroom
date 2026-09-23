mod accounts;
mod cli;
mod client;
mod commands;
mod daemon;
mod paths;
mod pricing;
mod render;
mod state;
mod waybar;

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use headroom_daemon::BusTarget;
use tracing_subscriber::EnvFilter;

use cli::{AccountsAction, Cli, Command};
use paths::Globals;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    init_logging(&cli.command);
    let globals = Globals {
        bus: cli
            .bus_address
            .map_or(BusTarget::Session, BusTarget::Address),
        db: cli.db,
    };
    match dispatch(&globals, cli.command).await {
        Ok(code) => code,
        Err(error) => {
            let _ = writeln!(io::stderr(), "headroom: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn dispatch(globals: &Globals, command: Command) -> Result<ExitCode> {
    match command {
        Command::Daemon => return daemon::run(globals).await,
        Command::Status(args) => commands::status(globals, &args).await?,
        Command::Refresh(args) => commands::refresh(globals, args.account_id.as_deref()).await?,
        Command::Accounts(args) => accounts_action(globals, args.action).await?,
        Command::Waybar => waybar::run(globals).await?,
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
        } => accounts::add(globals, provider.kind(), label.as_deref(), progress).await,
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
