use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

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
    Waybar(WaybarArgs),
    #[command(
        about = "Exit non-zero when a limit has less than a given share left",
        after_help = GUARD_AFTER_HELP
    )]
    Guard(GuardArgs),
    #[command(about = "List the providers this build supports")]
    Providers(ProvidersArgs),
    #[command(about = "Check for a new Headroom release and install it")]
    Update(UpdateArgs),
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
    #[arg(
        long,
        help = "Never ask GitHub for new Headroom releases (for bundles that update themselves)"
    )]
    pub no_update_check: bool,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    #[arg(
        long,
        conflicts_with_all = ["yes", "progress"],
        help = "Only compare this version with the latest release"
    )]
    pub check: bool,
    #[arg(long, help = "Do not ask for confirmation")]
    pub yes: bool,
    #[arg(
        long,
        value_enum,
        value_name = "FORMAT",
        help = "Report progress as JSON lines on stdout"
    )]
    pub progress: Option<ProgressFormat>,
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

const GUARD_AFTER_HELP: &str = "\
\x1b[1m\x1b[4mExit status:\x1b[0m
  \x1b[1m0\x1b[0m  every checked limit has at least PERCENT left
  \x1b[1m1\x1b[0m  at least one checked limit is below PERCENT
  \x1b[1m2\x1b[0m  no data for the checked limits, or the daemon is not running

\x1b[1m\x1b[4mExamples:\x1b[0m
  headroom guard --min 15 && claude -p \"fix the failing tests\"
  headroom guard --min 25 --window weekly --provider codex --quiet || exit 0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowScope {
    Session,
    Weekly,
    Any,
}

impl WindowScope {
    pub fn includes(self, window_id: &str) -> bool {
        match self {
            WindowScope::Session => window_id == "session",
            WindowScope::Weekly => window_id == "weekly",
            WindowScope::Any => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LabelStyle {
    Full,
    None,
}

#[derive(Debug, Default, Args)]
pub struct WaybarArgs {
    #[arg(
        long,
        value_name = "ID,…",
        value_delimiter = ',',
        help = "One value per provider, in this order [default: every visible provider]"
    )]
    pub providers: Vec<String>,
    #[arg(
        long,
        value_enum,
        value_name = "WINDOW",
        help = "Limits to pick each value from [default: any]"
    )]
    pub window: Option<WindowScope>,
    #[arg(
        long,
        value_enum,
        value_name = "LABELS",
        help = "Show provider names before values [default: full]"
    )]
    pub labels: Option<LabelStyle>,
}

impl WaybarArgs {
    pub fn is_headline(&self) -> bool {
        self.providers.is_empty() && self.window.is_none() && self.labels.is_none()
    }
}

#[derive(Debug, Args)]
pub struct GuardArgs {
    #[arg(
        long,
        value_name = "PERCENT",
        value_parser = clap::value_parser!(u8).range(0..=100),
        help = "Fail when less than PERCENT is left (0-100)"
    )]
    pub min: u8,
    #[arg(
        long,
        value_enum,
        value_name = "WINDOW",
        default_value_t = WindowScope::Any,
        help = "Limits to check"
    )]
    pub window: WindowScope,
    #[arg(
        long,
        value_name = "PROVIDER",
        value_delimiter = ',',
        help = "Only check this provider, see `headroom providers`"
    )]
    pub provider: Vec<String>,
    #[arg(
        long,
        value_name = "ID",
        help = "Only check this account, see `headroom accounts`"
    )]
    pub account: Option<String>,
    #[arg(long, help = "Print the verdict and every checked limit as JSON")]
    pub json: bool,
    #[arg(
        short,
        long,
        conflicts_with = "json",
        help = "Print nothing, only set the exit status"
    )]
    pub quiet: bool,
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

    fn update_args(args: &[&str]) -> Result<UpdateArgs, clap::Error> {
        let parsed = Cli::try_parse_from([&["headroom", "update"], args].concat())?;
        match parsed.command {
            Command::Update(update) => Ok(update),
            other => panic!("parsed {other:?}"),
        }
    }

    #[test]
    fn update_check_excludes_installing() {
        let install = update_args(&["--yes", "--progress", "json"]).unwrap();
        assert!(install.yes && !install.check);
        assert_eq!(install.progress, Some(ProgressFormat::Json));
        assert!(update_args(&["--check"]).unwrap().check);
        assert!(update_args(&["--check", "--yes"]).is_err());
        assert!(update_args(&["--check", "--progress", "json"]).is_err());
    }

    #[test]
    fn update_checks_can_be_turned_off_for_the_daemon() {
        assert!(!daemon_args(&[]).no_update_check);
        assert!(daemon_args(&["--no-update-check"]).no_update_check);
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

    fn guard_args(args: &[&str]) -> Result<GuardArgs, clap::Error> {
        let parsed = Cli::try_parse_from([&["headroom", "guard"], args].concat())?;
        match parsed.command {
            Command::Guard(guard) => Ok(guard),
            other => panic!("parsed {other:?}"),
        }
    }

    fn waybar_args(args: &[&str]) -> WaybarArgs {
        let parsed = Cli::try_parse_from([&["headroom", "waybar"], args].concat()).unwrap();
        match parsed.command {
            Command::Waybar(waybar) => waybar,
            other => panic!("parsed {other:?}"),
        }
    }

    #[test]
    fn guard_needs_a_minimum_between_0_and_100() {
        assert!(guard_args(&[]).is_err());
        assert!(guard_args(&["--min", "101"]).is_err());
        let guard = guard_args(&["--min", "20"]).unwrap();
        assert_eq!((guard.min, guard.window), (20, WindowScope::Any));
        assert!(guard.provider.is_empty() && guard.account.is_none());
        assert!(!guard.json && !guard.quiet);
        assert_eq!(guard_args(&["--min", "0"]).unwrap().min, 0);
    }

    #[test]
    fn guard_providers_repeat_or_split_on_commas() {
        let guard = guard_args(&[
            "--min",
            "5",
            "--provider",
            "claude,codex",
            "--provider",
            "grok",
            "--window",
            "weekly",
            "--account",
            "codex:work",
            "-q",
        ])
        .unwrap();
        assert_eq!(guard.provider, ["claude", "codex", "grok"]);
        assert_eq!(guard.window, WindowScope::Weekly);
        assert_eq!(guard.account.as_deref(), Some("codex:work"));
        assert!(guard.quiet);
        assert!(guard_args(&["--min", "5", "--json", "--quiet"]).is_err());
    }

    #[test]
    fn waybar_without_flags_keeps_the_headline() {
        assert!(waybar_args(&[]).is_headline());
        let multi = waybar_args(&["--providers", "claude,codex", "--labels", "none"]);
        assert!(!multi.is_headline());
        assert_eq!(multi.providers, ["claude", "codex"]);
        assert_eq!(multi.labels, Some(LabelStyle::None));
        assert!(!waybar_args(&["--window", "session"]).is_headline());
    }

    #[test]
    fn window_scopes_match_window_ids() {
        assert!(WindowScope::Session.includes("session"));
        assert!(!WindowScope::Session.includes("weekly"));
        assert!(WindowScope::Weekly.includes("weekly"));
        assert!(!WindowScope::Weekly.includes("model:opus"));
        assert!(WindowScope::Any.includes("model:opus"));
    }
}
