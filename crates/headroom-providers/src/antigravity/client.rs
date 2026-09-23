use std::time::Duration;

use headroom_core::provider::ProviderError;
use headroom_core::secret::SecretString;
use jiff::SignedDuration;
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::{Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::json;

use super::raw::{RawCodeAssist, RawSummaryEnvelope};
use crate::http;

pub const DEFAULT_CLOUD_BASES: [&str; 2] = [
    "https://daily-cloudcode-pa.googleapis.com",
    "https://cloudcode-pa.googleapis.com",
];
const LS_SERVICE: &str = "exa.language_server_pb.LanguageServerService";
const LS_TIMEOUT: Duration = Duration::from_secs(5);
const CLOUD_TIMEOUT: Duration = Duration::from_secs(15);
const SUMMARY_PATH: &str = "/v1internal:retrieveUserQuotaSummary";
const CODE_ASSIST_PATH: &str = "/v1internal:loadCodeAssist";

#[derive(Debug, Clone)]
pub(super) struct LanguageServerClient {
    http: Client,
}

impl LanguageServerClient {
    pub(super) fn new() -> Result<LanguageServerClient, ProviderError> {
        let http = Client::builder()
            .danger_accept_invalid_certs(true)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(LS_TIMEOUT)
            .build()
            .map_err(|error| ProviderError::Network(error.without_url().to_string()))?;
        Ok(LanguageServerClient { http })
    }

    pub(super) async fn call<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        csrf: &str,
        method: &str,
    ) -> Option<T> {
        let body = json!({ "metadata": {
            "ideName": "antigravity",
            "extensionName": "antigravity",
            "ideVersion": "unknown",
            "locale": "en"
        }});
        let response = self
            .http
            .post(format!("{endpoint}/{LS_SERVICE}/{method}"))
            .header("Connect-Protocol-Version", "1")
            .header("x-codeium-csrf-token", csrf)
            .json(&body)
            .send()
            .await
            .map_err(|error| tracing::debug!(error = %error.without_url(), method, "language server unreachable"))
            .ok()?;
        if !response.status().is_success() {
            tracing::debug!(
                status = response.status().as_u16(),
                method,
                "language server refused"
            );
            return None;
        }
        response.json().await.ok()
    }
}

#[derive(Debug, Clone)]
pub(super) struct CloudClient {
    http: Client,
    bases: Vec<String>,
}

impl CloudClient {
    pub(super) fn new(http: Client, bases: &[String]) -> CloudClient {
        CloudClient {
            http,
            bases: bases
                .iter()
                .map(|base| base.trim_end_matches('/').to_owned())
                .collect(),
        }
    }

    pub(super) async fn quota_summary(
        &self,
        token: &SecretString,
    ) -> Result<RawSummaryEnvelope, ProviderError> {
        self.post(SUMMARY_PATH, token, "antigravity").await
    }

    pub(super) async fn code_assist(
        &self,
        token: &SecretString,
    ) -> Result<RawCodeAssist, ProviderError> {
        self.post(CODE_ASSIST_PATH, token, "agy").await
    }

    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        token: &SecretString,
        agent: &str,
    ) -> Result<T, ProviderError> {
        let mut last = ProviderError::Network(format!("no Cloud Code endpoint answered {path}"));
        for base in &self.bases {
            match self.send(base, path, token, agent).await {
                Ok(response) if response.status().is_success() => {
                    return parse(response, path).await;
                }
                Ok(response) => match status_error(&response, path) {
                    retry @ (ProviderError::Network(_) | ProviderError::InvalidResponse(_)) => {
                        last = retry;
                    }
                    fatal => return Err(fatal),
                },
                Err(error) => last = ProviderError::Network(error.without_url().to_string()),
            }
        }
        Err(last)
    }

    async fn send(
        &self,
        base: &str,
        path: &str,
        token: &SecretString,
        agent: &str,
    ) -> reqwest::Result<Response> {
        self.http
            .post(format!("{base}{path}"))
            .timeout(CLOUD_TIMEOUT)
            .bearer_auth(token.expose())
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, agent)
            .json(&json!({}))
            .send()
            .await
    }
}

async fn parse<T: DeserializeOwned>(response: Response, path: &str) -> Result<T, ProviderError> {
    let body = response
        .bytes()
        .await
        .map_err(|error| ProviderError::Network(error.without_url().to_string()))?;
    serde_json::from_slice(&body).map_err(|error| {
        ProviderError::InvalidResponse(format!(
            "cannot parse the Cloud Code {path} response at line {} column {}",
            error.line(),
            error.column()
        ))
    })
}

fn status_error(response: &Response, path: &str) -> ProviderError {
    let retry_after = http::retry_after_seconds(response.headers());
    error_for(response.status(), retry_after, path)
}

pub(super) fn error_for(
    status: StatusCode,
    retry_after: Option<SignedDuration>,
    path: &str,
) -> ProviderError {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited { retry_after },
        status if status.is_server_error() => ProviderError::Network(format!(
            "Cloud Code {path} returned HTTP {}",
            status.as_u16()
        )),
        status => ProviderError::InvalidResponse(format!(
            "Cloud Code {path} returned HTTP {}",
            status.as_u16()
        )),
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
