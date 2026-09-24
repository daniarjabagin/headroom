use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap};
use reqwest::{Client, Response, StatusCode};
use serde::de::DeserializeOwned;

use super::credentials::RefreshedTokens;
use super::raw::RawUsages;
use crate::http;

const USAGES_PATH: &str = "/usages";
const TOKEN_PATH: &str = "/api/oauth/token";
const CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";
const TIMEOUT: Duration = Duration::from_secs(12);
const FORM_TYPE: &str = "application/x-www-form-urlencoded";
pub(super) const NO_PLAN: &str = "this Kimi account has no Kimi Code plan";

#[derive(Debug, Clone)]
pub(super) struct KimiClient {
    http: Client,
    usages_url: String,
    token_url: String,
}

impl KimiClient {
    pub(super) fn new(http: Client, api_base: &str, oauth_host: &str) -> KimiClient {
        KimiClient {
            http,
            usages_url: format!("{}{USAGES_PATH}", api_base.trim_end_matches('/')),
            token_url: format!("{}{TOKEN_PATH}", oauth_host.trim_end_matches('/')),
        }
    }

    pub(super) async fn usages(
        &self,
        bearer: &str,
        now: Timestamp,
    ) -> Result<RawUsages, ProviderError> {
        let request = self
            .http
            .get(&self.usages_url)
            .timeout(TIMEOUT)
            .bearer_auth(bearer)
            .header(ACCEPT, "application/json");
        let response = request.send().await.map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(usages_error(status, response.headers(), now));
        }
        read_json(response, "usage").await
    }

    pub(super) async fn refresh(
        &self,
        refresh_token: &str,
        now: Timestamp,
    ) -> Result<RefreshedTokens, ProviderError> {
        let body = form(&[
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ]);
        let request = self
            .http
            .post(&self.token_url)
            .timeout(TIMEOUT)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, FORM_TYPE)
            .body(body);
        let response = request.send().await.map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(refresh_error(status, response.headers(), now));
        }
        read_json(response, "token").await
    }
}

async fn read_json<T: DeserializeOwned>(
    response: Response,
    what: &str,
) -> Result<T, ProviderError> {
    let body = response.bytes().await.map_err(transport_error)?;
    serde_json::from_slice(&body).map_err(|error| {
        ProviderError::InvalidResponse(format!(
            "cannot parse the Kimi {what} response at line {} column {}",
            error.line(),
            error.column()
        ))
    })
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

fn usages_error(status: StatusCode, headers: &HeaderMap, now: Timestamp) -> ProviderError {
    match status {
        StatusCode::UNAUTHORIZED => ProviderError::SignInExpired,
        StatusCode::PAYMENT_REQUIRED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND => {
            ProviderError::NoSubscription {
                detail: NO_PLAN.to_owned(),
            }
        }
        _ => common_error(status, headers, now, "usage"),
    }
}

fn refresh_error(status: StatusCode, headers: &HeaderMap, now: Timestamp) -> ProviderError {
    match status {
        StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            ProviderError::SignInExpired
        }
        _ => common_error(status, headers, now, "token"),
    }
}

fn common_error(
    status: StatusCode,
    headers: &HeaderMap,
    now: Timestamp,
    what: &str,
) -> ProviderError {
    let code = status.as_u16();
    match status {
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited {
            retry_after: http::retry_after(headers, now),
        },
        status if status.is_server_error() => {
            ProviderError::Network(format!("the Kimi {what} endpoint returned HTTP {code}"))
        }
        _ => {
            ProviderError::InvalidResponse(format!("the Kimi {what} endpoint returned HTTP {code}"))
        }
    }
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
