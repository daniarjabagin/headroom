#![cfg(test)]

mod fake_keyring;

use std::collections::HashMap;

use fake_keyring::{
    COLLECTION, FakeCollection, FakeService, FakeSession, PrivateBus, ROOT, SESSION, Shared,
    StoredItem,
};
use headroom_core::account::{AccountId, ProviderId};
use headroom_core::secret::SecretString;
use headroom_providers::secrets::{SecretBackend, SecretBus, SecretError, SecretStore};
use tempfile::TempDir;

const TOOL: ProviderId = ProviderId::from_static("tool");

struct Fixture {
    _bus: PrivateBus,
    _server: zbus::Connection,
    keyring: Shared,
    dir: TempDir,
    store: SecretStore,
}

async fn fixture(with_service: bool) -> Option<Fixture> {
    let Some(bus) = PrivateBus::start() else {
        eprintln!("dbus-daemon unavailable, skipping");
        return None;
    };
    let keyring = Shared::default();
    let mut builder = zbus::connection::Builder::address(bus.address.as_str()).unwrap();
    if with_service {
        builder = builder
            .name("org.freedesktop.secrets")
            .unwrap()
            .serve_at(ROOT, FakeService(keyring.clone()))
            .unwrap()
            .serve_at(COLLECTION, FakeCollection(keyring.clone()))
            .unwrap()
            .serve_at(SESSION, FakeSession(keyring.clone()))
            .unwrap();
    }
    let server = builder.build().await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = SecretStore::new(
        SecretBus::Address(bus.address.clone()),
        dir.path().join("secrets"),
    );
    Some(Fixture {
        _bus: bus,
        _server: server,
        keyring,
        dir,
        store,
    })
}

fn account() -> AccountId {
    AccountId("tool:0123456789ab".into())
}

fn key(text: &str) -> SecretString {
    SecretString::new(text.into())
}

impl Fixture {
    fn file(&self) -> std::path::PathBuf {
        self.dir.path().join("secrets").join(account().0)
    }

    fn live_items(&self) -> Vec<StoredItem> {
        self.keyring
            .lock()
            .unwrap()
            .items
            .iter()
            .flatten()
            .cloned()
            .collect()
    }
}

#[tokio::test]
async fn keys_go_to_the_default_collection_with_headroom_attributes() {
    let Some(fx) = fixture(true).await else {
        return;
    };
    let backend = fx
        .store
        .store(&TOOL, &account(), &key("sk-1"))
        .await
        .unwrap();
    assert_eq!(backend, SecretBackend::SecretService);
    fx.store
        .store(&TOOL, &account(), &key("sk-2"))
        .await
        .unwrap();
    let items = fx.live_items();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].label, "Headroom API key for tool:0123456789ab");
    assert_eq!(items[0].value, b"sk-2");
    assert_eq!(items[0].content_type, "text/plain; charset=utf8");
    let expected: HashMap<String, String> = [
        ("application", "io.github.headroom"),
        ("provider", "tool"),
        ("account", "tool:0123456789ab"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    assert_eq!(items[0].attributes, expected);
    assert!(!fx.file().exists());
    assert_eq!(fx.store.read(&account()).await.unwrap(), Some(key("sk-2")));
    fx.store.delete(&account()).await.unwrap();
    assert!(fx.live_items().is_empty());
    assert_eq!(fx.store.read(&account()).await.unwrap(), None);
    assert_eq!(fx.keyring.lock().unwrap().closed_sessions, 4);
}

#[tokio::test]
async fn a_locked_or_missing_keyring_falls_back_to_a_private_file() {
    let Some(fx) = fixture(true).await else {
        return;
    };
    fx.keyring.lock().unwrap().locked = true;
    let backend = fx
        .store
        .store(&TOOL, &account(), &key("sk-locked"))
        .await
        .unwrap();
    assert_eq!(backend, SecretBackend::File);
    assert_eq!(std::fs::read(fx.file()).unwrap(), b"sk-locked");
    assert_eq!(
        fx.store.read(&account()).await.unwrap(),
        Some(key("sk-locked"))
    );
    {
        let mut keyring = fx.keyring.lock().unwrap();
        keyring.locked = false;
        keyring.no_default = true;
    }
    let other = AccountId("tool:ba9876543210".into());
    let backend = fx.store.store(&TOOL, &other, &key("sk-x")).await.unwrap();
    assert_eq!(backend, SecretBackend::File);
    assert!(fx.live_items().is_empty());
}

#[tokio::test]
async fn a_locked_item_without_a_file_copy_is_reported() {
    let Some(fx) = fixture(true).await else {
        return;
    };
    fx.store
        .store(&TOOL, &account(), &key("sk-1"))
        .await
        .unwrap();
    fx.keyring.lock().unwrap().locked = true;
    assert!(matches!(
        fx.store.read(&account()).await,
        Err(SecretError::Locked)
    ));
    assert!(matches!(
        fx.store.delete(&account()).await,
        Err(SecretError::Locked)
    ));
}

#[tokio::test]
async fn a_bus_without_a_secret_service_uses_files() {
    let Some(fx) = fixture(false).await else {
        return;
    };
    let backend = fx
        .store
        .store(&TOOL, &account(), &key("sk-1"))
        .await
        .unwrap();
    assert_eq!(backend, SecretBackend::File);
    assert_eq!(fx.store.read(&account()).await.unwrap(), Some(key("sk-1")));
    fx.store.delete(&account()).await.unwrap();
    assert!(!fx.file().exists());
}
