use headroom_core::account::ProviderKind;

use super::*;
use crate::testing::{RecordingNotifier, account, session, snapshot, ts, weekly};

const NOW: &str = "2026-09-23T10:00:00Z";
const RESET: &str = "2026-09-23T12:00:00Z";

fn work() -> AccountRecord {
    AccountRecord {
        reference: account(ProviderKind::Codex, "work"),
        label: Some("Work".into()),
        hidden: false,
        sort_order: 0,
        email: None,
        plan: None,
        last_seen: ts(NOW),
        gone: false,
    }
}

fn load(storage: &Storage, notifier: &Arc<RecordingNotifier>) -> Alerts {
    let notifier: Arc<dyn Notifier> = notifier.clone();
    storage
        .blocking(|conn| Alerts::load(conn, storage.clone(), notifier))
        .unwrap()
}

async fn observe(alerts: &Alerts, account: &AccountRecord, used: f64) {
    observe_with(alerts, account, used, NotificationSettings::default()).await;
}

async fn observe_with(
    alerts: &Alerts,
    account: &AccountRecord,
    used: f64,
    settings: NotificationSettings,
) {
    let limits = snapshot(vec![session(used, RESET)], NOW);
    review(
        alerts,
        account,
        &limits,
        settings,
        &DisplaySettings::default(),
    )
    .await;
}

async fn review(
    alerts: &Alerts,
    account: &AccountRecord,
    limits: &LimitsSnapshot,
    settings: NotificationSettings,
    display: &DisplaySettings,
) {
    let review = Review {
        account,
        snapshot: limits,
        settings,
        display,
        locale: Locale::En,
        now: ts(NOW),
    };
    alerts.review(&review).await.unwrap();
}

#[tokio::test]
async fn delivers_rising_edges_after_priming() {
    let storage = Storage::open_in_memory().unwrap();
    let notifier = Arc::new(RecordingNotifier::default());
    let alerts = load(&storage, &notifier);
    observe(&alerts, &work(), 10.0).await;
    assert!(notifier.texts().is_empty());
    observe(&alerts, &work(), 58.0).await;
    observe(&alerts, &work(), 59.0).await;
    assert_eq!(
        notifier.texts(),
        [(
            "Codex · Work — Session".to_owned(),
            "Projected to finish close to the limit · resets in 2h".to_owned()
        )]
    );
}

#[tokio::test]
async fn disabled_milestones_advance_silently() {
    let storage = Storage::open_in_memory().unwrap();
    let notifier = Arc::new(RecordingNotifier::default());
    let alerts = load(&storage, &notifier);
    let quiet = NotificationSettings {
        cutting_it_close: false,
        ..NotificationSettings::default()
    };
    observe_with(&alerts, &work(), 10.0, quiet).await;
    observe_with(&alerts, &work(), 58.0, quiet).await;
    observe(&alerts, &work(), 58.0).await;
    assert!(notifier.texts().is_empty());
}

#[tokio::test]
async fn failed_delivery_is_rolled_back_and_retried() {
    let storage = Storage::open_in_memory().unwrap();
    let notifier = Arc::new(RecordingNotifier::default());
    let alerts = load(&storage, &notifier);
    observe(&alerts, &work(), 10.0).await;
    notifier.fail(true);
    observe(&alerts, &work(), 58.0).await;
    assert!(notifier.texts().is_empty());
    notifier.fail(false);
    observe(&alerts, &work(), 58.0).await;
    assert_eq!(notifier.texts().len(), 1);
    observe(&alerts, &work(), 58.0).await;
    assert_eq!(notifier.texts().len(), 1);
}

#[tokio::test]
async fn state_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("headroom.db");
    let notifier = Arc::new(RecordingNotifier::default());
    let first = Storage::open(&path).unwrap();
    observe(&load(&first, &notifier), &work(), 95.0).await;
    drop(first);
    let second = Storage::open(&path).unwrap();
    observe(&load(&second, &notifier), &work(), 95.0).await;
    assert!(notifier.texts().is_empty());
    drop(second);
    let third = Storage::open(&path).unwrap();
    let alerts = load(&third, &notifier);
    observe(&alerts, &work(), 20.0).await;
    observe(&alerts, &work(), 95.0).await;
    let bodies: Vec<String> = notifier.texts().into_iter().map(|(_, body)| body).collect();
    assert_eq!(bodies.len(), 2);
    assert!(bodies[0].starts_with("Projected to run out"));
    assert!(bodies[1].starts_with("Under 10% left"));
}

