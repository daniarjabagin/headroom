use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use reqwest::header::{ACCEPT, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};

use super::raw::{RawResponse, RawUser, RawUserResult};
use crate::http;

const GRAPHQL_PATH: &str = "/graphql/v2";
const OPERATION: &str = "GetRequestLimitInfo";
const TIMEOUT: Duration = Duration::from_secs(15);
const WARP_USER_AGENT: &str = "Warp/1.0";
const CLIENT_ID: &str = "warp-app";
const OS_VERSION: &str = "unknown";
const MAX_MESSAGE_CHARS: usize = 200;

const QUERY: &str = "query GetRequestLimitInfo($requestContext: RequestContext!) {
  user(requestContext: $requestContext) {
    __typename
    ... on UserOutput {
      user {
        requestLimitInfo {
          isUnlimited
          nextRefreshTime
          requestLimit
          requestsUsedSinceLastRefresh
        }
        bonusGrants {
          requestCreditsGranted
          requestCreditsRemaining
          expiration
        }
        workspaces {
          bonusGrantsInfo {
            grants {
              requestCreditsGranted
              requestCreditsRemaining
              expiration
            }
          }
        }
      }
    }
    ... on UserFacingError {
      error {
        message
      }
    }
  }
}";

#[derive(Debug, Clone)]
pub(super) struct WarpClient {
    http: Client,
    base: String,
}

impl WarpClient {
    pub(super) fn new(http: Client, api_base: &str) -> WarpClient {
        WarpClient {
            http,
            base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    pub(super) async fn request_limits(
        &self,
        key: &str,
        now: Timestamp,
    ) -> Result<RawUser, ProviderError> {
        let response = self
            .http
            .post(format!("{}{GRAPHQL_PATH}?op={OPERATION}", self.base))
            .timeout(TIMEOUT)
            .bearer_auth(key)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(USER_AGENT, WARP_USER_AGENT)
            .header("x-warp-client-id", CLIENT_ID)
            .header("x-warp-os-category", os_name())
            .header("x-warp-os-name", os_name())
            .header("x-warp-os-version", OS_VERSION)
            .body(request_body().to_string())
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            let retry_after = http::retry_after(response.headers(), now);
            return Err(status_error(status, retry_after));
        }
        let body = response.bytes().await.map_err(transport_error)?;
        parse_user(&body)
    }
}

pub(super) fn request_body() -> Value {
    json!({
        "operationName": OPERATION,
        "query": QUERY,
        "variables": {
            "requestContext": {
                "clientContext": {},
                "osContext": {
                    "category": os_name(),
                    "name": os_name(),
                    "version": OS_VERSION,
                },
            },
        },
    })
}

fn os_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    }
}

pub(super) fn parse_user(body: &[u8]) -> Result<RawUser, ProviderError> {
    let response: RawResponse = serde_json::from_slice(body).map_err(|error| {
        ProviderError::InvalidResponse(format!("cannot parse the Warp usage response: {error}"))
    })?;
    match (response.data, response.errors.first()) {
        (Some(data), _) => user_result(data.user),
        (None, Some(error)) => Err(answered(&error.message)),
        (None, None) => Err(ProviderError::InvalidResponse(
            "the Warp usage response has no data".to_owned(),
        )),
    }
}

fn user_result(result: RawUserResult) -> Result<RawUser, ProviderError> {
    match result {
        RawUserResult::UserOutput { user } => Ok(user),
        RawUserResult::UserFacingError { error } => Err(answered(&error.message)),
        RawUserResult::Unknown => Err(ProviderError::InvalidResponse(
            "the Warp usage response has an unknown user type".to_owned(),
        )),
    }
}

fn answered(message: &str) -> ProviderError {
    let short: String = message.chars().take(MAX_MESSAGE_CHARS).collect();
    ProviderError::InvalidResponse(format!("Warp answered: {short}"))
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

fn status_error(status: StatusCode, retry_after: Option<SignedDuration>) -> ProviderError {
    let code = status.as_u16();
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited { retry_after },
        status if status.is_server_error() => {
            ProviderError::Network(format!("Warp usage endpoint returned HTTP {code}"))
        }
        _ => ProviderError::InvalidResponse(format!("Warp usage endpoint returned HTTP {code}")),
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
