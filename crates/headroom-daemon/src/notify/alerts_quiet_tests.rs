use headroom_core::account::ProviderId;
use headroom_core::forecast::Liveness;
use jiff::SignedDuration;
use tokio::sync::mpsc;

use super::*;
use crate::ipc::{Hub, Topic, Topics};
use crate::settings::{ClockTime, QuietHours};
use crate::testing::{CODEX, RecordingNotifier, account, session, snapshot, ts, weekly};

const NIGHT: &str = "2026-09-23T23:00:00Z";
const LATER: &str = "2026-09-24T02:00:00Z";
const MORNING: &str = "2026-09-24T08:00:00Z";
const SESSION_RESET: &str = "2026-09-24T01:00:00Z";
const WEEK_RESET: &str = "2026-09-26T00:00:00Z";
const IDLE: Signal = Signal {
    liveness: Liveness::Idle,
    poll_interval: SignedDuration::from_mins(5),
};

fn record(provider: ProviderId, name: &str) -> AccountRecord {
    AccountRecord {
        reference: account(provider, name),
        label: Some(name.to_owned()),
        hidden: false,
        sort_order: 0,
        email: None,
        plan: None,
        last_seen: ts(NIGHT),
        gone: false,
    }
}

fn work() -> AccountRecord {
    AccountRecord {
        label: Some("Work".into()),
        ..record(CODEX, "work")
    }
}

fn clock(text: &str) -> ClockTime {
    ClockTime::try_from(text.to_owned()).unwrap()
}

fn quiet(allow_critical: bool) -> NotificationSettings {
    NotificationSettings {
        quiet_hours: QuietHours {
            enabled: true,
            from: clock("22:00"),
            to: clock("08:00"),
            allow_critical,
        },
        ..NotificationSettings::default()
    }
}

fn almost_out_only() -> NotificationSettings {
    NotificationSettings {
        will_run_out: false,
        ..quiet(true)
    }
}

fn load(storage: &Storage, notifier: Arc<dyn Notifier>) -> Alerts {
    storage
        .blocking(|conn| Alerts::load(conn, storage.clone(), notifier))
        .unwrap()
}

struct Scene {
    alerts: Alerts,
    notifier: Arc<RecordingNotifier>,
    settings: NotificationSettings,
    locale: Locale,
}

impl Scene {
    fn new(settings: NotificationSettings) -> Scene {
        let notifier = Arc::new(RecordingNotifier::default());
        let alerts = load(&Storage::open_in_memory().unwrap(), notifier.clone());
        Scene {
            alerts,
            notifier,
            settings,
            locale: Locale::En,
        }
    }

    async fn observe(&self, account: &AccountRecord, windows: Vec<QuotaWindow>, at: &str) {
        observe(
            &self.alerts,
            account,
            windows,
            &self.settings,
            self.locale,
            at,
        )
        .await;
    }

    async fn climb(&self, account: &AccountRecord) {
        self.observe(account, vec![weekly(10.0, WEEK_RESET)], NIGHT)
            .await;
        self.observe(account, vec![weekly(95.0, WEEK_RESET)], NIGHT)
            .await;
    }

    async fn spend(&self, account: &AccountRecord) {
        self.observe(account, vec![weekly(10.0, WEEK_RESET)], NIGHT)
            .await;
        self.observe(account, vec![weekly(100.0, WEEK_RESET)], NIGHT)
            .await;
    }

    async fn release_at(&self, at: &str) {
        release(&self.alerts, &self.settings, self.locale, at).await;
    }

    fn sent(&self) -> Vec<Notification> {
        self.notifier.sent.lock().unwrap().clone()
    }
}

async fn observe(
    alerts: &Alerts,
    account: &AccountRecord,
    windows: Vec<QuotaWindow>,
    settings: &NotificationSettings,
    locale: Locale,
    at: &str,
) {
    let limits = snapshot(windows, at);
    let review = Review {
        account,
        provider_name: "Codex",
        snapshot: &limits,
        history: None,
        signal: IDLE,
        spend: &[],
        settings: settings.clone(),
        display: &DisplaySettings::default(),
        locale,
        now: ts(at),
        tz: &TimeZone::UTC,
    };
    alerts.review(&review).await.unwrap();
}

async fn release(alerts: &Alerts, settings: &NotificationSettings, locale: Locale, at: &str) {
    let release = Release {
        settings,
        locale,
        now: ts(at),
        tz: &TimeZone::UTC,
    };
    alerts.release(&release).await.unwrap();
}

