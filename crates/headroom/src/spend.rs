use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use headroom_daemon::clock::SystemClock;
use headroom_daemon::home::HomeDisplay;
use headroom_daemon::usage::report::rows::SpendReport;
use headroom_daemon::usage::report::{OnceSpendContext, spend_report_once};
use headroom_pricing::PriceCatalog;
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde_json::{Map, Value};

use crate::cli::{SpendArgs, SpendBy};
use crate::client;
use crate::output;
use crate::paths::{Globals, pricing_cache_dir};
use crate::providers;
use crate::render::spend_breakdown::render_breakdown;
use crate::render::style::Palette;

mod since;

pub use since::{Since, parse_since};

const DATABASE_NOTICE: &str = "daemon not running, reading its database directly";

pub async fn run(globals: &Globals, args: &SpendArgs) -> Result<()> {
    let today = TimeZone::system().to_datetime(Timestamp::now()).date();
    let query = query(args, today);
    let (json, report) = if let Ok(Some(daemon)) = client::running_daemon(globals).await {
        let json = daemon.get_spend(&query).await?;
        let report = parse(&json)?;
        (json, report)
    } else {
        writeln!(io::stderr(), "{DATABASE_NOTICE}")?;
        let report = from_database(globals.db_path()?, pricing_cache_dir()?, query).await?;
        (serde_json::to_string(&report)?, report)
    };
    if args.json {
        return output::print_line(&json);
    }
    let no_color = std::env::var_os("NO_COLOR");
    let palette = Palette::detect(no_color.as_deref(), io::stdout().is_terminal());
    let catalog = providers::catalog();
    output::print(&render_breakdown(&report, &catalog, palette))
}

pub fn query(args: &SpendArgs, today: Date) -> String {
    let mut query = Map::new();
    args.since.insert_into(&mut query, today);
    if let Some(until) = args.until {
        query.insert("until".into(), until.to_string().into());
    }
    query.insert("by".into(), group_name(args.by).into());
    if let Some(provider) = &args.provider {
        query.insert("provider".into(), provider.trim().into());
    }
    Value::Object(query).to_string()
}

fn group_name(by: SpendBy) -> &'static str {
    match by {
        SpendBy::Model => "model",
        SpendBy::Project => "project",
        SpendBy::Provider => "provider",
        SpendBy::Day => "day",
    }
}

fn parse(json: &str) -> Result<SpendReport> {
    serde_json::from_str(json).context("the daemon sent a spend breakdown this CLI cannot read")
}

async fn from_database(db: PathBuf, pricing: PathBuf, query: String) -> Result<SpendReport> {
    tokio::task::spawn_blocking(move || read_database(&db, &pricing, &query)).await?
}

pub fn read_database(db: &Path, pricing: &Path, query: &str) -> Result<SpendReport> {
    if !db.is_file() {
        bail!("the Headroom daemon is not running and has no usage data yet");
    }
    let prices = PriceCatalog::load(pricing).context("could not load the price catalog")?;
    let homes = HomeDisplay::new(dirs::home_dir());
    let ctx = OnceSpendContext {
        price_book: &prices,
        clock: &SystemClock,
        tz: &TimeZone::system(),
        homes: &homes,
        catalog: &providers::catalog(),
    };
    Ok(spend_report_once(db, query, &ctx)?)
}

#[cfg(test)]
#[path = "spend_tests.rs"]
mod tests;
