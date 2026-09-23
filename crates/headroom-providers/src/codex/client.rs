use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use reqwest::StatusCode;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, USER_AGENT};
use serde::Deserialize;

use super::auth::Credentials;
use super::number::FlexNumber;
use super::plan::no_subscription;
use crate::http;
use crate::plan_error::mentions_plan;

pub const DEFAULT_API_BASE: &str = "https://chatgpt.com";
const USAGE_PATH: &str = "/backend-api/wham/usage";
const ACCOUNT_HEADER: &str = "ChatGPT-Account-Id";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

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
    pub(super) fn new(http: reqwest::Client, api_base: &str) -> UsageClient {
        UsageClient {
            http,
            api_base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) async fn fetch_usage(
        &self,
        credentials: &Credentials,
        now: Timestamp,
    ) -> Result<UsageResponse, ProviderError> {
        let response = self
            .http
            .get(format!("{}{USAGE_PATH}", self.api_base))
            .timeout(REQUEST_TIMEOUT)
            .headers(request_headers(credentials)?)
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

fn status_error(
    status: StatusCode,
    headers: &HeaderMap,
    body: &[u8],
    now: Timestamp,
) -> ProviderError {
    match status {
        StatusCode::PAYMENT_REQUIRED => no_subscription(None),
        StatusCode::FORBIDDEN if mentions_plan(body) => no_subscription(None),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited {
            retry_after: http::retry_after(headers, now),
        },
        _ => ProviderError::Network(format!("usage request returned HTTP {status}")),
    }
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
