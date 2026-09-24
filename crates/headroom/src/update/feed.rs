use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use headroom_daemon::update::{FeedError, FeedResponse, GithubRelease, ReleaseFeed};
use jiff::{SignedDuration, Timestamp};
use reqwest::header::{ACCEPT, ETAG, HeaderMap, IF_NONE_MATCH, USER_AGENT};
use reqwest::{Client, RequestBuilder, Response, StatusCode};

const AGENT: &str = concat!("headroom/", env!("CARGO_PKG_VERSION"));
const GITHUB_JSON: &str = "application/vnd.github+json";
const API_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_mins(5);
const RATE_LIMIT_RESET: &str = "x-ratelimit-reset";

pub struct GithubFeed {
    client: Client,
    url: String,
}

impl GithubFeed {
    pub fn new(client: Client, url: impl Into<String>) -> GithubFeed {
        GithubFeed {
            client,
            url: url.into(),
        }
    }

    pub async fn release(&self) -> Result<GithubRelease> {
        let response = self.api(None).send().await.map_err(plain)?;
        let status = response.status();
        if !status.is_success() {
            bail!("GitHub answered {status} for the latest release");
        }
        let body = response.text().await.map_err(plain)?;
        Ok(GithubRelease::parse(&body)?)
    }

    pub async fn download(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .header(USER_AGENT, AGENT)
            .timeout(DOWNLOAD_TIMEOUT)
            .send()
            .await
            .map_err(plain)
            .with_context(|| format!("could not download {url}"))?;
        let status = response.status();
        if !status.is_success() {
            bail!("could not download {url}: the server answered {status}");
        }
        Ok(response.bytes().await.map_err(plain)?.to_vec())
    }

    fn api(&self, etag: Option<&str>) -> RequestBuilder {
        let request = self
            .client
            .get(&self.url)
            .header(USER_AGENT, AGENT)
            .header(ACCEPT, GITHUB_JSON)
            .timeout(API_TIMEOUT);
        match etag {
            Some(etag) => request.header(IF_NONE_MATCH, etag),
            None => request,
        }
    }
}

#[async_trait]
impl ReleaseFeed for GithubFeed {
    async fn latest(&self, etag: Option<&str>) -> Result<FeedResponse, FeedError> {
        let response = self.api(etag).send().await.map_err(failed)?;
        interpret(response, Timestamp::now()).await
    }
}

async fn interpret(response: Response, now: Timestamp) -> Result<FeedResponse, FeedError> {
    let status = response.status();
    match status {
        StatusCode::NOT_MODIFIED => Ok(FeedResponse::NotModified),
        StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => Err(FeedError::RateLimited {
            retry_after: rate_limit_wait(response.headers(), now),
        }),
        status if status.is_success() => {
            let etag = header_text(response.headers(), ETAG.as_str());
            let body = response.text().await.map_err(failed)?;
            Ok(FeedResponse::Modified { body, etag })
        }
        status => Err(FeedError::Failed(format!("GitHub answered {status}"))),
    }
}

fn rate_limit_wait(headers: &HeaderMap, now: Timestamp) -> Option<SignedDuration> {
    headroom_providers::http::retry_after(headers, now).or_else(|| {
        let reset: i64 = header_text(headers, RATE_LIMIT_RESET)?.parse().ok()?;
        let at = Timestamp::from_second(reset).ok()?;
        Some(at.duration_since(now).max(SignedDuration::ZERO))
    })
}

fn header_text(headers: &HeaderMap, name: &str) -> Option<String> {
    Some(headers.get(name)?.to_str().ok()?.trim().to_owned())
}

fn failed(error: reqwest::Error) -> FeedError {
    FeedError::Failed(error.without_url().to_string())
}

fn plain(error: reqwest::Error) -> anyhow::Error {
    anyhow::Error::msg(error.without_url().to_string())
}

#[cfg(test)]
#[path = "feed_tests.rs"]
mod tests;
