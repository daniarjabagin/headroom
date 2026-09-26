use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use reqwest::header::{ACCEPT, HeaderMap};
use reqwest::{Client, StatusCode};

use super::auth::AccessToken;
use super::raw::RawUsage;
use super::subscription::no_subscription;
use crate::http;
use crate::plan_error::mentions_plan;

const USAGE_PATH: &str = "/api/oauth/usage";
const BETA_HEADER: &str = "anthropic-beta";
const BETA_VALUE: &str = "oauth-2025-04-20";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub(super) struct UsageClient {
    http: Client,
    url: String,
}

impl UsageClient {
    pub(super) fn new(http: Client, api_base: &str) -> UsageClient {
        UsageClient {
            http,
            url: format!("{}{USAGE_PATH}", api_base.trim_end_matches('/')),
        }
    }

    pub(super) async fn fetch(
        &self,
        token: &AccessToken,
        now: Timestamp,
    ) -> Result<RawUsage, ProviderError> {
        let response = self
            .http
            .get(&self.url)
            .timeout(TIMEOUT)
            .bearer_auth(token.secret())
            .header(BETA_HEADER, BETA_VALUE)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            let body = response.bytes().await.unwrap_or_default();
            return Err(status_error(status, &headers, &body, now));
        }
        let body = response.bytes().await.map_err(transport_error)?;
        parse_usage(&body)
    }
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

pub(super) fn parse_usage(body: &[u8]) -> Result<RawUsage, ProviderError> {
    serde_json::from_slice(body).map_err(|error| {
        ProviderError::InvalidResponse(format!(
            "cannot parse usage response at line {} column {}",
            error.line(),
            error.column()
        ))
    })
}

fn status_error(
    status: StatusCode,
    headers: &HeaderMap,
    body: &[u8],
    now: Timestamp,
) -> ProviderError {
    match status {
        StatusCode::PAYMENT_REQUIRED => no_subscription(None),
        StatusCode::FORBIDDEN | StatusCode::NOT_FOUND if mentions_plan(body) => {
            no_subscription(None)
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => {
            ProviderError::rate_limited(http::retry_after(headers, now))
        }
        status if status.is_server_error() => {
            ProviderError::Network(format!("usage endpoint returned HTTP {}", status.as_u16()))
        }
        status => ProviderError::InvalidResponse(format!(
            "usage endpoint returned HTTP {}",
            status.as_u16()
        )),
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
