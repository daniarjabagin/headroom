use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "headroom",
    version,
    about = "How much of your AI coding limits is left"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
    #[cfg(target_os = "linux")]
    #[arg(long, global = true, hide = true, value_name = "ADDRESS")]
    pub bus_address: Option<String>,
    #[arg(long, global = true, hide = true, value_name = "PATH")]
    pub db: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Run the Headroom daemon")]
    Daemon(DaemonArgs),
    #[command(about = "Show limits and spend for every account")]
    Status(StatusArgs),
    #[command(about = "Ask the daemon to refresh one account, all due accounts or everything now")]
    Refresh(RefreshArgs),
    #[command(about = "List and manage accounts")]
    Accounts(AccountsArgs),
    #[command(about = "Stream JSON lines for a Waybar custom module")]
    Waybar,
    #[command(about = "List the providers this build supports")]
    Providers(ProvidersArgs),
}

#[derive(Debug, Args)]
pub struct DaemonArgs {
    #[arg(
        long,
        value_name = "PATH",
        num_args = 0..=1,
        help = "Also serve the socket API, at PATH or the default path (always on for macOS)"
    )]
    #[allow(
        clippy::option_option,
        reason = "clap's shape for a flag whose value is optional"
    )]
    pub socket: Option<Option<PathBuf>>,
}

#[derive(Debug, Args)]
pub struct ProvidersArgs {
    #[arg(
        long,
        help = "Print the list as JSON, as the daemon's ListProviders returns it"
    )]
    pub json: bool,
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
    #[command(about = "Add another account in a Headroom-owned home")]
    Add {
        #[arg(
            value_name = "PROVIDER",
            help = "Provider id, see `headroom providers`"
        )]
        provider: String,
        #[arg(long, value_name = "NAME", help = "Label to give the new account")]
        label: Option<String>,
        #[arg(
            long,
            value_enum,
            value_name = "FORMAT",
            help = "Report progress as JSON lines on stdout instead of using the terminal"
        )]
        progress: Option<ProgressFormat>,
        #[arg(
            long,
            help = "Read the provider's API key from the first line of stdin"
        )]
        api_key_stdin: bool,
    },
    #[command(
        about = "Remove an account: delete a Headroom-owned home, or stop showing a CLI account"
    )]
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
    #[command(about = "Show CLI accounts removed with `accounts remove` again")]
    Restore {
        #[arg(
            value_name = "PROVIDER",
            help = "Provider id, see `headroom providers`; all providers when omitted"
        )]
        provider: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProgressFormat {
    Json,
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

    fn daemon_args(args: &[&str]) -> DaemonArgs {
        let parsed = Cli::try_parse_from([&["headroom", "daemon"], args].concat()).unwrap();
        match parsed.command {
            Command::Daemon(daemon) => daemon,
            other => panic!("parsed {other:?}"),
        }
    }

    #[test]
    fn the_socket_flag_takes_an_optional_path() {
        assert_eq!(daemon_args(&[]).socket, None);
        assert_eq!(daemon_args(&["--socket"]).socket, Some(None));
        assert_eq!(
            daemon_args(&["--socket", "/tmp/h.sock"]).socket,
            Some(Some(PathBuf::from("/tmp/h.sock")))
        );
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
