use std::time::Duration;

use headroom_core::provider::ProviderError;
use jiff::{SignedDuration, Timestamp};
use reqwest::header::ACCEPT;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;

use crate::http;

const TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) async fn get_json<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    key: &str,
    service: &str,
    now: Timestamp,
) -> Result<T, ProviderError> {
    let response = client
        .get(url)
        .timeout(TIMEOUT)
        .bearer_auth(key)
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(transport_error)?;
    let status = response.status();
    if !status.is_success() {
        let retry_after = http::retry_after(response.headers(), now);
        return Err(status_error(status, retry_after, service));
    }
    let body = response.bytes().await.map_err(transport_error)?;
    serde_json::from_slice(&body).map_err(|error| {
        ProviderError::InvalidResponse(format!("cannot parse the {service} response: {error}"))
    })
}

fn transport_error(error: reqwest::Error) -> ProviderError {
    ProviderError::Network(error.without_url().to_string())
}

fn status_error(
    status: StatusCode,
    retry_after: Option<SignedDuration>,
    service: &str,
) -> ProviderError {
    let code = status.as_u16();
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::SignInExpired,
        StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimited { retry_after },
        status if status.is_server_error() => {
            ProviderError::Network(format!("{service} returned HTTP {code}"))
        }
        _ => ProviderError::InvalidResponse(format!("{service} returned HTTP {code}")),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[derive(Debug, PartialEq, Eq, Deserialize)]
    struct Body {
        ok: bool,
    }

    fn now() -> Timestamp {
        "2026-09-24T10:00:00Z".parse().unwrap()
    }

    async fn fetch(response: ResponseTemplate) -> Result<Body, ProviderError> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/balance"))
            .and(header("authorization", "Bearer sk-test"))
            .and(header("accept", "application/json"))
            .respond_with(response)
            .mount(&server)
            .await;
        let url = format!("{}/balance", server.uri());
        let client = http::client().unwrap();
        get_json(&client, &url, "sk-test", "Example", now()).await
    }

    fn reply(status: u16, body: &str) -> ResponseTemplate {
        ResponseTemplate::new(status).set_body_string(body)
    }

    #[tokio::test]
    async fn a_successful_answer_is_parsed() {
        assert_eq!(
            fetch(reply(200, r#"{"ok":true}"#)).await,
            Ok(Body { ok: true })
        );
    }

    #[tokio::test]
    async fn statuses_map_to_provider_errors() {
        for status in [401, 403] {
            assert_eq!(
                fetch(reply(status, "{}")).await,
                Err(ProviderError::SignInExpired)
            );
        }
        let limited = reply(429, "").insert_header("retry-after", "30");
        assert_eq!(
            fetch(limited).await,
            Err(ProviderError::RateLimited {
                retry_after: Some(SignedDuration::from_secs(30))
            })
        );
        assert_eq!(
            fetch(reply(503, "")).await,
            Err(ProviderError::Network("Example returned HTTP 503".into()))
        );
        assert_eq!(
            fetch(reply(404, "")).await,
            Err(ProviderError::InvalidResponse(
                "Example returned HTTP 404".into()
            ))
        );
    }

    #[tokio::test]
    async fn garbage_is_an_invalid_response() {
        let error = fetch(reply(200, "<html>")).await.unwrap_err();
        assert!(
            matches!(&error, ProviderError::InvalidResponse(text) if text.starts_with("cannot parse the Example response")),
            "{error:?}"
        );
    }
}
