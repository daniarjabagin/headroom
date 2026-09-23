use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use reqwest::header::{ACCEPT, HeaderMap, RETRY_AFTER};
use reqwest::{Client, StatusCode};
use serde::Deserialize;

const USAGE_PATH: &str = "/zen/go/v1/usage";
const TIMEOUT: Duration = Duration::from_secs(15);
const ENTITLEMENT_ERROR: &str = "EntitlementError";
const NO_SUBSCRIPTION: &str = "No OpenCode Go subscription on this key.";

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RawUsage {
    pub(super) usage: RawWindows,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RawWindows {
    pub(super) rolling: RawWindow,
    pub(super) weekly: RawWindow,
    pub(super) monthly: RawWindow,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(super) struct RawWindow {
    pub(super) percent: f64,
    #[serde(rename = "resetsAt")]
    pub(super) resets_at: Option<Timestamp>,
}

#[derive(Debug, Deserialize)]
struct RawErrorBody {
    error: RawError,
}

#[derive(Debug, Deserialize)]
struct RawError {
    #[serde(rename = "type")]
    kind: String,
}

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

    pub(super) async fn fetch(&self, key: &str, now: Timestamp) -> Result<RawUsage, ProviderError> {
        let response = self
            .http
            .get(&self.url)
            .timeout(TIMEOUT)
            .bearer_auth(key)
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
            "cannot parse OpenCode Go usage at line {} column {}",
            error.line(),
            error.column()
        ))
    })
}

pub(super) fn status_error(
    status: StatusCode,
    headers: &HeaderMap,
    body: &[u8],
    now: Timestamp,
) -> ProviderError {
    match status {
        StatusCode::FORBIDDEN if error_kind(body).as_deref() == Some(ENTITLEMENT_ERROR) => {
            ProviderError::NoSubscription {
                detail: NO_SUBSCRIPTION.to_owned(),
            }
        }
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited {
            retry_after: headers
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| retry_after(value, now)),
        },
        status if status.is_server_error() => ProviderError::Network(format!(
            "OpenCode usage endpoint returned HTTP {}",
            status.as_u16()
        )),
        status => ProviderError::InvalidResponse(format!(
            "OpenCode usage endpoint returned HTTP {}",
            status.as_u16()
        )),
    }
}

fn error_kind(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<RawErrorBody>(body)
        .ok()
        .map(|body| body.error.kind)
}

fn retry_after(value: &str, now: Timestamp) -> Option<SignedDuration> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<u32>() {
        return Some(SignedDuration::from_secs(i64::from(seconds)));
    }
    let at = jiff::fmt::rfc2822::parse(value).ok()?.timestamp();
    let wait = at.duration_since(now).max(SignedDuration::ZERO);
    let partial = i64::from(wait.subsec_nanos() > 0);
    Some(SignedDuration::from_secs(wait.as_secs() + partial))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
