use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, RETRY_AFTER, USER_AGENT};
use reqwest::{Response, StatusCode};
use serde::Deserialize;

use super::auth::Credentials;
use super::number::FlexNumber;

pub const DEFAULT_API_BASE: &str = "https://chatgpt.com";
const USAGE_PATH: &str = "/backend-api/wham/usage";
const ACCOUNT_HEADER: &str = "ChatGPT-Account-Id";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct UsageResponse {
    pub plan_type: Option<String>,
    pub rate_limit: Option<RawRateLimit>,
    pub additional_rate_limits: Option<Vec<RawAdditionalLimit>>,
    pub credits: Option<RawCredits>,
    pub rate_limit_reset_credits: Option<RawResetCredits>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct RawRateLimit {
    pub primary_window: Option<RawWindow>,
    pub secondary_window: Option<RawWindow>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct RawWindow {
    pub used_percent: Option<FlexNumber>,
    pub limit_window_seconds: Option<FlexNumber>,
    pub reset_after_seconds: Option<FlexNumber>,
    pub reset_at: Option<FlexNumber>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct RawAdditionalLimit {
    pub limit_name: Option<String>,
    pub metered_feature: Option<String>,
    pub rate_limit: Option<RawRateLimit>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct RawCredits {
    pub has_credits: Option<bool>,
    pub unlimited: Option<bool>,
    pub balance: Option<FlexNumber>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct RawResetCredits {
    pub available_count: Option<FlexNumber>,
}

#[derive(Debug, Clone)]
pub(super) struct UsageClient {
    http: reqwest::Client,
    api_base: String,
}

impl UsageClient {
    pub(super) fn new(api_base: &str) -> Result<UsageClient, ProviderError> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .build()
            .map_err(|error| ProviderError::Network(error.without_url().to_string()))?;
        Ok(UsageClient {
            http,
            api_base: api_base.trim_end_matches('/').to_owned(),
        })
    }

    pub(super) async fn fetch_usage(
        &self,
        credentials: &Credentials,
        now: Timestamp,
    ) -> Result<UsageResponse, ProviderError> {
        let response = self
            .http
            .get(format!("{}{USAGE_PATH}", self.api_base))
            .headers(request_headers(credentials)?)
            .send()
            .await
            .map_err(transport_error)?;
        check_status(&response, now)?;
        let body = response.bytes().await.map_err(transport_error)?;
        serde_json::from_slice(&body)
            .map_err(|error| ProviderError::InvalidResponse(format!("usage body: {error}")))
    }
}

fn request_headers(credentials: &Credentials) -> Result<HeaderMap, ProviderError> {
    let invalid = |_| ProviderError::LocalData("auth.json contains an unusable token".into());
    let mut headers = HeaderMap::new();
    let bearer = format!("Bearer {}", credentials.access_token);
    headers.insert(AUTHORIZATION, bearer.parse().map_err(invalid)?);
    headers.insert(ACCEPT, "application/json".parse().map_err(invalid)?);
    let agent = format!("headroom/{}", env!("CARGO_PKG_VERSION"));
    headers.insert(USER_AGENT, agent.parse().map_err(invalid)?);
    if let Some(account_id) = &credentials.account_id {
        headers.insert(ACCOUNT_HEADER, account_id.parse().map_err(invalid)?);
    }
    Ok(headers)
}

fn check_status(response: &Response, now: Timestamp) -> Result<(), ProviderError> {
    let status = response.status();
    match status {
        _ if status.is_success() => Ok(()),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(ProviderError::SignInExpired),
        StatusCode::TOO_MANY_REQUESTS => Err(ProviderError::RateLimited {
            retry_after: response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| parse_retry_after(value, now)),
        }),
        _ => Err(ProviderError::Network(format!(
            "usage request returned HTTP {status}"
        ))),
    }
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

pub(super) fn parse_retry_after(value: &str, now: Timestamp) -> Option<SignedDuration> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<i64>() {
        return Some(SignedDuration::from_secs(seconds.max(0)));
    }
    let at = jiff::fmt::rfc2822::parse(value).ok()?.timestamp();
    Some(at.duration_since(now).max(SignedDuration::ZERO))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
