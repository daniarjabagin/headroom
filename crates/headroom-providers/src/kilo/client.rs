use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use reqwest::header::ACCEPT;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::money::Usd;
use crate::http;

const BALANCE_PATH: &str = "/api/profile/balance";
const PROFILE_PATH: &str = "/api/profile";
const ORGANIZATION_HEADER: &str = "x-kilocode-organizationid";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawBalance {
    pub(super) balance: Usd,
    pub(super) is_depleted: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawProfile {
    pub(super) user: RawUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawUser {
    pub(super) email: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct KiloClient {
    http: Client,
    base: String,
}

impl KiloClient {
    pub(super) fn new(http: Client, api_base: &str) -> KiloClient {
        KiloClient {
            http,
            base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) async fn balance(
        &self,
        token: &str,
        organization: Option<&str>,
        now: Timestamp,
    ) -> Result<RawBalance, ProviderError> {
        let mut request = self.get(BALANCE_PATH, token);
        if let Some(organization) = organization {
            request = request.header(ORGANIZATION_HEADER, organization);
        }
        send(request, "balance", now).await
    }

    pub(super) async fn profile(
        &self,
        token: &str,
        now: Timestamp,
    ) -> Result<RawProfile, ProviderError> {
        send(self.get(PROFILE_PATH, token), "profile", now).await
    }

    fn get(&self, path: &str, token: &str) -> RequestBuilder {
        self.http
            .get(format!("{}{path}", self.base))
            .timeout(TIMEOUT)
            .bearer_auth(token)
            .header(ACCEPT, "application/json")
    }
}

async fn send<T: DeserializeOwned>(
    request: RequestBuilder,
    endpoint: &str,
    now: Timestamp,
) -> Result<T, ProviderError> {
    let response = request.send().await.map_err(transport_error)?;
    let status = response.status();
    if !status.is_success() {
        let retry_after = http::retry_after(response.headers(), now);
        return Err(status_error(status, retry_after, endpoint));
    }
    let body = response.bytes().await.map_err(transport_error)?;
    serde_json::from_slice(&body).map_err(|error| {
        ProviderError::InvalidResponse(format!(
            "cannot parse the Kilo {endpoint} response: {error}"
        ))
    })
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

fn status_error(
    status: StatusCode,
    retry_after: Option<SignedDuration>,
    endpoint: &str,
) -> ProviderError {
    let code = status.as_u16();
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::rate_limited(retry_after),
        status if status.is_server_error() => {
            ProviderError::Network(format!("Kilo {endpoint} endpoint returned HTTP {code}"))
        }
        _ => {
            ProviderError::InvalidResponse(format!("Kilo {endpoint} endpoint returned HTTP {code}"))
        }
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
