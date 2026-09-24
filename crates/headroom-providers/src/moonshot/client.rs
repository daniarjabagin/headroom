use headroom_core::provider::ProviderError;
use jiff::Timestamp;
use reqwest::Client;

use super::raw::{RawBalance, RawEnvelope};
use super::region::Region;
use crate::bearer;

const BALANCE_PATH: &str = "/v1/users/me/balance";
const SERVICE: &str = "Moonshot";

#[derive(Debug, Clone)]
pub(super) struct BalanceClient {
    http: Client,
    global_url: String,
    mainland_url: String,
}

impl BalanceClient {
    pub(super) fn new(http: Client, global_base: &str, mainland_base: &str) -> BalanceClient {
        BalanceClient {
            http,
            global_url: balance_url(global_base),
            mainland_url: balance_url(mainland_base),
        }
    }

    pub(super) async fn balance(
        &self,
        region: Region,
        key: &str,
        now: Timestamp,
    ) -> Result<RawBalance, ProviderError> {
        let url = match region {
            Region::Global => &self.global_url,
            Region::Mainland => &self.mainland_url,
        };
        let envelope: RawEnvelope = bearer::get_json(&self.http, url, key, SERVICE, now).await?;
        unwrap_envelope(envelope)
    }

    pub(super) async fn locate(
        &self,
        first: Region,
        key: &str,
        now: Timestamp,
    ) -> Result<(Region, RawBalance), ProviderError> {
        match self.balance(first, key, now).await {
            Err(ProviderError::SignInExpired) => {
                let second = first.other();
                let balance = self.balance(second, key, now).await?;
                Ok((second, balance))
            }
            answer => answer.map(|balance| (first, balance)),
        }
    }
}

fn balance_url(base: &str) -> String {
    format!("{}{BALANCE_PATH}", base.trim_end_matches('/'))
}

fn unwrap_envelope(envelope: RawEnvelope) -> Result<RawBalance, ProviderError> {
    match envelope {
        RawEnvelope {
            code: 0,
            status: true,
            data: Some(balance),
        } => Ok(balance),
        RawEnvelope { code, status, .. } => Err(ProviderError::InvalidResponse(format!(
            "Moonshot balance answered code {code}, status {status}, without a balance"
        ))),
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
