mod assess;
mod fetch;
mod incidentio;
mod indicator;
pub mod policy;
mod poller;
mod sources;
mod statuspage;

use std::collections::BTreeSet;

use headroom_core::account::ProviderId;
use jiff::Timestamp;

pub use fetch::{FetchError, Fetched, StatusFetch};
pub use indicator::Indicator;
pub use poller::run;
pub use sources::followed_providers;

use crate::model::Model;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderStatus {
    pub assessment: Assessment,
    pub fetched_at: Timestamp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Assessment {
    pub indicator: Indicator,
    pub event: Option<StatusEvent>,
}

/// The incident or maintenance behind a non-`none` indicator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEvent {
    pub title: String,
    pub stage: Option<String>,
    pub started_at: Option<Timestamp>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StatusError {
    #[error("the status page has not been read yet")]
    NotRead,
    #[error("the status page answered with JSON Headroom cannot read: {0}")]
    Unreadable(String),
}

/// Providers with at least one listed account that is not hidden.
#[must_use]
pub fn providers_in_use(model: &Model) -> BTreeSet<ProviderId> {
    model
        .active_accounts()
        .filter(|account| !account.hidden)
        .map(|account| account.reference.provider.clone())
        .collect()
}

impl From<serde_json::Error> for StatusError {
    fn from(error: serde_json::Error) -> StatusError {
        StatusError::Unreadable(error.to_string())
    }
}
