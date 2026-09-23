use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use headroom_core::provider::Provider;
use headroom_core::usage::PriceBook;
use jiff::tz::TimeZone;

use crate::clock::{Clock, SystemClock};
use crate::error::DaemonError;

pub type Shutdown = Pin<Box<dyn Future<Output = ()> + Send>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusTarget {
    Session,
    Address(String),
}

pub struct DaemonConfig {
    pub providers: Vec<Arc<dyn Provider>>,
    pub price_book: Arc<dyn PriceBook>,
    pub db_path: PathBuf,
    pub clock: Arc<dyn Clock>,
    pub tz: TimeZone,
    pub bus: BusTarget,
    pub shutdown: Shutdown,
}

impl DaemonConfig {
    pub fn new(
        providers: Vec<Arc<dyn Provider>>,
        price_book: Arc<dyn PriceBook>,
        shutdown: Shutdown,
    ) -> Result<DaemonConfig, DaemonError> {
        Ok(DaemonConfig {
            providers,
            price_book,
            db_path: default_db_path()?,
            clock: Arc::new(SystemClock),
            tz: TimeZone::system(),
            bus: BusTarget::Session,
            shutdown,
        })
    }
}

pub fn default_db_path() -> Result<PathBuf, DaemonError> {
    let state = dirs::state_dir().ok_or(DaemonError::NoStateDir)?;
    Ok(state.join("headroom").join("headroom.db"))
}
