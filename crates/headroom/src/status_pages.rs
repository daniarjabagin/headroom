use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use headroom_daemon::status::{FetchError, Fetched, StatusFetch};
use jiff::Timestamp;
use reqwest::header::{ACCEPT, ETAG, IF_NONE_MATCH};
use reqwest::redirect::Policy;
use reqwest::{Client, Response, StatusCode};

const AGENT: &str = concat!("Headroom/", env!("CARGO_PKG_VERSION"));
const JSON: &str = "application/json";
const TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_REDIRECTS: usize = 3;
const BODY_LIMIT: usize = 512 * 1024;

pub struct StatusPageClient {
    client: Client,
}

impl StatusPageClient {
    pub fn new() -> Result<StatusPageClient> {
        StatusPageClient::build(true)
    }

    fn build(https_only: bool) -> Result<StatusPageClient> {
        let client = Client::builder()
            .user_agent(AGENT)
            .timeout(TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .https_only(https_only)
            .redirect(Policy::limited(MAX_REDIRECTS))
            .build()
            .map_err(|error| anyhow::Error::msg(error.without_url().to_string()))
            .context("cannot create the HTTP client for status pages")?;
        Ok(StatusPageClient { client })
    }
}

#[async_trait]
impl StatusFetch for StatusPageClient {
    async fn get(&self, url: &str, etag: Option<&str>) -> Result<Fetched, FetchError> {
        let request = self.client.get(url).header(ACCEPT, JSON);
        let request = match etag {
            Some(etag) => request.header(IF_NONE_MATCH, etag),
            None => request,
        };
        let response = request.send().await.map_err(failed)?;
        interpret(response).await
    }
}

async fn interpret(response: Response) -> Result<Fetched, FetchError> {
    match response.status() {
        StatusCode::NOT_MODIFIED => Ok(Fetched::NotModified),
        StatusCode::TOO_MANY_REQUESTS => Err(FetchError::RateLimited {
            retry_after: headroom_providers::http::retry_after(
                response.headers(),
                Timestamp::now(),
            ),
        }),
        status if status.is_success() => {
            let etag = response
                .headers()
                .get(ETAG)
                .and_then(|value| value.to_str().ok())
                .map(|value| value.trim().to_owned());
            let body = read_capped(response).await?;
            Ok(Fetched::Modified { body, etag })
        }
        status => Err(FetchError::Failed(format!(
            "the status page answered {status}"
        ))),
    }
}

async fn read_capped(mut response: Response) -> Result<String, FetchError> {
    let too_large = || FetchError::Failed(format!("the answer is larger than {BODY_LIMIT} bytes"));
    let declared = response.content_length().unwrap_or(0);
    if usize::try_from(declared).map_or(true, |declared| declared > BODY_LIMIT) {
        return Err(too_large());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(failed)? {
        if body.len() + chunk.len() > BODY_LIMIT {
            return Err(too_large());
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|_| {
        FetchError::Failed("the status page answered with text that is not UTF-8".into())
    })
}

fn failed(error: reqwest::Error) -> FetchError {
    FetchError::Failed(error.without_url().to_string())
}

#[cfg(test)]
#[path = "status_pages_tests.rs"]
mod tests;
