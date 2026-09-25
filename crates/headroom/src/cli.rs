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
    #[command(about = "Print a diagnostics report for bug reports, without secrets or emails")]
    Diagnostics,
    #[command(
        about = "Break down local spend by model, project, provider or day",
        after_help = SPEND_EXAMPLES
    )]
    Spend(SpendArgs),
}

const SPEND_EXAMPLES: &str = "\
Examples:
  headroom spend                          models of the last 7 days
  headroom spend --by project --since 30d
  headroom spend --by day --since 2026-09-01 --until 2026-09-15
  headroom spend --provider claude --json

Without a running daemon the numbers are read from its database directly.";

#[derive(Debug, Args)]
pub struct SpendArgs {
    #[arg(long, value_enum, default_value_t = SpendBy::Model, help = "How to group the spend")]
    pub by: SpendBy,
    #[arg(
        long,
        value_name = "7d|30d|YYYY-MM-DD",
        default_value = "7d",
        help = "The last 7 or 30 days, today or yesterday, or a first day"
    )]
    pub since: String,
    #[arg(
        long,
        value_name = "YYYY-MM-DD",
        help = "Last day, with a --since date; default today"
    )]
    pub until: Option<String>,
    #[arg(
        long,
        value_name = "PROVIDER",
        help = "Only this provider, see `headroom providers`"
    )]
    pub provider: Option<String>,
    #[arg(
        long,
        help = "Print the breakdown as JSON, as the daemon's GetSpend returns it"
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SpendBy {
    Model,
    Project,
    Provider,
    Day,
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
    #[command(about = "Sign a Headroom-owned account in again, in the same home")]
    Login {
        #[arg(value_name = "ID", help = "Account id, see `headroom accounts`")]
        id: String,
        #[arg(
            long,
            value_enum,
            value_name = "FORMAT",
            help = "Report progress as JSON lines on stdout instead of using the terminal"
        )]
        progress: Option<ProgressFormat>,
        #[arg(
            long,
            help = "Read the account's new API key from the first line of stdin"
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
  \x1b[1m2\x1b[0m  no fresh data for the checked limits (stale, signed out, errors),
     or the daemon is not running

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
#[path = "cli_tests.rs"]
mod tests;
