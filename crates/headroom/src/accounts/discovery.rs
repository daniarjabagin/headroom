use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use headroom_core::account::{AccountRef, ProviderKind};
use headroom_core::provider::{Provider, ProviderError};
use headroom_providers::claude::{ClaudeConfig, ClaudeProvider};
use headroom_providers::codex::{CodexConfig, CodexEnvironment, CodexProvider};

pub async fn discover_local() -> Vec<AccountRef> {
    let mut accounts = Vec::new();
    for provider in local_providers() {
        accounts.extend(discover(provider.as_ref()).await);
    }
    accounts
}

pub async fn account_at(provider: ProviderKind, home: &Path) -> Result<Option<AccountRef>> {
    let provider = provider_for_home(provider, home)?;
    let found = discover(provider.as_ref()).await;
    Ok(found.into_iter().find(|account| account.home == home))
}

async fn discover(provider: &dyn Provider) -> Vec<AccountRef> {
    match provider.discover().await {
        Ok(accounts) => accounts,
        Err(ProviderError::NotSignedIn) => Vec::new(),
        Err(error) => {
            tracing::warn!(provider = %provider.kind(), %error, "account discovery failed");
            Vec::new()
        }
    }
}

fn local_providers() -> Vec<Arc<dyn Provider>> {
    let mut providers: Vec<Arc<dyn Provider>> = Vec::new();
    if let Ok(codex) = CodexProvider::new(CodexConfig::from_process()) {
        providers.push(Arc::new(codex));
    }
    if let Ok(claude) = ClaudeConfig::from_env().and_then(ClaudeProvider::new) {
        providers.push(Arc::new(claude));
    }
    providers
}

fn provider_for_home(provider: ProviderKind, home: &Path) -> Result<Arc<dyn Provider>> {
    Ok(match provider {
        ProviderKind::Codex => {
            let environment = CodexEnvironment {
                codex_home: Some(home.display().to_string()),
                home_dir: dirs::home_dir(),
                data_dir: None,
            };
            let config = CodexConfig {
                environment,
                ..CodexConfig::from_process()
            };
            Arc::new(CodexProvider::new(config)?)
        }
        ProviderKind::Claude => {
            let config = ClaudeConfig {
                config_dir: Some(home.to_path_buf()),
                ..ClaudeConfig::from_env()?
            };
            Arc::new(ClaudeProvider::new(config)?)
        }
    })
}
