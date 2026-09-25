use async_trait::async_trait;
use jiff::SignedDuration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fetched {
    Modified { body: String, etag: Option<String> },
    NotModified,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    #[error("the status page asked Headroom to slow down")]
    RateLimited { retry_after: Option<SignedDuration> },
    #[error("{0}")]
    Failed(String),
}

#[async_trait]
pub trait StatusFetch: Send + Sync {
    async fn get(&self, url: &str, etag: Option<&str>) -> Result<Fetched, FetchError>;
}
