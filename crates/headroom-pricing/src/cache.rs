use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::AsyncWriteExt;

use crate::catalog::Catalog;
use crate::error::{PricingError, io_error, json_error};
use crate::source::PriceSource;
use crate::{litellm, models_dev};

const BUNDLED_LITELLM: &str = include_str!("../resources/litellm.json");
const BUNDLED_MODELS_DEV: &str = include_str!("../resources/models_dev.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Feed {
    LiteLlm,
    ModelsDev,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CachedFeed {
    pub(crate) etag: Option<String>,
    pub(crate) data: Value,
}

impl Feed {
    pub(crate) fn source(self) -> PriceSource {
        match self {
            Feed::LiteLlm => PriceSource::LiteLlm,
            Feed::ModelsDev => PriceSource::ModelsDev,
        }
    }

    pub(crate) fn trim(self, document: &Value) -> Value {
        match self {
            Feed::LiteLlm => litellm::trim(document),
            Feed::ModelsDev => models_dev::trim(document),
        }
    }

    pub(crate) fn parse(self, document: &Value) -> Result<Catalog, PricingError> {
        match self {
            Feed::LiteLlm => litellm::parse(document),
            Feed::ModelsDev => models_dev::parse(document),
        }
    }

    pub(crate) fn bundled_catalog(self) -> Result<Catalog, PricingError> {
        let text = match self {
            Feed::LiteLlm => BUNDLED_LITELLM,
            Feed::ModelsDev => BUNDLED_MODELS_DEV,
        };
        let document: Value = serde_json::from_str(text).map_err(json_error(self.source()))?;
        self.parse(&document)
    }

    pub(crate) fn cache_path(self, dir: &Path) -> PathBuf {
        dir.join(match self {
            Feed::LiteLlm => "litellm.json",
            Feed::ModelsDev => "models_dev.json",
        })
    }
}

pub(crate) fn cached_or_bundled(dir: &Path, feed: Feed) -> Result<Catalog, PricingError> {
    let bundled = feed.bundled_catalog()?;
    match read_cached_catalog(dir, feed) {
        Ok(Some(cached)) => Ok(bundled.overlaid_with(cached)),
        Ok(None) => Ok(bundled),
        Err(error) => {
            tracing::warn!(%error, "ignoring price cache, using bundled snapshot");
            Ok(bundled)
        }
    }
}

fn read_cached_catalog(dir: &Path, feed: Feed) -> Result<Option<Catalog>, PricingError> {
    let path = feed.cache_path(dir);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error(&path)(error)),
    };
    let cached = decode(feed, &bytes)?;
    feed.parse(&cached.data).map(Some)
}

pub(crate) fn decode(feed: Feed, bytes: &[u8]) -> Result<CachedFeed, PricingError> {
    serde_json::from_slice(bytes).map_err(json_error(feed.source()))
}

pub(crate) async fn read_etag(path: &Path, feed: Feed) -> Option<String> {
    let bytes = tokio::fs::read(path).await.ok()?;
    let cached = decode(feed, &bytes).ok()?;
    feed.parse(&cached.data).ok()?;
    cached.etag
}

pub(crate) async fn write_atomic(
    path: &Path,
    feed: Feed,
    cached: &CachedFeed,
) -> Result<(), PricingError> {
    let bytes = serde_json::to_vec(cached).map_err(json_error(feed.source()))?;
    let temp = path.with_extension("json.tmp");
    let mut file = tokio::fs::File::create(&temp)
        .await
        .map_err(io_error(&temp))?;
    file.write_all(&bytes).await.map_err(io_error(&temp))?;
    file.sync_all().await.map_err(io_error(&temp))?;
    drop(file);
    tokio::fs::rename(&temp, path).await.map_err(io_error(path))
}
