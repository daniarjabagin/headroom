use std::collections::BTreeSet;
use std::path::PathBuf;

use headroom_core::account::AccountId;
use headroom_core::descriptor::{AddAccountMethod, ProviderDescriptor};
use headroom_core::secret::SecretString;

use super::*;
use crate::secrets::{SecretBus, SecretStore};

fn context() -> RegistryContext {
    let store = SecretStore::new(SecretBus::Disabled, PathBuf::from("/nonexistent/secrets"));
    RegistryContext {
        http: crate::http::client().unwrap(),
        secrets: Arc::new(store),
    }
}

#[test]
fn every_descriptor_is_valid_and_ids_are_unique() {
    let mut seen = BTreeSet::new();
    for descriptor in descriptors() {
        assert_eq!(descriptor.validate(), Ok(()), "{}", descriptor.id);
        assert!(
            seen.insert(descriptor.id.as_str()),
            "duplicate {}",
            descriptor.id
        );
    }
    assert_eq!(
        seen.into_iter().collect::<Vec<_>>(),
        ["claude", "codex", "grok"]
    );
}

#[test]
fn codex_and_claude_sign_in_with_their_clis() {
    let program = |id: &str| match descriptor(id).and_then(ProviderDescriptor::default_method) {
        Some(AddAccountMethod::CliLogin(login)) => login.program,
        other => panic!("{id}: {other:?}"),
    };
    assert_eq!(program("codex"), "codex");
    assert_eq!(program("claude"), "claude");
    assert_eq!(descriptor("codex").unwrap().display_name, "Codex");
    assert!(descriptor("cursor").is_none());
}

#[test]
fn built_providers_follow_the_registry_order() {
    let providers = build_all(&context());
    let ids: Vec<_> = providers.iter().map(|p| p.id().as_str()).collect();
    assert_eq!(ids, ["codex", "claude", "grok"]);
    for provider in &providers {
        assert!(std::ptr::eq(
            provider.descriptor(),
            descriptor(provider.id().as_str()).unwrap()
        ));
    }
    assert!(build(&context(), "claude").unwrap().is_ok());
    assert!(build(&context(), "nope").is_none());
}

#[tokio::test]
async fn the_context_hands_providers_a_secret_reader() {
    let ctx = context();
    let account = AccountId("tool:0123456789ab".into());
    let missing: Option<SecretString> = ctx.secrets.read_secret(&account).await.unwrap();
    assert_eq!(missing, None);
}
