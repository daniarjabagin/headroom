use super::*;
use crate::keychain::fake::FakeKeychain;

fn provider() -> ProviderId {
    ProviderId::from_static("tool")
}

fn account() -> AccountId {
    AccountId("tool:0123456789ab".into())
}

fn key(text: &str) -> SecretString {
    SecretString::new(text.into())
}

#[cfg(target_os = "linux")]
#[test]
fn attributes_name_the_application_account_and_provider() {
    use super::backend::attributes;
    use super::service::Attributes;
    let provider = provider();
    let account = account();
    let full = attributes(Some(&provider), &account);
    assert_eq!(
        full,
        Attributes::from([
            ("application", "io.github.headroom"),
            ("provider", "tool"),
            ("account", "tool:0123456789ab"),
        ])
    );
    assert_eq!(attributes(None, &account).len(), 2);
}

#[test]
fn the_platform_store_is_the_keychain_on_macos_and_the_secret_service_on_linux() {
    let cases = [
        (Os::MacOs, SecretBus::Platform, SecretBackend::Keychain),
        (Os::MacOs, SecretBus::Session, SecretBackend::Keychain),
        (Os::MacOs, SecretBus::Disabled, SecretBackend::File),
        (Os::Linux, SecretBus::Platform, SecretBackend::SecretService),
        (Os::Linux, SecretBus::Session, SecretBackend::SecretService),
        (Os::Linux, SecretBus::Disabled, SecretBackend::File),
    ];
    for (os, bus, expected) in cases {
        assert_eq!(backend_kind(os, &bus), expected, "{os:?} {bus:?}");
    }
}

#[tokio::test]
async fn without_a_keyring_keys_use_the_file_fallback() {
    let root = tempfile::tempdir().unwrap();
    let store = SecretStore::new(SecretBus::Disabled, root.path().join("secrets"));
    let key = key("sk-test");
    assert_eq!(store.read(&account()).await.unwrap(), None);
    let backend = store.store(&provider(), &account(), &key).await.unwrap();
    assert_eq!(backend, SecretBackend::File);
    assert_eq!(store.read_secret(&account()).await.unwrap(), Some(key));
    store.delete(&account()).await.unwrap();
    assert_eq!(store.read(&account()).await.unwrap(), None);
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn an_unreachable_bus_falls_back_to_files() {
    let root = tempfile::tempdir().unwrap();
    let address = format!("unix:path={}", root.path().join("no-bus").display());
    let store = SecretStore::new(SecretBus::Address(address), root.path().join("secrets"));
    let key = key("sk-test");
    assert_eq!(
        store.store(&provider(), &account(), &key).await.unwrap(),
        SecretBackend::File
    );
    assert_eq!(store.read(&account()).await.unwrap(), Some(key));
}

#[tokio::test]
async fn foreign_items_are_absent_without_a_reachable_bus() {
    let attributes = [("service", "tool")];
    let disabled = read_foreign(&SecretBus::Disabled, &attributes).await;
    assert_eq!(disabled, ForeignSecret::Absent);
    let root = tempfile::tempdir().unwrap();
    let address = format!("unix:path={}", root.path().join("no-bus").display());
    let unreachable = read_foreign(&SecretBus::Address(address), &attributes).await;
    assert_eq!(unreachable, ForeignSecret::Absent);
}

#[tokio::test]
async fn keychain_keys_live_under_the_headroom_service() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakeKeychain::new(root.path());
    let files = root.path().join("secrets");
    let store = SecretStore::with_keychain(fake.security(), files.clone());
    let key = key("sk-keychain");
    FileSecrets::new(files.clone())
        .write(&account(), &SecretString::new("stale".into()))
        .unwrap();
    let backend = store.store(&provider(), &account(), &key).await.unwrap();
    assert_eq!(backend, SecretBackend::Keychain);
    assert_eq!(
        fake.item("io.github.headroom", Some("tool:0123456789ab"))
            .as_deref(),
        Some("sk-keychain")
    );
    assert!(!files.join("tool:0123456789ab").exists());
    assert!(!fake.argv_log().contains("sk-keychain"));
    assert_eq!(store.read(&account()).await.unwrap(), Some(key));
    store.delete(&account()).await.unwrap();
    assert_eq!(
        fake.item("io.github.headroom", Some("tool:0123456789ab")),
        None
    );
    assert_eq!(store.read(&account()).await.unwrap(), None);
}

#[tokio::test]
async fn a_denied_keychain_falls_back_to_files() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakeKeychain::new(root.path());
    fake.fail_with(51);
    let store = SecretStore::with_keychain(fake.security(), root.path().join("secrets"));
    let key = key("sk-denied");
    let backend = store.store(&provider(), &account(), &key).await.unwrap();
    assert_eq!(backend, SecretBackend::File);
    assert_eq!(store.read(&account()).await.unwrap(), Some(key));
    store.delete(&account()).await.unwrap_err();
}

#[tokio::test]
async fn a_denied_keychain_without_a_file_is_locked() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakeKeychain::new(root.path());
    fake.fail_with(36);
    let store = SecretStore::with_keychain(fake.security(), root.path().join("secrets"));
    assert!(matches!(
        store.read(&account()).await,
        Err(SecretError::Locked)
    ));
}

#[tokio::test]
async fn a_missing_security_tool_uses_files_but_refuses_blind_deletes() {
    let root = tempfile::tempdir().unwrap();
    let security = Security::new(
        root.path().join("absent"),
        std::time::Duration::from_secs(1),
    );
    let store = SecretStore::with_keychain(security, root.path().join("secrets"));
    assert!(matches!(
        store.delete(&account()).await,
        Err(SecretError::Unreachable)
    ));
    let key = key("sk-file");
    let backend = store.store(&provider(), &account(), &key).await.unwrap();
    assert_eq!(backend, SecretBackend::File);
    store.delete(&account()).await.unwrap();
}
