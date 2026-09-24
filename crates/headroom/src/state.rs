use anyhow::{Context, Result, bail};
use headroom_daemon::clock::SystemClock;
use headroom_daemon::home::HomeDisplay;
use headroom_daemon::state::payload::StatePayload;
use headroom_daemon::{OnceContext, assemble_state_once};
use headroom_pricing::PriceCatalog;
use jiff::tz::TimeZone;

use crate::client;
use crate::paths::{Globals, pricing_cache_dir};
use crate::providers;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Daemon,
    Cache,
}

pub struct LoadedState {
    pub source: Source,
    pub json: String,
    pub state: StatePayload,
}

pub async fn load(globals: &Globals) -> Result<LoadedState> {
    if let Ok(Some(daemon)) = client::running_daemon(globals).await {
        let (json, state) = client::fetch_state(&daemon).await?;
        return Ok(LoadedState {
            source: Source::Daemon,
            json,
            state,
        });
    }
    let state = tokio::task::spawn_blocking(cached_state(globals)?).await??;
    Ok(LoadedState {
        source: Source::Cache,
        json: serde_json::to_string(&state)?,
        state,
    })
}

fn cached_state(globals: &Globals) -> Result<impl FnOnce() -> Result<StatePayload> + use<>> {
    let db = globals.db_path()?;
    let pricing = pricing_cache_dir()?;
    Ok(move || {
        if !db.is_file() {
            bail!("the Headroom daemon is not running and has no cached data yet");
        }
        let prices = PriceCatalog::load(&pricing).context("could not load the price catalog")?;
        let homes = HomeDisplay::new(dirs::home_dir());
        let ctx = OnceContext {
            price_book: &prices,
            clock: &SystemClock,
            tz: &TimeZone::system(),
            homes: &homes,
            catalog: &providers::catalog(),
        };
        assemble_state_once(&db, &ctx).context("could not read cached data")
    })
}
