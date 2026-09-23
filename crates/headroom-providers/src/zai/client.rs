use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderValue};
use reqwest::{Client, Response, StatusCode};

use crate::http;
use crate::plan_error::mentions_plan;

const QUOTA_PATH: &str = "/api/monitor/usage/quota/limit";
const SUBSCRIPTION_PATH: &str = "/api/biz/subscription/list";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scheme {
    Bearer,
    Raw,
}

#[derive(Debug, Clone)]
pub(super) struct QuotaClient {
    http: Client,
    base: String,
}

impl QuotaClient {
    pub(super) fn new(http: Client, api_base: &str) -> QuotaClient {
        QuotaClient {
            http,
            base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) async fn quota(
        &self,
        key: &str,
        now: Timestamp,
    ) -> Result<(Vec<u8>, Scheme), ProviderError> {
        self.get(QUOTA_PATH, key, &[Scheme::Bearer, Scheme::Raw], now)
            .await
    }

    pub(super) async fn subscriptions(
        &self,
        key: &str,
        scheme: Scheme,
        now: Timestamp,
    ) -> Result<Vec<u8>, ProviderError> {
        let (body, _) = self.get(SUBSCRIPTION_PATH, key, &[scheme], now).await?;
        Ok(body)
    }

    async fn get(
        &self,
        path: &str,
        key: &str,
        schemes: &[Scheme],
        now: Timestamp,
    ) -> Result<(Vec<u8>, Scheme), ProviderError> {
        let url = format!("{}{path}", self.base);
        let mut remaining = schemes.iter().copied().peekable();
        while let Some(scheme) = remaining.next() {
            let response = self.send(&url, key, scheme).await?;
            let status = response.status();
            if status == StatusCode::UNAUTHORIZED && remaining.peek().is_some() {
                continue;
            }
            if !status.is_success() {
                return Err(failure(response, now).await);
            }
            let body = response.bytes().await.map_err(transport_error)?;
            return Ok((body.to_vec(), scheme));
        }
        Err(ProviderError::SignInExpired)
    }

    async fn send(&self, url: &str, key: &str, scheme: Scheme) -> Result<Response, ProviderError> {
        self.http
            .get(url)
            .timeout(TIMEOUT)
            .header(AUTHORIZATION, authorization(key, scheme)?)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(transport_error)
    }
}

fn authorization(key: &str, scheme: Scheme) -> Result<HeaderValue, ProviderError> {
    let text = match scheme {
        Scheme::Bearer => format!("Bearer {key}"),
        Scheme::Raw => key.to_owned(),
    };
    let mut value = HeaderValue::from_str(&text).map_err(|_| {
        ProviderError::LocalData("the Z.ai API key contains characters that cannot be sent".into())
    })?;
    value.set_sensitive(true);
    Ok(value)
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

async fn failure(response: Response, now: Timestamp) -> ProviderError {
    let status = response.status();
    let retry_after = http::retry_after(response.headers(), now);
    let body = response.bytes().await.unwrap_or_default();
    status_error(status, retry_after, &body)
}

fn status_error(
    status: StatusCode,
    retry_after: Option<SignedDuration>,
    body: &[u8],
) -> ProviderError {
    let code = status.as_u16();
    match status {
        StatusCode::FORBIDDEN if mentions_plan(body) => super::no_plan(),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited { retry_after },
        status if status.is_server_error() => {
            ProviderError::Network(format!("Z.ai returned HTTP {code}"))
        }
        _ => ProviderError::InvalidResponse(format!("Z.ai returned HTTP {code}")),
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
