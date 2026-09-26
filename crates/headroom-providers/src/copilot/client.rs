use std::time::Duration;

use headroom_core::provider::ProviderError;
use headroom_core::secret::SecretString;
use jiff::{SignedDuration, Timestamp};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, RETRY_AFTER, USER_AGENT};
use reqwest::{Client, StatusCode};

use super::mapper::no_subscription;
use super::raw::RawUser;
use crate::http;
use crate::plan_error::mentions_plan;

const USER_PATH: &str = "/copilot_internal/user";
const EDITOR_HEADERS: [(&str, &str); 3] = [
    ("Editor-Version", "vscode/1.96.2"),
    ("Editor-Plugin-Version", "copilot-chat/0.26.7"),
    ("X-Github-Api-Version", "2025-04-01"),
];
const EDITOR_USER_AGENT: &str = "GitHubCopilotChat/0.26.7";
const RATE_REMAINING: &str = "x-ratelimit-remaining";
const RATE_RESET: &str = "x-ratelimit-reset";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub(super) struct UserClient {
    http: Client,
    url: String,
}

impl UserClient {
    pub(super) fn new(http: Client, api_base: &str) -> UserClient {
        UserClient {
            http,
            url: format!("{}{USER_PATH}", api_base.trim_end_matches('/')),
        }
    }

    pub(super) async fn fetch(
        &self,
        token: &SecretString,
        now: Timestamp,
    ) -> Result<RawUser, ProviderError> {
        let mut request = self
            .http
            .get(&self.url)
            .timeout(TIMEOUT)
            .header(AUTHORIZATION, format!("token {}", token.expose()))
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, EDITOR_USER_AGENT);
        for (name, value) in EDITOR_HEADERS {
            request = request.header(name, value);
        }
        let response = request.send().await.map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            let body = response.bytes().await.unwrap_or_default();
            return Err(status_error(status, &headers, &body, now));
        }
        let body = response.bytes().await.map_err(transport_error)?;
        parse_user(&body)
    }
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

fn parse_user(body: &[u8]) -> Result<RawUser, ProviderError> {
    serde_json::from_slice(body).map_err(|error| {
        ProviderError::InvalidResponse(format!(
            "cannot parse the Copilot user at line {} column {}",
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
        StatusCode::TOO_MANY_REQUESTS => rate_limited(headers, now),
        StatusCode::FORBIDDEN if is_rate_limited(headers) => rate_limited(headers, now),
        StatusCode::NOT_FOUND => no_subscription(),
        StatusCode::FORBIDDEN if mentions_plan(body) => no_subscription(),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        status if status.is_server_error() => ProviderError::Network(format!(
            "Copilot user endpoint returned HTTP {}",
            status.as_u16()
        )),
        status => ProviderError::InvalidResponse(format!(
            "Copilot user endpoint returned HTTP {}",
            status.as_u16()
        )),
    }
}

fn is_rate_limited(headers: &HeaderMap) -> bool {
    headers.contains_key(RETRY_AFTER) || header_text(headers, RATE_REMAINING) == Some("0")
}

fn rate_limited(headers: &HeaderMap, now: Timestamp) -> ProviderError {
    let retry_after = http::retry_after(headers, now).or_else(|| rate_reset_wait(headers, now));
    ProviderError::rate_limited(retry_after)
}

fn rate_reset_wait(headers: &HeaderMap, now: Timestamp) -> Option<SignedDuration> {
    let reset = header_text(headers, RATE_RESET)?.parse::<i64>().ok()?;
    let at = Timestamp::from_second(reset).ok()?;
    Some(at.duration_since(now).max(SignedDuration::ZERO))
}

fn header_text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok().map(str::trim)
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
