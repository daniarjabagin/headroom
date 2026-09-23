use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use headroom_core::provider::Provider;

use super::*;
use crate::dbus::publisher::publish_changes;
use crate::dbus::signals::SignalSink;
use crate::scheduler::{FirstRefresh, Scheduler};
use crate::testing::CODEX;
use crate::testing::{FakeProvider, Harness, account, eventually, harness, session, snapshot};

async fn two_accounts() -> (Harness, Arc<FakeProvider>) {
    let limits = snapshot(
        vec![session(20.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    );
    let accounts = vec![account(CODEX, "a"), account(CODEX, "b")];
    let provider = Arc::new(FakeProvider::new(CODEX, accounts, limits));
    let dynamic: Arc<dyn Provider> = provider.clone();
    (harness(vec![dynamic]).await, provider)
}

fn settled(harness: &Harness, accounts: usize) -> bool {
    let model = harness.core.model();
    model.runtime.len() == accounts
        && model
            .runtime
            .values()
            .all(|r| !r.refreshing && r.last_attempt.is_some())
}

fn listed(harness: &Harness) -> Vec<(String, Option<String>, bool)> {
    harness
        .core
        .state()
        .accounts
        .into_iter()
        .map(|a| (a.id, a.label, a.hidden))
        .collect()
}

#[tokio::test]
async fn settings_are_validated_persisted_and_served() {
    let (harness, _) = two_accounts().await;
    let core = &harness.core;
    core.set_settings(r#"{"refresh_interval_secs":120,"reduced_motion":true}"#)
        .await
        .unwrap();
    let stored = harness
        .storage
        .blocking(|conn| crate::storage::settings::load(conn))
        .unwrap();
    assert_eq!(stored.refresh_interval_secs, 120);
    assert!(stored.reduced_motion);
    assert!(matches!(
        core.set_settings(r#"{"refresh_interval_secs":5}"#).await,
        Err(CommandError::Settings(_))
    ));
    let served: Settings = serde_json::from_str(&core.settings_json().unwrap()).unwrap();
    assert_eq!(served, stored);
}

fn stored_settings(harness: &Harness) -> Settings {
    harness
        .storage
        .blocking(|conn| crate::storage::settings::load(conn))
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_patches_never_lose_each_other() {
    let (harness, _) = two_accounts().await;
    let tasks: Vec<_> = (0..16)
        .map(|n| {
            let core = harness.core.clone();
            tokio::spawn(async move {
                let patch = format!(r#"{{"display":{{"hidden_windows":{{"acc:{n}":["w"]}}}}}}"#);
                core.update_settings(&patch).await
            })
        })
        .collect();
    let theme = harness
        .core
        .update_settings(r#"{"display":{"theme":"dark"}}"#);
    theme.await.unwrap();
    for task in tasks {
        task.await.unwrap().unwrap();
    }
    let served = harness.core.model().settings.clone();
    assert_eq!(served.display.hidden_windows.len(), 16);
    assert_eq!(served.display.theme, crate::settings::Theme::Dark);
    assert_eq!(stored_settings(&harness), served);
}

#[tokio::test]
async fn patches_reset_with_null_and_reject_invalid_results() {
    let (harness, _) = two_accounts().await;
    let core = &harness.core;
    core.update_settings(
        r#"{"refresh_interval_secs":120,"display":{"hidden_windows":{"codex:a":["weekly"],"codex:b":["session"]}}}"#,
    )
    .await
    .unwrap();
    core.update_settings(
        r#"{"refresh_interval_secs":null,"display":{"hidden_windows":{"codex:a":null}}}"#,
    )
    .await
    .unwrap();
    let settings = stored_settings(&harness);
    assert_eq!(settings.refresh_interval_secs, 300);
    assert_eq!(
        settings.display.hidden_windows.keys().collect::<Vec<_>>(),
        ["codex:b"]
    );
    for patch in [
        r#"{"display":{"compact":true}}"#,
        r#"{"refresh_interval_secs":1}"#,
        "[]",
    ] {
        assert!(matches!(
            core.update_settings(patch).await,
            Err(CommandError::Settings(_))
        ));
    }
    assert_eq!(stored_settings(&harness), settings);
    assert_eq!(core.model().settings, settings);
}

#[tokio::test]
async fn account_label_order_and_visibility_reach_the_state() {
    let (harness, _) = two_accounts().await;
    let core = &harness.core;
    core.set_account_label("codex:b", "  Home ").await.unwrap();
    core.set_account_order(&strings(&["codex:b"]))
        .await
        .unwrap();
    core.set_account_hidden("codex:a", true).await.unwrap();
    assert_eq!(
        listed(&harness),
        [
            ("codex:b".to_owned(), Some("Home".to_owned()), false),
            ("codex:a".to_owned(), None, true)
        ]
    );
    core.set_account_label("codex:b", "").await.unwrap();
    assert_eq!(listed(&harness)[0].1, None);
    assert!(matches!(
        core.set_account_hidden("codex:zzz", true).await,
        Err(CommandError::UnknownAccount(_))
    ));
}

#[tokio::test]
async fn refresh_results_drive_notifications() {
    let (harness, provider) = two_accounts().await;
    harness
        .core
        .set_account_hidden("codex:b", true)
        .await
        .unwrap();
    let mut scheduler = Scheduler::new(harness.core.clone());
    scheduler.sync(&harness.core.active_accounts(), FirstRefresh::Scheduled);
    eventually(|| settled(&harness, 2)).await;
    *provider.limits.lock().unwrap() = Ok(snapshot(
        vec![session(58.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    ));
    harness.core.refresh("codex:a").unwrap();
    harness.core.refresh("codex:b").unwrap();
    eventually(|| !harness.notifier.texts().is_empty()).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        harness.notifier.texts(),
        [(
            "Codex · ada@example.com — Session".to_owned(),
            "Projected to finish close to the limit · resets in 2h".to_owned()
        )]
    );
}

#[derive(Default)]
struct RecordingSink {
    states: Mutex<Vec<String>>,
}

#[async_trait]
impl SignalSink for RecordingSink {
    async fn state_changed(&self, state: &str) -> zbus::Result<()> {
        self.states.lock().unwrap().push(state.to_owned());
        Ok(())
    }

    async fn open_requested(&self) -> zbus::Result<()> {
        Ok(())
    }
}

#[tokio::test(start_paused = true)]
async fn state_changes_are_debounced_into_one_signal() {
    let (harness, _) = two_accounts().await;
    let sink = Arc::new(RecordingSink::default());
    let dynamic: Arc<dyn SignalSink> = sink.clone();
    let task = tokio::spawn(publish_changes(harness.core.clone(), dynamic));
    tokio::time::sleep(Duration::from_millis(300)).await;
    sink.states.lock().unwrap().clear();
    for _ in 0..5 {
        harness.core.mark_changed();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(sink.states.lock().unwrap().len(), 1);
    harness.core.mark_changed();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let states = sink.states.lock().unwrap().clone();
    assert_eq!(states.len(), 2);
    let parsed: crate::StatePayload = serde_json::from_str(&states[1]).unwrap();
    assert_eq!(parsed.accounts.len(), 2);
    task.abort();
}

#[tokio::test(start_paused = true)]
async fn settings_changes_emit_state_with_display_and_hidden_windows() {
    let (harness, _) = two_accounts().await;
    let sink = Arc::new(RecordingSink::default());
    let dynamic: Arc<dyn SignalSink> = sink.clone();
    for (name, used) in [("a", 90.0), ("b", 30.0)] {
        let limits = snapshot(
            vec![session(used, "2026-09-23T12:00:00Z")],
            "2026-09-23T10:00:00Z",
        );
        let id = AccountId(format!("codex:{name}"));
        let now = crate::testing::ts("2026-09-23T10:00:00Z");
        harness.core.model().record_success(&id, limits, now);
    }
    let task = tokio::spawn(publish_changes(harness.core.clone(), dynamic));
    tokio::time::sleep(Duration::from_millis(300)).await;
    sink.states.lock().unwrap().clear();
    harness
        .core
        .set_settings(
            r#"{"display":{"theme":"dark","translucent":true,"hidden_windows":{"codex:a":["session"]}}}"#,
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let states = sink.states.lock().unwrap().clone();
    assert_eq!(states.len(), 1);
    let parsed: crate::StatePayload = serde_json::from_str(&states[0]).unwrap();
    assert_eq!(parsed.display.theme, crate::settings::Theme::Dark);
    assert!(parsed.display.translucent);
    let hidden: Vec<(String, bool)> = parsed
        .accounts
        .iter()
        .flat_map(|a| a.windows.iter().map(|w| (a.id.clone(), w.hidden)))
        .collect();
    assert_eq!(
        hidden,
        [("codex:a".to_owned(), true), ("codex:b".to_owned(), false)]
    );
    assert_eq!(parsed.headline.unwrap().account_id, "codex:b");
    task.abort();
}

fn ids(names: &[&str]) -> Vec<AccountId> {
    names.iter().map(|n| AccountId((*n).to_owned())).collect()
}

fn strings(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

#[test]
fn reorder_puts_requested_first_and_keeps_the_rest() {
    let current = ids(&["a", "b", "c", "d"]);
    let order = reorder(&current, &strings(&["c", "a"])).unwrap();
    assert_eq!(order, ids(&["c", "a", "b", "d"]));
}

#[test]
fn reorder_rejects_unknown_and_duplicate_ids() {
    let current = ids(&["a", "b"]);
    assert!(matches!(
        reorder(&current, &strings(&["x"])),
        Err(CommandError::UnknownAccount(id)) if id == "x"
    ));
    assert!(matches!(
        reorder(&current, &strings(&["a", "a"])),
        Err(CommandError::DuplicateAccount(id)) if id == "a"
    ));
}

#[test]
fn labels_are_trimmed_cleared_and_bounded() {
    assert_eq!(normalize_label("  Work ").unwrap().as_deref(), Some("Work"));
    assert_eq!(normalize_label("   ").unwrap(), None);
    assert!(normalize_label(&"é".repeat(64)).is_ok());
    assert!(matches!(
        normalize_label(&"x".repeat(65)),
        Err(CommandError::LabelTooLong(64))
    ));
}
