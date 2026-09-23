use std::io;
use std::path::{Path, PathBuf};

use crate::source::PriceSource;

#[derive(Debug, thiserror::Error)]
pub enum PricingError {
    #[error("invalid {feed} JSON: {source}")]
    Json {
        feed: PriceSource,
        #[source]
        source: serde_json::Error,
    },
    #[error("{feed} catalog has no usable models")]
    EmptyCatalog { feed: PriceSource },
    #[error("invalid rate {value} for {field}")]
    InvalidRate { field: String, value: f64 },
    #[error("supplement entry {model} lacks an input or output price")]
    IncompleteRates { model: String },
    #[error("invalid alias pattern {pattern}: {source}")]
    AliasPattern {
        pattern: String,
        #[source]
        source: regex::Error,
    },
    #[error("cannot access {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("request to {url} failed: {source}")]
    Http {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("{url} answered HTTP {status}")]
    Status { url: String, status: u16 },
}

pub(crate) fn io_error(path: &Path) -> impl Fn(io::Error) -> PricingError + '_ {
    move |source| PricingError::Io {
        path: path.to_path_buf(),
        source,
    }
}

pub(crate) fn json_error(feed: PriceSource) -> impl Fn(serde_json::Error) -> PricingError {
    move |source| PricingError::Json { feed, source }
}
