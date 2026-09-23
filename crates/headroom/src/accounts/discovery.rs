use std::sync::Arc;

use headroom_core::account::AccountRef;
use headroom_core::provider::{Provider, ProviderError};

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
        Err(ProviderError::NotSignedIn) => Vec::new(),
        Err(error) => {
            tracing::warn!(provider = %provider.id(), %error, "account discovery failed");
            Vec::new()
        }
    }
}
