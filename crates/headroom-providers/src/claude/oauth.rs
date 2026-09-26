use std::fmt;
use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use reqwest::StatusCode;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap};
use serde::{Deserialize, Serialize};

use crate::http;

pub const DEFAULT_TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
pub(super) const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Serialize)]
pub(super) struct RefreshRequest<'a> {
    pub grant_type: &'a str,
    pub refresh_token: &'a str,
    pub client_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Clone, Default, PartialEq, Eq, Deserialize)]
pub(super) struct RawTokens {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub scope: Option<String>,
}

impl fmt::Debug for RawTokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawTokens")
            .field("expires_in", &self.expires_in)
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub(super) struct TokenClient {
    http: reqwest::Client,
    url: String,
}

impl TokenClient {
    pub(super) fn new(http: reqwest::Client, url: &str) -> TokenClient {
        TokenClient {
            http,
            url: url.to_owned(),
        }
    }

    pub(super) async fn refresh(
        &self,
        request: &RefreshRequest<'_>,
        now: Timestamp,
    ) -> Result<RawTokens, ProviderError> {
        let response = self
            .http
            .post(&self.url)
            .timeout(REQUEST_TIMEOUT)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .json(request)
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(refresh_error(status, response.headers(), now));
        }
        let bytes = response.bytes().await.map_err(transport_error)?;
        serde_json::from_slice(&bytes)
            .map_err(|error| ProviderError::InvalidResponse(format!("token refresh: {error}")))
    }
}

fn refresh_error(status: StatusCode, headers: &HeaderMap, now: Timestamp) -> ProviderError {
    match status {
        StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            ProviderError::SignInExpired
        }
        StatusCode::TOO_MANY_REQUESTS => {
            ProviderError::token_refresh_rate_limited(http::retry_after(headers, now))
        }
        _ => ProviderError::Network(format!("token refresh returned HTTP {status}")),
    }
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

#[cfg(test)]
mod tests {
    use jiff::SignedDuration;
    use reqwest::header::{HeaderValue, RETRY_AFTER};

    use super::*;

    #[test]
    fn a_limited_token_refresh_says_so() {
        let now: Timestamp = "2026-09-23T10:00:00Z".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("90"));
        let error = refresh_error(StatusCode::TOO_MANY_REQUESTS, &headers, now);
        assert_eq!(
            error,
            ProviderError::token_refresh_rate_limited(Some(SignedDuration::from_secs(90)))
        );
        assert_eq!(
            error.to_string(),
            "token refresh rate limited by the provider"
        );
    }
}
