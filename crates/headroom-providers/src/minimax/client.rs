use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap};
use reqwest::{Client, StatusCode};

use super::mapper::check_status;
use super::raw::RawRemains;
use crate::http;

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
        StatusCode::TOO_MANY_REQUESTS => {
            ProviderError::rate_limited(http::retry_after(headers, now))
        }
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

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
