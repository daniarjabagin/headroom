use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::SignedDuration;
use reqwest::header::{ACCEPT, HeaderMap, RETRY_AFTER};
use reqwest::{Client, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::auth::Secret;
use super::raw::{RawBalance, RawCurrentPlan, RawTokens, RawUser};

const ME_PATH: &str = "/api/v1/users/me";
const PLAN_PATH: &str = "/api/v1/users/me/plan";
const REFRESH_PATH: &str = "/api/v1/auth/refresh";
const TIMEOUT: Duration = Duration::from_secs(15);
const MAX_ERROR_CHARS: usize = 200;

#[derive(Debug, Clone)]
pub(super) struct ClineClient {
    http: Client,
    base: String,
}

impl ClineClient {
    pub(super) fn new(http: Client, api_base: &str) -> ClineClient {
        ClineClient {
            http,
            base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) async fn me(&self, token: &Secret) -> Result<RawUser, ProviderError> {
        self.get(ME_PATH, token).await
    }

    pub(super) async fn plan(
        &self,
        token: &Secret,
    ) -> Result<Option<RawCurrentPlan>, ProviderError> {
        self.get(PLAN_PATH, token).await
    }

    pub(super) async fn balance(
        &self,
        token: &Secret,
        user_id: &str,
    ) -> Result<RawBalance, ProviderError> {
        let path = format!("/api/v1/users/{}/balance", path_segment(user_id));
        self.get(&path, token).await
    }

    pub(super) async fn organization_balance(
        &self,
        token: &Secret,
        organization_id: &str,
    ) -> Result<RawBalance, ProviderError> {
        let path = format!(
            "/api/v1/organizations/{}/balance",
            path_segment(organization_id)
        );
        self.get(&path, token).await
    }

    pub(super) async fn refresh(&self, refresh_token: &Secret) -> Result<RawTokens, ProviderError> {
        let body = json!({ "refreshToken": refresh_token.expose(), "grantType": "refresh_token" });
        let request = self.http.post(self.url(REFRESH_PATH)).json(&body);
        send(request, refresh_status_error).await
    }

    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        token: &Secret,
    ) -> Result<T, ProviderError> {
        let request = self.http.get(self.url(path)).bearer_auth(token.expose());
        send(request, status_error).await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }
}

type StatusError = fn(StatusCode, &HeaderMap) -> ProviderError;

async fn send<T: DeserializeOwned>(
    request: RequestBuilder,
    on_status: StatusError,
) -> Result<T, ProviderError> {
    let response = request
        .timeout(TIMEOUT)
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(transport_error)?;
    let status = response.status();
    if !status.is_success() {
        return Err(on_status(status, response.headers()));
    }
    let body = response.bytes().await.map_err(transport_error)?;
    parse_envelope(&body)
}

fn path_segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                char::from(byte).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

pub(super) fn parse_envelope<T: DeserializeOwned>(body: &[u8]) -> Result<T, ProviderError> {
    let value: Value = serde_json::from_slice(body).map_err(|error| {
        ProviderError::InvalidResponse(format!(
            "cannot parse Cline response at line {} column {}",
            error.line(),
            error.column()
        ))
    })?;
    let data = unwrap_envelope(value)?;
    serde_json::from_value(data).map_err(|error| {
        ProviderError::InvalidResponse(format!("unexpected Cline response: {error}"))
    })
}

fn unwrap_envelope(value: Value) -> Result<Value, ProviderError> {
    let Value::Object(mut fields) = value else {
        return Ok(value);
    };
    match fields.get("success") {
        Some(Value::Bool(true)) => Ok(fields.remove("data").unwrap_or(Value::Null)),
        Some(Value::Bool(false)) => Err(envelope_error(fields.get("error"))),
        _ => Ok(Value::Object(fields)),
    }
}

fn envelope_error(error: Option<&Value>) -> ProviderError {
    let message = error
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or("request failed");
    let short: String = message.chars().take(MAX_ERROR_CHARS).collect();
    ProviderError::InvalidResponse(format!("Cline API error: {short}"))
}

fn refresh_status_error(status: StatusCode, headers: &HeaderMap) -> ProviderError {
    match status {
        StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            ProviderError::SignInExpired
        }
        status => status_error(status, headers),
    }
}

fn status_error(status: StatusCode, headers: &HeaderMap) -> ProviderError {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::PAYMENT_REQUIRED => ProviderError::NoSubscription {
            detail: "No active Cline plan.".to_owned(),
        },
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited {
            retry_after: headers
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.trim().parse::<u32>().ok())
                .map(|seconds| SignedDuration::from_secs(i64::from(seconds))),
        },
        status if status.is_server_error() => {
            ProviderError::Network(format!("Cline API returned HTTP {}", status.as_u16()))
        }
        status => {
            ProviderError::InvalidResponse(format!("Cline API returned HTTP {}", status.as_u16()))
        }
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
