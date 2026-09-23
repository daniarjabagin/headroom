use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, RETRY_AFTER};
use reqwest::{Client, StatusCode};

use super::mapper::check_status;
use super::raw::RawRemains;

const REMAINS_PATH: &str = "/v1/token_plan/remains";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub(super) struct MiniMaxClient {
    http: Client,
    url: String,
}

impl MiniMaxClient {
    pub(super) fn new(http: Client, api_base: &str) -> MiniMaxClient {
        MiniMaxClient {
            http,
            url: format!("{}{REMAINS_PATH}", api_base.trim_end_matches('/')),
        }
    }

    pub(super) async fn remains(
        &self,
        key: &str,
        now: Timestamp,
    ) -> Result<RawRemains, ProviderError> {
        let request = self
            .http
            .get(&self.url)
            .timeout(TIMEOUT)
            .bearer_auth(key)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json");
        let response = request.send().await.map_err(transport_error)?;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.bytes().await.map_err(transport_error)?;
        if status.is_success() {
            return parse_remains(&body);
        }
        Err(status_error(status, &headers, &body, now))
    }
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

pub(super) fn parse_remains(body: &[u8]) -> Result<RawRemains, ProviderError> {
    serde_json::from_slice(body).map_err(|error| {
        ProviderError::InvalidResponse(format!(
            "cannot parse the MiniMax quota response at line {} column {}",
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
    let code = status.as_u16();
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited {
            retry_after: headers
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| retry_after(value, now)),
        },
        status if status.is_server_error() => {
            ProviderError::Network(format!("the MiniMax quota endpoint returned HTTP {code}"))
        }
        _ => body_error(body).unwrap_or_else(|| {
            ProviderError::InvalidResponse(format!(
                "the MiniMax quota endpoint returned HTTP {code}"
            ))
        }),
    }
}

fn body_error(body: &[u8]) -> Option<ProviderError> {
    let base = parse_remains(body).ok()?.base_resp?;
    check_status(&base).err()
}

fn retry_after(value: &str, now: Timestamp) -> Option<SignedDuration> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<u32>() {
        return Some(SignedDuration::from_secs(i64::from(seconds)));
    }
    let at = jiff::fmt::rfc2822::parse(value).ok()?.timestamp();
    let wait = at.duration_since(now).max(SignedDuration::ZERO);
    Some(SignedDuration::from_secs(
        wait.as_secs() + i64::from(wait.subsec_nanos() > 0),
    ))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
