use std::io::{self, Write};
use std::sync::Arc;

use anyhow::{Context, Result};
use headroom_core::descriptor::ProviderDescriptor;
use headroom_core::provider::Provider;
#[cfg(target_os = "linux")]
use headroom_daemon::BusTarget;
use headroom_daemon::catalog::ProviderCatalog;
use headroom_providers::registry::{self, RegistryContext};
use headroom_providers::secrets::{SecretBus, SecretStore};

use crate::cli::ProvidersArgs;
use crate::paths::Globals;
use crate::render::providers::render_providers;

/// The compiled-in providers with what they need to run in this process.
pub struct LocalRegistry {
    pub secrets: Arc<SecretStore>,
    context: RegistryContext,
}

impl LocalRegistry {
    pub fn new(globals: &Globals, http: reqwest::Client) -> Result<LocalRegistry> {
        let secrets = Arc::new(secret_store(globals)?);
        let context = RegistryContext {
            http,
            secrets: secrets.clone(),
        };
        Ok(LocalRegistry { secrets, context })
    }

    pub fn for_cli(globals: &Globals) -> Result<LocalRegistry> {
        let http = headroom_providers::http::client().context("cannot create the HTTP client")?;
        LocalRegistry::new(globals, http)
    }

    pub fn all(&self) -> Vec<Arc<dyn Provider>> {
        registry::build_all(&self.context)
    }

    pub fn provider(&self, descriptor: &ProviderDescriptor) -> Result<Arc<dyn Provider>> {
        let id = descriptor.id.as_str();
        let built = registry::build(&self.context, id)
            .with_context(|| format!("provider {id} is not compiled in"))?;
        built.with_context(|| format!("the {} provider cannot start", descriptor.display_name))
    }
}

pub fn catalog() -> ProviderCatalog {
    ProviderCatalog::new(registry::descriptors())
}

pub fn descriptor(id: &str) -> Result<&'static ProviderDescriptor> {
    registry::descriptor(id).with_context(|| {
        let known: Vec<&str> = registry::descriptors().map(|d| d.id.as_str()).collect();
        format!(
            "unknown provider {id:?}; known providers: {}",
            known.join(", ")
        )
    })
}

pub fn list(args: &ProvidersArgs) -> Result<()> {
    let payload = catalog().payload();
    let mut stdout = io::stdout().lock();
    if args.json {
        writeln!(stdout, "{}", serde_json::to_string(&payload)?)?;
    } else {
        write!(stdout, "{}", render_providers(&payload))?;
    }
    Ok(())
}

fn secret_store(globals: &Globals) -> Result<SecretStore> {
    let dir = SecretStore::default_dir().context("no home directory is available")?;
    Ok(SecretStore::new(secret_bus(globals), dir))
}

#[cfg(target_os = "linux")]
fn secret_bus(globals: &Globals) -> SecretBus {
    match &globals.bus {
        BusTarget::Session => SecretBus::Session,
        BusTarget::Address(address) => SecretBus::Address(address.clone()),
    }
}

#[cfg(not(target_os = "linux"))]
fn secret_bus(_globals: &Globals) -> SecretBus {
    SecretBus::Platform
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_providers_json_matches_the_snapshot() {
        let actual = serde_json::to_string_pretty(&catalog().payload()).unwrap() + "\n";
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/render/fixtures/providers.json"
        );
        if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
            std::fs::write(path, &actual).unwrap();
            return;
        }
        assert_eq!(actual, include_str!("render/fixtures/providers.json"));
    }

    #[test]
    fn unknown_providers_are_named_with_the_known_ones() {
        let error = descriptor("nope").unwrap_err().to_string();
        assert!(
            error.starts_with("unknown provider \"nope\"; known providers: codex, claude"),
            "{error}"
        );
        assert_eq!(descriptor("claude").unwrap().display_name, "Claude");
    }
}