#[tokio::test]
async fn alerts_in_quiet_hours_are_held_and_summarized_when_they_end() {
    let scene = Scene::new(quiet(true));
    scene.climb(&work()).await;
    assert!(scene.sent().is_empty());
    assert_eq!(scene.alerts.held_ids().await.len(), 2);
    scene.release_at(LATER).await;
    assert!(scene.sent().is_empty());
    scene.release_at(MORNING).await;
    scene.release_at(MORNING).await;
    let sent = scene.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].id, format!("summary/{}", ts(MORNING).as_second()));
    assert_eq!(sent[0].account_id, "");
    assert_eq!(sent[0].title, "Headroom — while you were away");
    assert_eq!(
        sent[0].body,
        "Codex · Work · Weekly — under 10% left\nCodex · Work · Weekly — projected to run out"
    );
    assert_eq!(sent[0].urgency, Urgency::Normal);
    assert!(scene.alerts.held_ids().await.is_empty());
}

#[tokio::test]
async fn a_single_held_alert_is_sent_as_the_original_with_a_fresh_countdown() {
    let scene = Scene::new(almost_out_only());
    scene.climb(&work()).await;
    scene.release_at(MORNING).await;
    let sent = scene.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].id, "codex:work/weekly/almost_out");
    assert_eq!(sent[0].account_id, "codex:work");
    assert_eq!(sent[0].title, "Codex · Work — Weekly");
    assert_eq!(sent[0].body, "Under 10% left · resets in 1d 16h");
}

#[tokio::test]
async fn more_than_four_items_end_with_a_count_in_both_languages() {
    for (locale, title, last) in [
        (Locale::En, "Headroom — while you were away", "+2 more"),
        (Locale::Ru, "Headroom — пока вас не было", "и ещё 2"),
    ] {
        let mut scene = Scene::new(quiet(true));
        scene.locale = locale;
        for name in ["A", "B", "C"] {
            scene.climb(&record(CODEX, name)).await;
        }
        scene.release_at(MORNING).await;
        let sent = scene.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].title, title);
        let lines: Vec<&str> = sent[0].body.lines().collect();
        assert_eq!(lines.len(), 5, "{lines:?}");
        assert_eq!(lines[4], last);
    }
    let mut russian = Scene::new(almost_out_only());
    russian.locale = Locale::Ru;
    russian.climb(&record(CODEX, "A")).await;
    russian.climb(&record(CODEX, "B")).await;
    russian.release_at(MORNING).await;
    assert_eq!(
        russian.sent()[0].body,
        "Codex · A · Неделя — осталось меньше 10%\nCodex · B · Неделя — осталось меньше 10%"
    );
}

#[tokio::test]
async fn reset_windows_and_cleared_conditions_are_dropped() {
    let scene = Scene::new(almost_out_only());
    let windows = |used| vec![session(used, SESSION_RESET), weekly(used, WEEK_RESET)];
    scene.observe(&work(), windows(10.0), NIGHT).await;
    scene.observe(&work(), windows(95.0), NIGHT).await;
    let home = record(CODEX, "Home");
    scene.climb(&home).await;
    scene
        .observe(&home, vec![weekly(20.0, WEEK_RESET)], LATER)
        .await;
    assert_eq!(scene.alerts.held_ids().await.len(), 3);
    scene.release_at(MORNING).await;
    let sent = scene.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].id, "codex:work/weekly/almost_out");
    assert!(scene.alerts.held_ids().await.is_empty());
}

#[tokio::test]
async fn nothing_is_sent_when_every_held_item_is_stale() {
    let scene = Scene::new(quiet(true));
    scene.climb(&work()).await;
    scene
        .observe(&work(), vec![weekly(20.0, WEEK_RESET)], LATER)
        .await;
    scene.release_at(MORNING).await;
    assert!(scene.sent().is_empty());
    assert!(scene.alerts.held_ids().await.is_empty());
}

#[tokio::test]
async fn critical_alerts_pass_quiet_hours_only_when_allowed() {
    let allowed = Scene::new(quiet(true));
    allowed.spend(&work()).await;
    let sent = allowed.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].urgency, Urgency::Critical);
    assert!(sent[0].body.starts_with("Limit reached"));
    assert_eq!(
        allowed.alerts.held_ids().await,
        ["codex:work/weekly/almost_out"]
    );
    let strict = Scene::new(quiet(false));
    strict.spend(&work()).await;
    assert!(strict.sent().is_empty());
    strict.release_at(MORNING).await;
    let summary = &strict.sent()[0];
    assert_eq!(summary.urgency, Urgency::Critical);
    assert_eq!(
        summary.body,
        "Codex · Work · Weekly — under 10% left\nCodex · Work · Weekly — limit reached"
    );
}

