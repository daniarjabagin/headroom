use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use headroom_core::account::ProviderKind;

#[derive(Debug, Parser)]
#[command(
    name = "headroom",
    version,
    about = "How much of your AI coding limits is left"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
    #[arg(long, global = true, hide = true, value_name = "ADDRESS")]
    pub bus_address: Option<String>,
    #[arg(long, global = true, hide = true, value_name = "PATH")]
    pub db: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Run the Headroom daemon on the session bus")]
    Daemon,
    #[command(about = "Show limits and spend for every account")]
    Status(StatusArgs),
    #[command(about = "Ask the daemon to refresh one account, all due accounts or everything now")]
    Refresh(RefreshArgs),
    #[command(about = "List and manage accounts")]
    Accounts(AccountsArgs),
    #[command(about = "Stream JSON lines for a Waybar custom module")]
    Waybar,
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    #[arg(long, help = "Print the raw state payload as JSON")]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct RefreshArgs {
    #[arg(
        value_name = "ACCOUNT_ID",
        help = "Account to refresh now; all due accounts when omitted"
    )]
    pub account_id: Option<String>,
    #[arg(
        long,
        conflicts_with = "account_id",
        help = "Refresh every account now, even if refreshed within the last minute"
    )]
    pub now: bool,
}

#[derive(Debug, Args)]
pub struct AccountsArgs {
    #[command(subcommand)]
    pub action: Option<AccountsAction>,
}

#[derive(Debug, Subcommand)]
pub enum AccountsAction {
    #[command(about = "List accounts (default)")]
    List,
    #[command(about = "Set an account label; an empty label clears it")]
    Label { id: String, label: String },
    #[command(about = "Hide an account from the panel and notifications")]
    Hide { id: String },
    #[command(about = "Show a hidden account again")]
    Show { id: String },
    #[command(about = "Move accounts to the front, in the given order")]
    Order {
        #[arg(required = true, value_name = "ID")]
        ids: Vec<String>,
    },
    #[command(about = "Sign in to another account in a Headroom-owned home")]
    Add {
        provider: ProviderArg,
        #[arg(long, value_name = "NAME", help = "Label to give the new account")]
        label: Option<String>,
        #[arg(
            long,
            value_enum,
            value_name = "FORMAT",
            help = "Report progress as JSON lines on stdout instead of using the terminal"
        )]
        progress: Option<ProgressFormat>,
    },
    #[command(about = "Delete the home of an account added with `accounts add`")]
    Remove {
        id: String,
        #[arg(long, help = "Do not ask for confirmation")]
        yes: bool,
        #[arg(
            long,
            value_enum,
            value_name = "FORMAT",
            help = "Report the outcome as JSON lines on stdout"
        )]
        progress: Option<ProgressFormat>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProgressFormat {
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProviderArg {
    Codex,
    Claude,
}

impl ProviderArg {
    pub fn kind(self) -> ProviderKind {
        match self {
            ProviderArg::Codex => ProviderKind::Codex,
            ProviderArg::Claude => ProviderKind::Claude,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refresh_args(args: &[&str]) -> Result<RefreshArgs, clap::Error> {
        let parsed = Cli::try_parse_from([&["headroom", "refresh"], args].concat())?;
        match parsed.command {
            Command::Refresh(refresh) => Ok(refresh),
            other => panic!("parsed {other:?}"),
        }
    }

    #[test]
    fn refresh_now_is_a_flag_that_excludes_an_account_id() {
        let now = refresh_args(&["--now"]).unwrap();
        assert!(now.now && now.account_id.is_none());
        let due = refresh_args(&[]).unwrap();
        assert!(!due.now && due.account_id.is_none());
        let one = refresh_args(&["codex:work"]).unwrap();
        assert_eq!(one.account_id.as_deref(), Some("codex:work"));
        assert!(refresh_args(&["--now", "codex:work"]).is_err());
    }
}
