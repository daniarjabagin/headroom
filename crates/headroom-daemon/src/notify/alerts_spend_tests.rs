use headroom_core::forecast::Liveness;
use headroom_core::history::UsageSample;
use headroom_core::units::{MicroUsd, Percent};
use jiff::SignedDuration;

use super::*;
use crate::testing::{CODEX, RecordingNotifier, account, session, snapshot, ts};

const NOW: &str = "2026-09-23T10:00:00Z";
const RESET: &str = "2026-09-23T12:00:00Z";
const LIVE: Signal = Signal {
    liveness: Liveness::Live,
    poll_interval: SignedDuration::from_mins(5),
};

fn work() -> AccountRecord {
    AccountRecord {
        reference: account(CODEX, "work"),
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

fn mins_ago(mins: i64) -> Timestamp {
    ts(NOW) - SignedDuration::from_mins(mins)
}

fn steps(used: &[f64], every: i64, last: i64) -> WindowSamples {
    let mut samples: Vec<UsageSample> = used
        .iter()
        .rev()
        .zip(0..)
        .map(|(used, step)| UsageSample {
            at: mins_ago(last + step * every),
            used: Percent::new(*used),
        })
        .collect();
    samples.reverse();
    WindowSamples::from([("session".to_owned(), samples)])
}

fn spent(at: Timestamp, micros: i64) -> SpendPoint {
    SpendPoint {
        at,
        cost: Some(MicroUsd(micros)),
    }
}

struct Seen<'a> {
    used: f64,
    data_time: &'a str,
    history: &'a WindowSamples,
    spend: &'a [SpendPoint],
}

async fn review_live(alerts: &Alerts, seen: &Seen<'_>) {
    let limits = snapshot(vec![session(seen.used, RESET)], seen.data_time);
    let review = Review {
        account: &work(),
        provider_name: "Codex",
        snapshot: &limits,
        history: Some(seen.history),
        signal: LIVE,
        spend: seen.spend,
        settings: NotificationSettings::default(),
        display: &DisplaySettings::default(),
        locale: Locale::En,
        now: ts(NOW),
        tz: &TimeZone::UTC,
    };
    alerts.review(&review).await.unwrap();
}

async fn primed(notifier: &Arc<RecordingNotifier>) -> Alerts {
    let alerts = load(&Storage::open_in_memory().unwrap(), notifier);
    let calm = steps(&[10.0], 60, 0);
    let seen = Seen {
        used: 10.0,
        data_time: NOW,
        history: &calm,
        spend: &[],
    };
    review_live(&alerts, &seen).await;
    alerts
}

#[tokio::test]
async fn a_pace_alert_waits_while_the_spend_forecast_is_calm() {
    let notifier = Arc::new(RecordingNotifier::default());
    let alerts = primed(&notifier).await;
    let history = steps(&[53.0, 54.0, 55.0, 56.0, 57.0, 58.0], 2, 0);
    let slowed: Vec<SpendPoint> = [9, 7, 5, 3, 1]
        .into_iter()
        .map(|mins| spent(mins_ago(mins), 100_000))
        .collect();
    let mut seen = Seen {
        used: 58.0,
        data_time: NOW,
        history: &history,
        spend: &slowed,
    };
    review_live(&alerts, &seen).await;
    assert!(notifier.texts().is_empty());
    seen.spend = &[];
    review_live(&alerts, &seen).await;
    let sent = notifier.sent.lock().unwrap().clone();
    let ids: Vec<&str> = sent.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["codex:work/session/will_run_out"]);
}

#[tokio::test]
async fn spend_alone_never_raises_a_pace_alert() {
    let notifier = Arc::new(RecordingNotifier::default());
    let alerts = primed(&notifier).await;
    let history = steps(&[17.0, 18.0, 19.0, 20.0], 10, 10);
    let mut burst: Vec<SpendPoint> = [35, 25, 15]
        .into_iter()
        .map(|mins| spent(mins_ago(mins), 100_000))
        .collect();
    burst.push(spent(ts(NOW) - SignedDuration::from_secs(30), 5_000_000));
    let seen = Seen {
        used: 20.0,
        data_time: "2026-09-23T09:59:00Z",
        history: &history,
        spend: &burst,
    };
    review_live(&alerts, &seen).await;
    assert!(notifier.texts().is_empty());
    let limits = snapshot(vec![session(seen.used, RESET)], seen.data_time);
    let activity = Activity {
        samples: window_samples(Some(&history), "session"),
        signal: LIVE,
        observed_at: observed_at(&limits),
    };
    let popup = forecast_with_spend(&limits.windows[0], activity, &burst, ts(NOW));
    assert_eq!(popup.severity, Severity::RunningOut);
}
