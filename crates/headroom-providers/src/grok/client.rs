use std::time::Duration;

use headroom_core::provider::ProviderError;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{RequestBuilder, StatusCode};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Number;

use super::mapper::no_subscription;
use crate::http;
use crate::plan_error::mentions_plan;

const BILLING_PATH: &str = "/billing?format=credits";
const SETTINGS_PATH: &str = "/settings";
const TOKEN_PATH: &str = "/oauth2/token";
const TOKEN_AUTH_HEADER: &str = "X-XAI-Token-Auth";
const TOKEN_AUTH_VALUE: &str = "xai-grok-cli";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct RawBilling {
    pub config: RawCreditsConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawCreditsConfig {
    pub credit_usage_percent: Option<f64>,
    pub current_period: Option<RawPeriod>,
    pub on_demand_cap: Option<RawValue>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct RawPeriod {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct RawValue {
    pub val: Option<Number>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub(super) struct RawSettings {
    pub subscription_tier_display: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub(super) struct RawTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_in: Option<i64>,
}

#[derive(Debug, Clone)]
pub(super) struct GrokClient {
    http: reqwest::Client,
    api_base: String,
}

impl GrokClient {
    pub(super) fn new(http: reqwest::Client, api_base: &str) -> GrokClient {
        GrokClient {
            http,
            api_base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) async fn billing(&self, token: &str) -> Result<RawBilling, ProviderError> {
        self.get(BILLING_PATH, token, "billing").await
    }

    pub(super) async fn settings(&self, token: &str) -> Result<RawSettings, ProviderError> {
        self.get(SETTINGS_PATH, token, "settings").await
    }

    pub(super) async fn refresh(
        &self,
        issuer: &str,
        client_id: &str,
        refresh_token: &str,
    ) -> Result<RawTokens, ProviderError> {
        let body = form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh_token),
        ]);
        let request = self
            .http
            .post(format!("{}{TOKEN_PATH}", issuer.trim_end_matches('/')))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(ACCEPT, "application/json")
            .body(body);
        send(request, "token refresh", refresh_error).await
    }

    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        token: &str,
        what: &'static str,
    ) -> Result<T, ProviderError> {
        let request = self
            .http
            .get(format!("{}{path}", self.api_base))
            .headers(auth_headers(token)?);
        send(request, what, status_error).await
    }
}

async fn send<T: DeserializeOwned>(
    request: RequestBuilder,
    what: &'static str,
    on_status: fn(StatusCode, &HeaderMap, &[u8], &str) -> ProviderError,
) -> Result<T, ProviderError> {
    let response = request
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(transport_error)?;
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.map_err(transport_error)?;
    if !status.is_success() {
        return Err(on_status(status, &headers, &body, what));
    }
    serde_json::from_slice(&body)
        .map_err(|error| ProviderError::InvalidResponse(format!("{what} body: {error}")))
}

fn auth_headers(token: &str) -> Result<HeaderMap, ProviderError> {
    let bearer = HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| ProviderError::LocalData("auth.json contains an unusable token".into()))?;
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, bearer);
    headers.insert(
        TOKEN_AUTH_HEADER,
        HeaderValue::from_static(TOKEN_AUTH_VALUE),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    Ok(headers)
}

fn status_error(status: StatusCode, headers: &HeaderMap, body: &[u8], what: &str) -> ProviderError {
    match status {
        StatusCode::PAYMENT_REQUIRED => no_subscription(),
        StatusCode::FORBIDDEN if mentions_plan(body) => no_subscription(),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => {
            ProviderError::rate_limited(http::retry_after_seconds(headers))
        }
        _ => ProviderError::Network(format!("{what} request returned HTTP {status}")),
    }
}

fn refresh_error(
    status: StatusCode,
    headers: &HeaderMap,
    body: &[u8],
    what: &str,
) -> ProviderError {
    match status {
        StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            ProviderError::SignInExpired
        }
        _ => status_error(status, headers, body, what),
    }
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

fn form(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(name, value)| format!("{}={}", encode(name), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
