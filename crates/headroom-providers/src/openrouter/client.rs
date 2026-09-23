use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use reqwest::header::ACCEPT;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;

use super::raw::{Envelope, RawCredits, RawKey};
use crate::http;

const KEY_PATH: &str = "/api/v1/key";
const CREDITS_PATH: &str = "/api/v1/credits";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub(super) struct KeyClient {
    http: Client,
    base: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Credits {
    Available(RawCredits),
    NeedsManagementKey,
    Unavailable(ProviderError),
}

#[derive(Debug)]
enum FetchError {
    Transport(ProviderError),
    Status {
        status: StatusCode,
        retry_after: Option<SignedDuration>,
    },
}

impl KeyClient {
    pub(super) fn new(http: Client, api_base: &str) -> KeyClient {
        KeyClient {
            http,
            base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) async fn key(&self, key: &str, now: Timestamp) -> Result<RawKey, ProviderError> {
        let body = self
            .get(KEY_PATH, key, now)
            .await
            .map_err(|error| to_provider_error(error, "key"))?;
        parse_data(&body, "key")
    }

    pub(super) async fn credits(&self, key: &str, now: Timestamp) -> Credits {
        match self.get(CREDITS_PATH, key, now).await {
            Ok(body) => match parse_data(&body, "credits") {
                Ok(credits) => Credits::Available(credits),
                Err(error) => Credits::Unavailable(error),
            },
            Err(FetchError::Status {
                status: StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN,
                ..
            }) => Credits::NeedsManagementKey,
            Err(error) => Credits::Unavailable(to_provider_error(error, "credits")),
        }
    }

    async fn get(&self, path: &str, key: &str, now: Timestamp) -> Result<Vec<u8>, FetchError> {
        let response = self
            .http
            .get(format!("{}{path}", self.base))
            .timeout(TIMEOUT)
            .bearer_auth(key)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| FetchError::Transport(transport_error(error)))?;
        let status = response.status();
        if !status.is_success() {
            let retry_after = http::retry_after(response.headers(), now);
            return Err(FetchError::Status {
                status,
                retry_after,
            });
        }
        let body = response
            .bytes()
            .await
            .map_err(|error| FetchError::Transport(transport_error(error)))?;
        Ok(body.to_vec())
    }
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

fn to_provider_error(error: FetchError, endpoint: &str) -> ProviderError {
    match error {
        FetchError::Transport(error) => error,
        FetchError::Status {
            status,
            retry_after,
        } => status_error(status, retry_after, endpoint),
    }
}

fn status_error(
    status: StatusCode,
    retry_after: Option<SignedDuration>,
    endpoint: &str,
) -> ProviderError {
    let code = status.as_u16();
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited { retry_after },
        status if status.is_server_error() => ProviderError::Network(format!(
            "OpenRouter {endpoint} endpoint returned HTTP {code}"
        )),
        _ => ProviderError::InvalidResponse(format!(
            "OpenRouter {endpoint} endpoint returned HTTP {code}"
        )),
    }
}

fn parse_data<T: DeserializeOwned>(body: &[u8], endpoint: &str) -> Result<T, ProviderError> {
    serde_json::from_slice::<Envelope<T>>(body)
        .map(|envelope| envelope.data)
        .map_err(|error| {
            ProviderError::InvalidResponse(format!(
                "cannot parse the OpenRouter {endpoint} response: {error}"
            ))
        })
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
