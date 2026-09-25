use std::fmt;
use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use reqwest::StatusCode;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap};
use serde::{Deserialize, Serialize};

use crate::http;

pub const DEFAULT_AUTH_BASE: &str = "https://auth.openai.com";
const TOKEN_PATH: &str = "/oauth/token";
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Serialize)]
struct RefreshRequest<'a> {
    client_id: &'a str,
    grant_type: &'a str,
    refresh_token: &'a str,
}

#[derive(Clone, Default, PartialEq, Eq, Deserialize)]
pub(super) struct RawTokens {
    #[serde(rename = "id_token")]
    pub id: Option<String>,
    #[serde(rename = "access_token")]
    pub access: Option<String>,
    #[serde(rename = "refresh_token")]
    pub refresh: Option<String>,
}

impl fmt::Debug for RawTokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RawTokens(<redacted>)")
    }
}

#[derive(Debug, Clone)]
pub(super) struct TokenClient {
    http: reqwest::Client,
    auth_base: String,
}

impl TokenClient {
    pub(super) fn new(http: reqwest::Client, auth_base: &str) -> TokenClient {
        TokenClient {
            http,
            auth_base: auth_base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) async fn refresh(
        &self,
        refresh_token: &str,
        now: Timestamp,
    ) -> Result<RawTokens, ProviderError> {
        let body = RefreshRequest {
            client_id: CLIENT_ID,
            grant_type: "refresh_token",
            refresh_token,
        };
        let response = self
            .http
            .post(format!("{}{TOKEN_PATH}", self.auth_base))
            .timeout(REQUEST_TIMEOUT)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .json(&body)
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
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited {
            retry_after: http::retry_after(headers, now),
        },
        _ => ProviderError::Network(format!("token refresh returned HTTP {status}")),
    }
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}
