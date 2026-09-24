use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use reqwest::Client;

use super::raw::RawBalance;
use crate::bearer;

const BALANCE_PATH: &str = "/user/balance";
const SERVICE: &str = "DeepSeek";

#[derive(Debug, Clone)]
pub(super) struct BalanceClient {
    http: Client,
    url: String,
}

impl BalanceClient {
    pub(super) fn new(http: Client, api_base: &str) -> BalanceClient {
        BalanceClient {
            http,
            url: format!("{}{BALANCE_PATH}", api_base.trim_end_matches('/')),
        }
    }

    pub(super) async fn balance(
        &self,
        key: &str,
        now: Timestamp,
    ) -> Result<RawBalance, ProviderError> {
        bearer::get_json(&self.http, &self.url, key, SERVICE, now).await
    }
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::decimal::ExactMicros;

    const BALANCE: &str = include_str!("fixtures/balance.json");
    const INVALID_KEY: &str = include_str!("fixtures/invalid_key.json");

    fn now() -> Timestamp {
        "2026-09-24T10:00:00Z".parse().unwrap()
    }

    async fn serving(status: u16, body: &str) -> (MockServer, BalanceClient) {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/balance"))
            .and(header("authorization", "Bearer sk-test"))
            .and(header("accept", "application/json"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .mount(&server)
            .await;
        let client = BalanceClient::new(
            crate::http::client().unwrap(),
            &format!("{}/", server.uri()),
        );
        (server, client)
    }

    #[tokio::test]
    async fn the_balance_is_read_with_bearer_auth() {
        let (_server, client) = serving(200, BALANCE).await;
        let raw = client.balance("sk-test", now()).await.unwrap();
        assert!(raw.is_available);
        assert_eq!(raw.balance_infos[0].currency, "CNY");
        assert_eq!(raw.balance_infos[0].total_balance, ExactMicros(110_000_000));
    }

    #[tokio::test]
    async fn an_invalid_key_means_sign_in_expired() {
        let (_server, client) = serving(401, INVALID_KEY).await;
        assert_eq!(
            client.balance("sk-test", now()).await,
            Err(ProviderError::SignInExpired)
        );
    }

    #[tokio::test]
    async fn amounts_that_are_not_decimal_strings_are_invalid() {
        let body =
            r#"{"is_available":true,"balance_infos":[{"currency":"CNY","total_balance":"1.1e2"}]}"#;
        let (_server, client) = serving(200, body).await;
        assert!(matches!(
            client.balance("sk-test", now()).await,
            Err(ProviderError::InvalidResponse(_))
        ));
        let missing = r#"{"balance_infos":[]}"#;
        let (_server, client) = serving(200, missing).await;
        assert!(matches!(
            client.balance("sk-test", now()).await,
            Err(ProviderError::InvalidResponse(_))
        ));
    }
}
