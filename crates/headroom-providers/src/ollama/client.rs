use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use reqwest::header::{ACCEPT, AUTHORIZATION, RETRY_AFTER};
use reqwest::{Client, Method, Response, StatusCode};
use serde::de::DeserializeOwned;

use super::key::SigningKey;
use super::raw::{RawMe, RawUsage};

pub const DEFAULT_API_BASE: &str = "https://ollama.com";
const USAGE_PATH: &str = "/api/usage";
const ME_PATH: &str = "/api/me";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub(super) struct CloudClient {
    http: Client,
    base: String,
}

impl CloudClient {
    pub(super) fn new(http: Client, base: &str) -> CloudClient {
        CloudClient {
            http,
            base: base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) async fn usage(
        &self,
        key: &SigningKey,
        now: Timestamp,
    ) -> Result<RawUsage, ProviderError> {
        self.signed(Method::GET, USAGE_PATH, key, now).await
    }

    pub(super) async fn me(
        &self,
        key: &SigningKey,
        now: Timestamp,
    ) -> Result<RawMe, ProviderError> {
        self.signed(Method::POST, ME_PATH, key, now).await
    }

    async fn signed<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        key: &SigningKey,
        now: Timestamp,
    ) -> Result<T, ProviderError> {
        let request_uri = signed_uri(path, now);
        let authorization = key.authorization(&challenge(&method, &request_uri));
        let response = self
            .http
            .request(method, format!("{}{request_uri}", self.base))
            .timeout(TIMEOUT)
            .header(AUTHORIZATION, authorization)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(transport_error)?;
        parse(response, path).await
    }
}

pub(super) fn signed_uri(path: &str, now: Timestamp) -> String {
    format!("{path}?ts={}", now.as_second())
}

pub(super) fn challenge(method: &Method, request_uri: &str) -> String {
    format!("{method},{request_uri}")
}

async fn parse<T: DeserializeOwned>(response: Response, path: &str) -> Result<T, ProviderError> {
    let status = response.status();
    if !status.is_success() {
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(retry_after_seconds);
        return Err(status_error(status, retry_after, path));
    }
    let body = response.bytes().await.map_err(transport_error)?;
    serde_json::from_slice(&body).map_err(|error| {
        ProviderError::InvalidResponse(format!(
            "cannot parse the Ollama {path} response at line {} column {}",
            error.line(),
            error.column()
        ))
    })
}

pub(super) fn status_error(
    status: StatusCode,
    retry_after: Option<SignedDuration>,
    path: &str,
) -> ProviderError {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited { retry_after },
        status if status.is_server_error() => {
            ProviderError::Network(format!("Ollama {path} returned HTTP {}", status.as_u16()))
        }
        status => ProviderError::InvalidResponse(format!(
            "Ollama {path} returned HTTP {}",
            status.as_u16()
        )),
    }
}

fn retry_after_seconds(value: &str) -> Option<SignedDuration> {
    value
        .trim()
        .parse::<u32>()
        .ok()
        .map(|seconds| SignedDuration::from_secs(i64::from(seconds)))
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_challenge_signs_method_path_and_timestamp() {
        let now: Timestamp = "2026-09-23T10:00:00Z".parse().unwrap();
        let uri = signed_uri("/api/usage", now);
        assert_eq!(uri, "/api/usage?ts=1790157600");
        assert_eq!(
            challenge(&Method::GET, &uri),
            "GET,/api/usage?ts=1790157600"
        );
        assert_eq!(
            challenge(&Method::POST, &signed_uri("/api/me", now)),
            "POST,/api/me?ts=1790157600"
        );
    }

    #[test]
    fn statuses_map_to_provider_errors() {
        let wait = Some(SignedDuration::from_secs(30));
        assert_eq!(
            status_error(StatusCode::UNAUTHORIZED, None, "/api/usage"),
            ProviderError::SignInExpired
        );
        assert_eq!(
            status_error(StatusCode::FORBIDDEN, None, "/api/usage"),
            ProviderError::SignInExpired
        );
        assert_eq!(
            status_error(StatusCode::TOO_MANY_REQUESTS, wait, "/api/usage"),
            ProviderError::RateLimited { retry_after: wait }
        );
        assert!(matches!(
            status_error(StatusCode::BAD_GATEWAY, None, "/api/usage"),
            ProviderError::Network(_)
        ));
        assert!(matches!(
            status_error(StatusCode::NOT_FOUND, None, "/api/usage"),
            ProviderError::InvalidResponse(_)
        ));
    }

    #[test]
    fn retry_after_reads_whole_seconds() {
        assert_eq!(
            retry_after_seconds(" 120 "),
            Some(SignedDuration::from_secs(120))
        );
        assert_eq!(retry_after_seconds("soon"), None);
    }
}
