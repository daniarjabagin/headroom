use std::sync::Arc;

use headroom_core::account::AccountRef;
use headroom_core::provider::Provider;

pub async fn discover_local(providers: &[Arc<dyn Provider>]) -> Vec<AccountRef> {
    let mut accounts = Vec::new();
    for provider in providers {
        accounts.extend(discover(provider.as_ref()).await);
    }
    accounts
}

async fn discover(provider: &dyn Provider) -> Vec<AccountRef> {
    match provider.discover().await {
        Ok(accounts) => accounts,
        Err(error) if error.is_nothing_to_discover() => {
            tracing::debug!(provider = %provider.id(), %error, "nothing to discover");
            Vec::new()
        }
        Err(error) => {
            tracing::warn!(provider = %provider.id(), %error, "account discovery failed");
            Vec::new()
        }
    }
}