#[tokio::test]
async fn held_alerts_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("headroom.db");
    let notifier = Arc::new(RecordingNotifier::default());
    let settings = quiet(true);
    let first = Storage::open(&path).unwrap();
    let alerts = load(&first, notifier.clone());
    for used in [10.0, 95.0] {
        let windows = vec![weekly(used, WEEK_RESET)];
        observe(&alerts, &work(), windows, &settings, Locale::En, NIGHT).await;
    }
    drop(alerts);
    drop(first);
    let second = Storage::open(&path).unwrap();
    let restarted = load(&second, notifier.clone());
    assert_eq!(restarted.held_ids().await.len(), 2);
    release(&restarted, &settings, Locale::En, MORNING).await;
    assert_eq!(notifier.texts().len(), 1);
    drop(restarted);
    let third = load(&second, notifier.clone());
    assert!(third.held_ids().await.is_empty());
}

#[tokio::test]
async fn turning_quiet_hours_off_releases_held_alerts_at_once() {
    let scene = Scene::new(quiet(true));
    scene.climb(&work()).await;
    let off = NotificationSettings::default();
    release(&scene.alerts, &off, Locale::En, NIGHT).await;
    assert_eq!(scene.sent().len(), 1);
    assert!(scene.sent()[0].id.starts_with("summary/"));
}

#[tokio::test]
async fn a_failed_release_is_retried() {
    let scene = Scene::new(quiet(true));
    scene.climb(&work()).await;
    scene.notifier.fail(true);
    scene.release_at(MORNING).await;
    assert_eq!(scene.alerts.held_ids().await.len(), 2);
    scene.notifier.fail(false);
    scene.release_at(MORNING).await;
    assert_eq!(scene.sent().len(), 1);
    assert!(scene.alerts.held_ids().await.is_empty());
}

async fn lapse(scene: &Scene, account: &AccountRecord) {
    let review = LapseReview {
        account,
        provider_name: "Codex",
        settings: &scene.settings,
        locale: Locale::En,
        now: ts(NIGHT),
        tz: &TimeZone::UTC,
    };
    scene.alerts.review_lapse(&review).await.unwrap();
}

#[tokio::test]
async fn lapses_are_held_and_dropped_after_renewal() {
    let scene = Scene::new(quiet(true));
    lapse(&scene, &work()).await;
    lapse(&scene, &work()).await;
    assert!(scene.sent().is_empty());
    assert_eq!(
        scene.alerts.held_ids().await,
        ["codex:work/subscription_inactive"]
    );
    scene.release_at(MORNING).await;
    assert_eq!(
        scene.notifier.texts(),
        [(
            "Codex · Work — subscription inactive".to_owned(),
            "Limits are unavailable until the plan is renewed.".to_owned()
        )]
    );
    let renewed = Scene::new(quiet(true));
    lapse(&renewed, &work()).await;
    renewed.alerts.renewed(work().id());
    renewed.release_at(MORNING).await;
    assert!(renewed.sent().is_empty());
}

#[tokio::test]
async fn summaries_reach_macos_as_an_alert_line() {
    let hub = Arc::new(Hub::default());
    let (outbox, mut inbox) = mpsc::channel(8);
    let _subscription = hub.subscribe(outbox, Topics::from_list(&[Topic::Alerts]));
    let notifier: Arc<dyn Notifier> = hub.clone();
    let alerts = load(&Storage::open_in_memory().unwrap(), notifier);
    let settings = quiet(true);
    for used in [10.0, 95.0] {
        let windows = vec![weekly(used, WEEK_RESET)];
        observe(&alerts, &work(), windows, &settings, Locale::En, NIGHT).await;
    }
    release(&alerts, &settings, Locale::En, MORNING).await;
    let line = inbox.recv().await.unwrap();
    let message: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(message["method"], "Alert");
    let params = &message["params"];
    assert_eq!(params["id"], format!("summary/{}", ts(MORNING).as_second()));
    assert_eq!(params["title"], "Headroom — while you were away");
    assert_eq!(params["account_id"], "");
    assert_eq!(params["urgency"], "normal");
    assert!(inbox.try_recv().is_err());
}
