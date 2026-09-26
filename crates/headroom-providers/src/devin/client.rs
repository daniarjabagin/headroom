use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode};
use serde_json::json;

use super::auth::DevinKey;
use super::mapper::no_subscription;
use super::raw::RawStatusResponse;
use crate::http;
use crate::plan_error::mentions_plan;

const STATUS_PATH: &str = "/exa.seat_management_pb.SeatManagementService/GetUserStatus";
const CLIENT_NAME: &str = "devin";
const COMPAT_VERSION: &str = "1.108.2";
const CONNECT_HEADER: &str = "Connect-Protocol-Version";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub(super) struct StatusClient {
    http: Client,
}

impl StatusClient {
    pub(super) fn new(http: Client) -> StatusClient {
        StatusClient { http }
    }

    pub(super) async fn fetch(
        &self,
        api_base: &str,
        key: &DevinKey,
        now: Timestamp,
    ) -> Result<RawStatusResponse, ProviderError> {
        let url = format!("{}{STATUS_PATH}", api_base.trim_end_matches('/'));
        let response = self
            .http
            .post(url)
            .timeout(TIMEOUT)
            .header(CONNECT_HEADER, "1")
            .json(&request_body(key))
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            let body = response.bytes().await.unwrap_or_default();
            return Err(status_error(status, &headers, &body, now));
        }
        let body = response.bytes().await.map_err(transport_error)?;
        parse_status(&body)
    }
}

fn request_body(key: &DevinKey) -> serde_json::Value {
    json!({
        "metadata": {
            "apiKey": key.api_key.expose(),
            "ideName": CLIENT_NAME,
            "ideVersion": COMPAT_VERSION,
            "extensionName": CLIENT_NAME,
            "extensionVersion": COMPAT_VERSION,
            "locale": "en"
        }
    })
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

pub(super) fn parse_status(body: &[u8]) -> Result<RawStatusResponse, ProviderError> {
    serde_json::from_slice(body).map_err(|error| {
        ProviderError::InvalidResponse(format!(
            "cannot parse Devin user status at line {} column {}",
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
    match status {
        StatusCode::PAYMENT_REQUIRED => no_subscription(),
        StatusCode::FORBIDDEN if mentions_plan(body) => no_subscription(),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => {
            ProviderError::rate_limited(http::retry_after(headers, now))
        }
        status if status.is_server_error() => ProviderError::Network(format!(
            "Devin status endpoint returned HTTP {}",
            status.as_u16()
        )),
        status => ProviderError::InvalidResponse(format!(
            "Devin status endpoint returned HTTP {}",
            status.as_u16()
        )),
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
