use std::sync::Arc;

use headroom_core::descriptor::ProviderDescriptor;
use headroom_core::provider::{Provider, ProviderError};
use headroom_core::secret::SecretReader;

use crate::claude::{self, ClaudeConfig, ClaudeProvider};
use crate::codex::{self, CodexConfig, CodexProvider};
use crate::grok::{self, GrokConfig, GrokProvider};
use crate::kimi::{self, KimiConfig, KimiProvider};
use crate::minimax::{self, MiniMaxConfig, MiniMaxProvider};
use crate::opencode::{self, OpenCodeConfig, OpenCodeProvider};
use crate::openrouter::{self, OpenRouterConfig, OpenRouterProvider};
use crate::zai::{self, ZaiConfig, ZaiProvider};

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

static ENTRIES: [Entry; 8] = [
    Entry {
        descriptor: &codex::DESCRIPTOR,
        build: build_codex,
    },
    Entry {
        descriptor: &claude::DESCRIPTOR,
        build: build_claude,
    },
    Entry {
        descriptor: &opencode::DESCRIPTOR,
        build: build_opencode,
    },
    Entry {
        descriptor: &openrouter::DESCRIPTOR,
        build: build_openrouter,
    },
    Entry {
        descriptor: &zai::DESCRIPTOR,
        build: build_zai,
    },
    Entry {
        descriptor: &kimi::DESCRIPTOR,
        build: build_kimi,
    },
    Entry {
        descriptor: &minimax::DESCRIPTOR,
        build: build_minimax,
    },
    Entry {
        descriptor: &grok::DESCRIPTOR,
        build: build_grok,
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

fn build_opencode(context: &RegistryContext) -> Result<Arc<dyn Provider>, ProviderError> {
    let config = OpenCodeConfig::from_env()?;
    Ok(Arc::new(OpenCodeProvider::with_http(
        config,
        context.http.clone(),
        Arc::clone(&context.secrets),
    )))
}

fn build_openrouter(context: &RegistryContext) -> Result<Arc<dyn Provider>, ProviderError> {
    let config = OpenRouterConfig::from_process()?;
    let secrets = Arc::clone(&context.secrets);
    Ok(Arc::new(OpenRouterProvider::new(
        config,
        context.http.clone(),
        secrets,
    )))
}

fn build_zai(context: &RegistryContext) -> Result<Arc<dyn Provider>, ProviderError> {
    let config = ZaiConfig::from_process()?;
    let secrets = Arc::clone(&context.secrets);
    Ok(Arc::new(ZaiProvider::new(
        config,
        context.http.clone(),
        secrets,
    )))
}

fn build_kimi(context: &RegistryContext) -> Result<Arc<dyn Provider>, ProviderError> {
    let config = KimiConfig::from_env()?;
    let http = context.http.clone();
    Ok(Arc::new(KimiProvider::with_http(
        config,
        http,
        context.secrets.clone(),
    )))
}

fn build_minimax(context: &RegistryContext) -> Result<Arc<dyn Provider>, ProviderError> {
    let config = MiniMaxConfig::from_env()?;
    let http = context.http.clone();
    Ok(Arc::new(MiniMaxProvider::with_http(
        config,
        http,
        context.secrets.clone(),
    )))
}

fn build_grok(context: &RegistryContext) -> Result<Arc<dyn Provider>, ProviderError> {
    let config = GrokConfig::from_env()?;
    Ok(Arc::new(GrokProvider::with_http(
        config,
        context.http.clone(),
    )))
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
