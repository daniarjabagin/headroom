use std::path::PathBuf;

use headroom_core::account::{AccountRef, CredentialOwner};
use headroom_core::provider::ProviderError;
use jiff::tz::TimeZone;
use serde_json::{Value, json};

use super::*;
use crate::home::HomeDisplay;
use crate::model::{AccountRuntime, RefreshFailure};
use crate::storage::accounts::AccountRecord;
use crate::testing::{CODEX, account, catalog, ts};

const NOW: &str = "2026-09-23T10:00:00Z";

fn record(name: &str, owner: CredentialOwner, order: i64) -> AccountRecord {
    AccountRecord {
        reference: AccountRef {
            owner,
            ..account(CODEX, name)
        },
        label: None,
        hidden: false,
        sort_order: order,
        email: None,
        plan: None,
        last_seen: ts(NOW),
        gone: false,
    }
}

fn failing(model: &mut Model, record: &AccountRecord, error: ProviderError) {
    let runtime = AccountRuntime {
        failure: Some(RefreshFailure::Provider(error)),
        failures: 1,
        refreshing: true,
        ..AccountRuntime::default()
    };
    model.runtime.insert(record.id().clone(), runtime);
}

fn model() -> Model {
    let healthy = record("healthy", CredentialOwner::Cli, 0);
    let cli = record("cli", CredentialOwner::Cli, 1);
    let owned = record("owned", CredentialOwner::Headroom, 2);
    let moved = record("moved", CredentialOwner::Cli, 3);
    let mut model = Model {
        accounts: vec![healthy, cli.clone(), owned.clone(), moved.clone()],
        ..Model::default()
    };
    failing(&mut model, &cli, ProviderError::SignInExpired);
    failing(&mut model, &owned, ProviderError::SignInExpired);
    let changed = ProviderError::AccountChanged("the Codex account has changed".into());
    failing(&mut model, &moved, changed);
    model
}

fn recoveries(model: &Model) -> Value {
    let homes = HomeDisplay::new(Some(PathBuf::from("/home/ada")));
    let ctx = AssembleContext {
        now: ts(NOW),
        tz: &TimeZone::UTC,
        homes: &homes,
        catalog: &catalog(),
    };
    let payload = serde_json::to_value(assemble(model, &ctx)).unwrap();
    let accounts = payload["accounts"].as_array().unwrap().iter();
    accounts
        .map(|a| json!({ "id": a["id"], "status": a["status"], "error": a["error"]["kind"], "recovery": a["recovery"] }))
        .collect()
}

#[test]
fn each_failing_account_carries_the_recovery_the_daemon_chose() {
    let expected = json!([
        { "id": "codex:healthy", "status": "stale", "error": null, "recovery": null },
        {
            "id": "codex:cli",
            "status": "refreshing",
            "error": "sign_in_expired",
            "recovery": { "action": "cli_login", "command": "codex login", "account_id": "codex:cli" }
        },
        {
            "id": "codex:owned",
            "status": "refreshing",
            "error": "sign_in_expired",
            "recovery": { "action": "sign_in", "account_id": "codex:owned" }
        },
        {
            "id": "codex:moved",
            "status": "refreshing",
            "error": "account_changed",
            "recovery": { "action": "retry" }
        }
    ]);
    assert_eq!(recoveries(&model()), expected);
}

#[test]
fn a_payload_without_recovery_still_parses() {
    let homes = HomeDisplay::new(Some(PathBuf::from("/home/ada")));
    let ctx = AssembleContext {
        now: ts(NOW),
        tz: &TimeZone::UTC,
        homes: &homes,
        catalog: &catalog(),
    };
    let mut payload = serde_json::to_value(assemble(&model(), &ctx)).unwrap();
    for account in payload["accounts"].as_array_mut().unwrap() {
        account.as_object_mut().unwrap().remove("recovery");
    }
    let parsed: StatePayload = serde_json::from_value(payload).unwrap();
    assert!(parsed.accounts.iter().all(|a| a.recovery.is_none()));
}