#[tokio::test]
async fn hidden_accounts_are_not_reviewed() {
    let storage = Storage::open_in_memory().unwrap();
    let notifier = Arc::new(RecordingNotifier::default());
    let alerts = load(&storage, &notifier);
    let mut hidden = work();
    hidden.hidden = true;
    observe(&alerts, &hidden, 10.0).await;
    observe(&alerts, &hidden, 95.0).await;
    assert!(notifier.texts().is_empty());
    let stored: Vec<(AccountId, String, AlertState)> =
        storage.blocking(|conn| alerts::load_all(conn)).unwrap();
    assert!(stored.is_empty());
}

#[tokio::test]
async fn hidden_windows_are_not_reviewed() {
    let storage = Storage::open_in_memory().unwrap();
    let notifier = Arc::new(RecordingNotifier::default());
    let alerts = load(&storage, &notifier);
    let mut display = DisplaySettings::default();
    display
        .hidden_windows
        .insert("codex:work".into(), vec!["session".into()]);
    let settings = NotificationSettings::default();
    for used in [10.0, 95.0] {
        let limits = snapshot(
            vec![session(used, RESET), weekly(used, "2026-09-25T10:00:00Z")],
            NOW,
        );
        review(&alerts, &work(), &limits, settings, &display).await;
    }
    let titles: Vec<String> = notifier
        .texts()
        .into_iter()
        .map(|(title, _)| title)
        .collect();
    assert_eq!(titles, ["Codex · Work — Weekly", "Codex · Work — Weekly"]);
    let stored: Vec<(AccountId, String, AlertState)> =
        storage.blocking(|conn| alerts::load_all(conn)).unwrap();
    let windows: Vec<&str> = stored.iter().map(|(_, w, _)| w.as_str()).collect();
    assert_eq!(windows, ["weekly"]);
}

#[tokio::test]
async fn russian_locale_is_used_for_delivery() {
    let storage = Storage::open_in_memory().unwrap();
    let notifier = Arc::new(RecordingNotifier::default());
    let alerts = load(&storage, &notifier);
    let display = DisplaySettings::default();
    for used in [10.0, 95.0] {
        let limits = snapshot(vec![session(used, RESET)], NOW);
        let review = Review {
            account: &work(),
            snapshot: &limits,
            settings: NotificationSettings::default(),
            display: &display,
            locale: Locale::Ru,
            now: ts(NOW),
        };
        alerts.review(&review).await.unwrap();
    }
    let titles: Vec<String> = notifier
        .texts()
        .into_iter()
        .map(|(title, _)| title)
        .collect();
    assert!(
        titles.iter().all(|t| t == "Codex · Work — Сессия"),
        "{titles:?}"
    );
}

async fn lapse(alerts: &Alerts, storage: &Storage, account: &AccountRecord) {
    let id = account.id().clone();
    storage
        .run(move |conn| crate::storage::lapses::record(conn, &id, "none"))
        .await
        .unwrap();
    alerts.review_lapse(account, Locale::En).await.unwrap();
}

#[tokio::test]
async fn lapse_is_announced_once_even_across_restarts() {
    let storage = Storage::open_in_memory().unwrap();
    let notifier = Arc::new(RecordingNotifier::default());
    let alerts = load(&storage, &notifier);
    lapse(&alerts, &storage, &work()).await;
    lapse(&alerts, &storage, &work()).await;
    let reloaded = load(&storage, &notifier);
    lapse(&reloaded, &storage, &work()).await;
    assert_eq!(
        notifier.texts(),
        [(
            "Codex · Work — subscription inactive".to_owned(),
            "Limits are unavailable until the plan is renewed.".to_owned()
        )]
    );
    reloaded.renewed(work().id());
    lapse(&reloaded, &storage, &work()).await;
    assert_eq!(notifier.texts().len(), 2);
}

#[tokio::test]
async fn undelivered_lapse_is_retried_and_hidden_accounts_stay_quiet() {
    let storage = Storage::open_in_memory().unwrap();
    let notifier = Arc::new(RecordingNotifier::default());
    let alerts = load(&storage, &notifier);
    let mut hidden = work();
    hidden.hidden = true;
    lapse(&alerts, &storage, &hidden).await;
    notifier.fail(true);
    lapse(&alerts, &storage, &work()).await;
    assert!(notifier.texts().is_empty());
    notifier.fail(false);
    lapse(&alerts, &storage, &work()).await;
    assert_eq!(notifier.texts().len(), 1);
}
