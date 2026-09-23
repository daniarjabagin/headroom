use std::sync::Arc;

use headroom_core::descriptor::ProviderDescriptor;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::secret::SecretReader;

use crate::antigravity::{self, AntigravityConfig, AntigravityProvider};
use crate::claude::{self, ClaudeConfig, ClaudeProvider};
use crate::codex::{self, CodexConfig, CodexProvider};
use crate::ollama::{self, OllamaConfig, OllamaProvider};

/// What every provider may need; each one takes its own settings from the process environment.
pub struct RegistryContext {
    pub http: reqwest::Client,
    pub secrets: Arc<dyn SecretReader>,
}

type Build = fn(&RegistryContext) -> Result<Arc<dyn Provider>, ProviderError>;

struct Entry {
    descriptor: &'static ProviderDescriptor,
    build: Build,
}

static ENTRIES: [Entry; 4] = [
    Entry {
        descriptor: &codex::DESCRIPTOR,
        build: build_codex,
    },
    Entry {
        descriptor: &claude::DESCRIPTOR,
        build: build_claude,
    },
    Entry {
        descriptor: &antigravity::DESCRIPTOR,
        build: build_antigravity,
    },
    Entry {
        descriptor: &ollama::DESCRIPTOR,
        build: build_ollama,
    },
];

pub fn descriptors() -> impl Iterator<Item = &'static ProviderDescriptor> {
    ENTRIES.iter().map(|entry| entry.descriptor)
}

#[must_use]
pub fn descriptor(id: &str) -> Option<&'static ProviderDescriptor> {
    descriptors().find(|descriptor| descriptor.id.as_str() == id)
}

/// Every compiled-in provider; one that cannot start is logged and left out.
#[must_use]
pub fn build_all(context: &RegistryContext) -> Vec<Arc<dyn Provider>> {
    ENTRIES
        .iter()
        .filter_map(|entry| match (entry.build)(context) {
            Ok(provider) => Some(provider),
            Err(error) => {
                let id = &entry.descriptor.id;
                tracing::error!(provider = %id, %error, "provider unavailable");
                None
            }
        })
        .collect()
}

#[must_use]
pub fn build(
    context: &RegistryContext,
    id: &str,
) -> Option<Result<Arc<dyn Provider>, ProviderError>> {
    ENTRIES
        .iter()
        .find(|entry| entry.descriptor.id.as_str() == id)
        .map(|entry| (entry.build)(context))
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "every registry entry builds through the same fallible signature"
)]
fn build_codex(context: &RegistryContext) -> Result<Arc<dyn Provider>, ProviderError> {
    let config = CodexConfig::from_process();
    Ok(Arc::new(CodexProvider::with_http(
        config,
        context.http.clone(),
    )))
}

fn build_claude(context: &RegistryContext) -> Result<Arc<dyn Provider>, ProviderError> {
    let config = ClaudeConfig::from_env()?;
    Ok(Arc::new(ClaudeProvider::with_http(
        config,
        context.http.clone(),
    )))
}

fn build_antigravity(context: &RegistryContext) -> Result<Arc<dyn Provider>, ProviderError> {
    let config = AntigravityConfig::from_env()?;
    Ok(Arc::new(AntigravityProvider::with_http(
        config,
        context.http.clone(),
    )?))
}

fn build_ollama(context: &RegistryContext) -> Result<Arc<dyn Provider>, ProviderError> {
    let config = OllamaConfig::from_env()?;
    Ok(Arc::new(OllamaProvider::with_http(
        config,
        context.http.clone(),
    )))
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
