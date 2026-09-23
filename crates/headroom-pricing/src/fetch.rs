use std::path::Path;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::{ETAG, IF_NONE_MATCH};
use serde_json::Value;

use crate::cache::{self, CachedFeed, Feed};
use crate::error::{PricingError, io_error, json_error};

const TIMEOUT: Duration = Duration::from_secs(20);
const LITELLM_URL: &str =
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";
const MODELS_DEV_URL: &str = "https://models.dev/api.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sources {
    pub litellm: String,
    pub models_dev: String,
}

impl Default for Sources {
    fn default() -> Sources {
        Sources {
            litellm: LITELLM_URL.to_owned(),
            models_dev: MODELS_DEV_URL.to_owned(),
        }
    }
}

#[derive(Debug)]
pub enum FeedStatus {
    Updated,
    NotModified,
    Failed(PricingError),
}

#[derive(Debug)]
pub struct RefreshOutcome {
    pub litellm: FeedStatus,
    pub models_dev: FeedStatus,
}

pub async fn refresh(
    cache_dir: &Path,
    client: &reqwest::Client,
    sources: &Sources,
) -> Result<RefreshOutcome, PricingError> {
    tokio::fs::create_dir_all(cache_dir)
        .await
        .map_err(io_error(cache_dir))?;
    Ok(RefreshOutcome {
        litellm: refresh_feed(cache_dir, client, Feed::LiteLlm, &sources.litellm).await,
        models_dev: refresh_feed(cache_dir, client, Feed::ModelsDev, &sources.models_dev).await,
    })
}

async fn refresh_feed(dir: &Path, client: &reqwest::Client, feed: Feed, url: &str) -> FeedStatus {
    match try_refresh(dir, client, feed, url).await {
        Ok(status) => status,
        Err(error) => {
            tracing::warn!(%error, "price feed refresh failed");
            FeedStatus::Failed(error)
        }
    }
}

async fn try_refresh(
    dir: &Path,
    client: &reqwest::Client,
    feed: Feed,
    url: &str,
) -> Result<FeedStatus, PricingError> {
    let path = feed.cache_path(dir);
    let mut request = client.get(url).timeout(TIMEOUT);
    if let Some(etag) = cache::read_etag(&path, feed).await {
        request = request.header(IF_NONE_MATCH, etag);
    }
    let http_error = |source| PricingError::Http {
        url: url.to_owned(),
        source,
    };
    let response = request.send().await.map_err(http_error)?;
    let status = response.status();
    if status == StatusCode::NOT_MODIFIED {
        return Ok(FeedStatus::NotModified);
    }
    if !status.is_success() {
        return Err(PricingError::Status {
            url: url.to_owned(),
            status: status.as_u16(),
        });
    }
    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body = response.bytes().await.map_err(http_error)?;
    let cached = validated_feed(feed, &body, etag)?;
    cache::write_atomic(&path, feed, &cached).await?;
    Ok(FeedStatus::Updated)
}

fn validated_feed(
    feed: Feed,
    body: &[u8],
    etag: Option<String>,
) -> Result<CachedFeed, PricingError> {
    let document: Value = serde_json::from_slice(body).map_err(json_error(feed.source()))?;
    let data = feed.trim(&document);
    feed.parse(&data)?;
    Ok(CachedFeed { etag, data })
}

#[cfg(test)]
#[path = "fetch_tests.rs"]
mod tests;
