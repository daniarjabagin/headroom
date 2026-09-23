use std::fmt;
use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use reqwest::header::{ACCEPT, CONTENT_TYPE, COOKIE, HeaderMap};
use reqwest::{Client, RequestBuilder, StatusCode, Url};
use serde::de::DeserializeOwned;

use super::auth::AccessToken;
use super::config::CursorConfig;
use super::jwt;
use super::raw::{
    RawCreditGrants, RawGrokBotUsage, RawPeriodUsage, RawPlanInfoResponse, RawRequestUsage,
    RawStripe,
};
use crate::http;

const DASHBOARD_SERVICE: &str = "/aiserver.v1.DashboardService/";
const PERIOD_USAGE: &str = "GetCurrentPeriodUsage";
const PLAN_INFO: &str = "GetPlanInfo";
const CREDIT_GRANTS: &str = "GetCreditGrantsBalance";
const GROK_BOT_USAGE: &str = "GetSandUsageStatus";
const STRIPE_PATH: &str = "/api/auth/stripe";
const REQUEST_USAGE_PATH: &str = "/api/usage";
const SESSION_COOKIE: &str = "WorkosCursorSessionToken";
const TIMEOUT: Duration = Duration::from_secs(10);

pub(super) struct Session {
    user_id: String,
    cookie: String,
}

impl Session {
    pub(super) fn new(subject: &str, token: &AccessToken) -> Session {
        let user_id = jwt::user_id(subject).to_owned();
        let cookie = format!("{SESSION_COOKIE}={user_id}%3A%3A{}", token.secret());
        Session { user_id, cookie }
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Session(<redacted>)")
    }
}

#[derive(Debug, Clone)]
pub(super) struct CursorClient {
    http: Client,
    api_base: String,
    web_base: String,
}

impl CursorClient {
    pub(super) fn new(http: Client, config: &CursorConfig) -> CursorClient {
        CursorClient {
            http,
            api_base: config.api_base.trim_end_matches('/').to_owned(),
            web_base: config.web_base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) async fn period_usage(
        &self,
        token: &AccessToken,
        now: Timestamp,
    ) -> Result<RawPeriodUsage, ProviderError> {
        self.connect(PERIOD_USAGE, token, now).await
    }

    pub(super) async fn plan_info(
        &self,
        token: &AccessToken,
        now: Timestamp,
    ) -> Result<RawPlanInfoResponse, ProviderError> {
        self.connect(PLAN_INFO, token, now).await
    }

    pub(super) async fn credit_grants(
        &self,
        token: &AccessToken,
        now: Timestamp,
    ) -> Result<RawCreditGrants, ProviderError> {
        self.connect(CREDIT_GRANTS, token, now).await
    }

    pub(super) async fn grok_bot_usage(
        &self,
        token: &AccessToken,
        now: Timestamp,
    ) -> Result<RawGrokBotUsage, ProviderError> {
        self.connect(GROK_BOT_USAGE, token, now).await
    }

    pub(super) async fn stripe(
        &self,
        session: &Session,
        now: Timestamp,
    ) -> Result<RawStripe, ProviderError> {
        let request = self.http.get(format!("{}{STRIPE_PATH}", self.web_base));
        send(request.header(COOKIE, &session.cookie), now).await
    }

    pub(super) async fn request_usage(
        &self,
        session: &Session,
        now: Timestamp,
    ) -> Result<RawRequestUsage, ProviderError> {
        let mut url = Url::parse(&format!("{}{REQUEST_USAGE_PATH}", self.web_base))
            .map_err(|error| ProviderError::LocalData(format!("invalid Cursor URL: {error}")))?;
        url.query_pairs_mut().append_pair("user", &session.user_id);
        let request = self.http.get(url).header(COOKIE, &session.cookie);
        send(request, now).await
    }

    async fn connect<T: DeserializeOwned>(
        &self,
        method: &str,
        token: &AccessToken,
        now: Timestamp,
    ) -> Result<T, ProviderError> {
        let request = self
            .http
            .post(format!("{}{DASHBOARD_SERVICE}{method}", self.api_base))
            .bearer_auth(token.secret())
            .header(CONTENT_TYPE, "application/json")
            .header("Connect-Protocol-Version", "1")
            .body("{}");
        send(request, now).await
    }
}

async fn send<T: DeserializeOwned>(
    request: RequestBuilder,
    now: Timestamp,
) -> Result<T, ProviderError> {
    let response = request
        .timeout(TIMEOUT)
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(transport_error)?;
    let status = response.status();
    if !status.is_success() {
        return Err(status_error(status, response.headers(), now));
    }
    let body = response.bytes().await.map_err(transport_error)?;
    parse(&body)
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

pub(super) fn parse<T: DeserializeOwned>(body: &[u8]) -> Result<T, ProviderError> {
    serde_json::from_slice(body).map_err(|error| {
        ProviderError::InvalidResponse(format!(
            "cannot parse Cursor response at line {} column {}",
            error.line(),
            error.column()
        ))
    })
}

pub(super) fn status_error(
    status: StatusCode,
    headers: &HeaderMap,
    now: Timestamp,
) -> ProviderError {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited {
            retry_after: http::retry_after(headers, now),
        },
        status if status.is_server_error() => {
            ProviderError::Network(format!("Cursor returned HTTP {}", status.as_u16()))
        }
        status => {
            ProviderError::InvalidResponse(format!("Cursor returned HTTP {}", status.as_u16()))
        }
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
