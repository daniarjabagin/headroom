use headroom_core::forecast::{Liveness, Signal};
use jiff::SignedDuration;
use jiff::tz::TimeZone;

use super::*;
use crate::notify::alerts::Review;
use crate::settings::DisplaySettings;
use crate::storage::accounts::AccountRecord;
use crate::testing::{CODEX, account, eventually, harness, snapshot, ts, weekly};

const IDLE: Signal = Signal {
    liveness: Liveness::Idle,
    poll_interval: SignedDuration::from_mins(5),
};

#[test]
fn waits_until_quiet_hours_end_but_never_longer_than_a_minute() {
    let now = ts("2026-09-24T07:59:30Z");
    let cases = [
        (None, 60),
        (Some("2026-09-24T08:00:00Z"), 30),
        (Some("2026-09-24T09:00:00Z"), 60),
        (Some("2026-09-24T07:59:30Z"), 1),
        (Some("2026-09-24T07:00:00Z"), 1),
    ];
    for (end, secs) in cases {
        let wait = next_wait(end.map(ts), now);
        assert_eq!(wait, Duration::from_secs(secs), "{end:?}");
    }
}

fn work() -> AccountRecord {
    AccountRecord {
        reference: account(CODEX, "work"),
        label: Some("Work".into()),
        hidden: false,
        sort_order: 0,
        email: None,
        plan: None,
        last_seen: ts("2026-09-23T10:00:00Z"),
        gone: false,
    }
}

async fn climb(core: &Core) {
    let settings = core.model().settings.notifications.clone();
    for used in [10.0, 95.0] {
        let limits = snapshot(
            vec![weekly(used, "2026-09-26T00:00:00Z")],
            "2026-09-23T23:00:00Z",
        );
        let review = Review {
            account: &work(),
            provider_name: "Codex",
            snapshot: &limits,
            history: None,
            signal: IDLE,
            spend: &[],
            settings: settings.clone(),
            display: &DisplaySettings::default(),
            locale: Locale::En,
            now: core.clock.now(),
            tz: &TimeZone::UTC,
        };
        core.alerts.review(&review).await.unwrap();
    }
}

#[tokio::test]
async fn turning_quiet_hours_off_releases_held_alerts() {
    let h = harness(Vec::new()).await;
    h.core
        .update_settings(r#"{"notifications":{"quiet_hours":{"enabled":true}}}"#)
        .await
        .unwrap();
    h.clock.advance(SignedDuration::from_hours(13));
    climb(&h.core).await;
    let task = tokio::spawn(run(h.core.clone()));
    tokio::task::yield_now().await;
    assert_eq!(release_due(&h.core).await, Duration::from_secs(60));
    assert!(h.notifier.texts().is_empty());
    h.core
        .update_settings(r#"{"notifications":{"quiet_hours":{"enabled":false}}}"#)
        .await
        .unwrap();
    eventually(|| h.notifier.texts().len() == 1).await;
    assert_eq!(h.notifier.texts()[0].0, "Headroom — while you were away");
    task.abort();
}
