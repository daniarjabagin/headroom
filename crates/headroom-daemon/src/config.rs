use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use headroom_core::provider::Provider;
use headroom_core::usage::PriceBook;
use jiff::tz::TimeZone;

use crate::catalog::ProviderCatalog;
use crate::clock::{Clock, SystemClock};
use crate::daemon::log_level::LogControl;
use crate::error::DaemonError;
use crate::notify::text::Locale;
use crate::update::UpdateConfig;

pub type Shutdown = Pin<Box<dyn Future<Output = ()> + Send>>;

#[cfg(target_os = "macos")]
const DATA_DIR_NAME: &str = "Headroom";
#[cfg(not(target_os = "macos"))]
const DATA_DIR_NAME: &str = "headroom";
const STATE_DIR_NAME: &str = "headroom";

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusTarget {
    Session,
    Address(String),
}

pub struct DaemonConfig {
    pub providers: Vec<Arc<dyn Provider>>,
    pub catalog: ProviderCatalog,
    pub price_book: Arc<dyn PriceBook>,
    pub db_path: PathBuf,
    pub clock: Arc<dyn Clock>,
    pub tz: TimeZone,
    #[cfg(target_os = "linux")]
    pub bus: BusTarget,
    pub socket: Option<PathBuf>,
    pub system_locale: Locale,
    pub updates: Option<UpdateConfig>,
    pub logging: Option<Arc<dyn LogControl>>,
    pub shutdown: Shutdown,
}

impl DaemonConfig {
    pub fn new(
        providers: Vec<Arc<dyn Provider>>,
        price_book: Arc<dyn PriceBook>,
        shutdown: Shutdown,
    ) -> Result<DaemonConfig, DaemonError> {
        Ok(DaemonConfig {
            catalog: ProviderCatalog::of_providers(&providers),
            providers,
            price_book,
            db_path: default_db_path()?,
            clock: Arc::new(SystemClock),
            tz: TimeZone::system(),
            #[cfg(target_os = "linux")]
            bus: BusTarget::Session,
            #[cfg(target_os = "linux")]
            socket: None,
            #[cfg(not(target_os = "linux"))]
            socket: Some(crate::ipc::default_socket_path()?),
            system_locale: Locale::from_env(),
            updates: None,
            logging: None,
            shutdown,
        })
    }
}

pub fn default_db_path() -> Result<PathBuf, DaemonError> {
    Ok(app_dir()?.join("headroom.db"))
}

pub fn app_dir() -> Result<PathBuf, DaemonError> {
    dirs::state_dir()
        .map(|state| state.join(STATE_DIR_NAME))
        .or_else(|| dirs::data_dir().map(|data| data.join(DATA_DIR_NAME)))
        .ok_or(DaemonError::NoStateDir)
}
