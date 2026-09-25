use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use headroom_daemon::update::{FeedError, FeedResponse, GithubRelease, ReleaseFeed};
use jiff::{SignedDuration, Timestamp};
use reqwest::header::{ACCEPT, ETAG, HeaderMap, IF_NONE_MATCH, USER_AGENT};
use reqwest::{Client, RequestBuilder, Response, StatusCode};

use super::origins::Origins;

const AGENT: &str = concat!("headroom/", env!("CARGO_PKG_VERSION"));
const GITHUB_JSON: &str = "application/vnd.github+json";
const API_TIMEOUT: Duration = Duration::from_secs(20);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_mins(5);
const RATE_LIMIT_RESET: &str = "x-ratelimit-reset";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const API_BODY_LIMIT: usize = 1024 * 1024;

pub struct GithubFeed {
    client: Client,
    url: String,
    origins: Origins,
}

impl GithubFeed {
    pub fn new(url: impl Into<String>, origins: Origins) -> Result<GithubFeed> {
        let client = Client::builder()
            .user_agent(AGENT)
            .connect_timeout(CONNECT_TIMEOUT)
            .https_only(origins.https_only())
            .redirect(origins.redirect_policy())
            .build()
            .map_err(plain)
            .context("cannot create the HTTP client for updates")?;
        Ok(GithubFeed {
            client,
            url: url.into(),
            origins,
        })
    }

    pub async fn release(&self) -> Result<GithubRelease> {
        let response = self.api(None).send().await.map_err(unreachable)?;
        let status = response.status();
        if !status.is_success() {
            bail!("GitHub answered {status} for the latest release");
        }
        let body = read_text(response, API_BODY_LIMIT).await?;
        Ok(GithubRelease::parse(&body)?)
    }

    pub async fn download(&self, url: &str, limit: usize) -> Result<Vec<u8>> {
        if !self.origins.allows_text(url) {
            bail!("refusing to download {url}: it is not a GitHub release address over HTTPS");
        }
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
        read_capped(response, limit)
            .await
            .with_context(|| format!("could not download {url}"))
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
            let body = read_text(response, API_BODY_LIMIT)
                .await
                .map_err(|error| FeedError::Failed(format!("{error:#}")))?;
            Ok(FeedResponse::Modified { body, etag })
        }
        status => Err(FeedError::Failed(format!("GitHub answered {status}"))),
    }
}

async fn read_text(response: Response, limit: usize) -> Result<String> {
    let body = read_capped(response, limit).await?;
    String::from_utf8(body).context("GitHub answered with text that is not UTF-8")
}

async fn read_capped(mut response: Response, limit: usize) -> Result<Vec<u8>> {
    let too_large = || anyhow::anyhow!("the response is larger than {limit} bytes");
    let declared = response.content_length().unwrap_or(0);
    if usize::try_from(declared).map_or(true, |declared| declared > limit) {
        return Err(too_large());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(plain)? {
        if body.len() + chunk.len() > limit {
            return Err(too_large());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
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
    anyhow::Error::msg(error_chain(&error.without_url()))
}

fn unreachable(error: reqwest::Error) -> anyhow::Error {
    anyhow::anyhow!(
        "couldn't reach GitHub to check for updates: {}; check your connection",
        error_chain(&error.without_url())
    )
}

fn error_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

#[cfg(test)]
#[path = "feed_tests.rs"]
mod tests;
