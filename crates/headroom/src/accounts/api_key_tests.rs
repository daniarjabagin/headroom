use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use async_trait::async_trait;
use headroom_core::account::ProviderId;
use headroom_core::cursor::LogCursors;
use headroom_core::descriptor::{AddAccountMethod, ApiKeyPrompt, ProviderDescriptor};
use headroom_core::event::UsageEvent;
use headroom_core::provider::ProviderError;
use headroom_core::quota::LimitsSnapshot;
use headroom_daemon::BusTarget;
use headroom_providers::secrets::SecretBus;
use tempfile::TempDir;

use super::*;

static KEYED: ProviderDescriptor = ProviderDescriptor {
    id: ProviderId::from_static("keyed"),
    display_name: "Keyed",
    add_account: &[AddAccountMethod::ApiKey(ApiKeyPrompt {
        label: "API key",
        console_url: "https://keyed.example/keys",
        hint: "Starts with kd-",
    })],
    multi_account: true,
    local_usage: false,
};

struct KeyedProvider {
    root: PathBuf,
}

#[async_trait]
impl Provider for KeyedProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &KEYED
    }

    async fn discover(&self) -> Result<Vec<AccountRef>, ProviderError> {
        key_accounts::discover(&self.root.join("keyed"), &KEYED.id)
    }

    async fn usage_homes(&self) -> Result<Vec<PathBuf>, ProviderError> {
        Ok(Vec::new())
    }

    async fn fetch_limits(&self, _: &AccountRef) -> Result<LimitsSnapshot, ProviderError> {
        Err(ProviderError::NotSignedIn)
    }

    fn read_usage(&self, _: &Path, _: &mut LogCursors) -> Result<Vec<UsageEvent>, ProviderError> {
        Ok(Vec::new())
    }

    async fn validate_key(&self, key: &str) -> Result<AccountIdentity, ProviderError> {
        if key.starts_with("kd-") {
            Ok(identity())
        } else {
            Err(ProviderError::NotSignedIn)
        }
    }
}

fn identity() -> AccountIdentity {
    AccountIdentity {
        email: Some("ada@keyed.example".into()),
        plan: None,
        stable_key: "acct-1".into(),
    }
}

struct Sandbox {
    dir: TempDir,
    provider: KeyedProvider,
    secrets: SecretStore,
    globals: Globals,
}

impl Sandbox {
    fn new() -> Sandbox {
        let dir = tempfile::tempdir().unwrap();
        let bus = format!("unix:path={}", dir.path().join("no-bus").display());
        Sandbox {
            provider: KeyedProvider {
                root: dir.path().join("accounts"),
            },
            secrets: SecretStore::new(SecretBus::Disabled, dir.path().join("secrets")),
            globals: Globals {
                bus: BusTarget::Address(bus),
                db: None,
            },
            dir,
        }
    }

    async fn add(
        &self,
        input: &str,
        cancel: &Cancel,
    ) -> (Result<(String, Option<String>)>, Vec<ProgressEvent>) {
        let target = KeyTarget {
            provider: &self.provider,
            secrets: &self.secrets,
            root: &self.provider.root,
        };
        let mut events = Vec::new();
        let mut record = |event: ProgressEvent| {
            events.push(event);
            Ok(())
        };
        let input = Cursor::new(input.to_owned());
        let result = add(
            &self.globals,
            &target,
            input,
            Some("Work"),
            &mut record,
            cancel,
        )
        .await;
        (result, events)
    }

    fn homes(&self) -> Vec<PathBuf> {
        match fs::read_dir(self.provider.root.join("keyed")) {
            Ok(entries) => entries.map(|entry| entry.unwrap().path()).collect(),
            Err(_) => Vec::new(),
        }
    }

    fn secret_file(&self, id: &str) -> PathBuf {
        self.dir.path().join("secrets").join(id)
    }
}

#[tokio::test]
async fn a_validated_key_becomes_a_headroom_account() {
    let sandbox = Sandbox::new();
    let (result, events) = sandbox.add("  kd-good \n", &Cancel::default()).await;
    let (id, label) = result.unwrap();
    assert_eq!(id, identity().account_id(&KEYED.id).0);
    assert_eq!(label, None);
    let homes = sandbox.homes();
    assert_eq!(homes.len(), 1);
    let started = ProgressEvent::Started {
        provider: KEYED.id.clone(),
        home: homes[0].display().to_string(),
    };
    assert_eq!(events, [started]);
    assert_eq!(
        fs::read_to_string(sandbox.secret_file(&id)).unwrap(),
        "kd-good"
    );
    assert_eq!(
        key_accounts::load_record(&homes[0]).unwrap(),
        Some(identity())
    );
    let account = sandbox
        .provider
        .account_at(&homes[0])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.id.0, id);
    assert_eq!(account.owner, CredentialOwner::Headroom);
}

#[tokio::test]
async fn rejected_keys_leave_nothing_behind_and_are_never_echoed() {
    let sandbox = Sandbox::new();
    let (result, events) = sandbox.add("sk-secret-value\n", &Cancel::default()).await;
    let message = format!("{:#}", result.unwrap_err());
    assert_eq!(message, "the API key was not accepted: not signed in");
    assert!(events.is_empty());
    assert!(sandbox.homes().is_empty());
    let (empty, _) = sandbox.add("\n", &Cancel::default()).await;
    assert_eq!(empty.unwrap_err().to_string(), "no API key on stdin");
}

#[tokio::test]
async fn adding_the_same_account_again_replaces_its_key() {
    let sandbox = Sandbox::new();
    let (first, _) = sandbox.add("kd-first\n", &Cancel::default()).await;
    let (second, events) = sandbox.add("kd-rotated\n", &Cancel::default()).await;
    let id = first.unwrap().0;
    assert_eq!(second.unwrap().0, id);
    assert_eq!(sandbox.homes().len(), 1);
    assert_eq!(events.len(), 1);
    assert_eq!(
        fs::read_to_string(sandbox.secret_file(&id)).unwrap(),
        "kd-rotated"
    );
}

#[tokio::test]
async fn a_cancelled_add_stores_nothing() {
    let sandbox = Sandbox::new();
    let cancel = Cancel::default();
    cancel.cancel();
    let (result, events) = sandbox.add("kd-good\n", &cancel).await;
    assert_eq!(result.unwrap_err().to_string(), "cancelled");
    assert!(events.is_empty());
    assert!(sandbox.homes().is_empty());
    let id = identity().account_id(&KEYED.id).0;
    assert!(!sandbox.secret_file(&id).exists());
}
