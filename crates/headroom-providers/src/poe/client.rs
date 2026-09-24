use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use reqwest::header::ACCEPT;
use reqwest::{Client, StatusCode};
use serde::Deserialize;

use crate::http;

const BALANCE_PATH: &str = "/usage/current_balance";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawBalance {
    pub(super) current_point_balance: u64,
}

#[derive(Debug, Clone)]
pub(super) struct PoeClient {
    http: Client,
    base: String,
}

impl PoeClient {
    pub(super) fn new(http: Client, api_base: &str) -> PoeClient {
        PoeClient {
            http,
            base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) async fn balance(
        &self,
        key: &str,
        now: Timestamp,
    ) -> Result<RawBalance, ProviderError> {
        let response = self
            .http
            .get(format!("{}{BALANCE_PATH}", self.base))
            .timeout(TIMEOUT)
            .bearer_auth(key)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            let retry_after = http::retry_after(response.headers(), now);
            return Err(status_error(status, retry_after));
        }
        let body = response.bytes().await.map_err(transport_error)?;
        serde_json::from_slice(&body).map_err(|error| {
            ProviderError::InvalidResponse(format!(
                "cannot parse the Poe balance response: {error}"
            ))
        })
    }
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

fn status_error(status: StatusCode, retry_after: Option<jiff::SignedDuration>) -> ProviderError {
    let code = status.as_u16();
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited { retry_after },
        status if status.is_server_error() => {
            ProviderError::Network(format!("Poe balance endpoint returned HTTP {code}"))
        }
        _ => ProviderError::InvalidResponse(format!("Poe balance endpoint returned HTTP {code}")),
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
