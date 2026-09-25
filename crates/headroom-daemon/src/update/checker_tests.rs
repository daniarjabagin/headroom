use super::test_feed::{FakeFeed, NOW, modified, release, start, stored, until_calls};
use super::*;
use crate::testing::{eventually, harness};
use crate::update::{Install, InstallKind, Packager};

#[test]
fn not_modified_keeps_the_previous_release_and_etag() {
    let now: Timestamp = NOW.parse().unwrap();
    let previous = CheckRecord {
        checked_at: "2026-09-22T10:00:00Z".parse().unwrap(),
        etag: Some("\"e1\"".into()),
        latest: Some(release()),
    };
    let kept = accept(Some(&previous), FeedResponse::NotModified, now).unwrap();
    assert_eq!(
        kept,
        CheckRecord {
            checked_at: now,
            ..previous
        }
    );
    let fresh = accept(None, modified("\"e2\""), now).unwrap();
    assert_eq!(fresh.etag.as_deref(), Some("\"e2\""));
    assert_eq!(fresh.latest, Some(release()));
    let broken = FeedResponse::Modified {
        body: "{}".into(),
        etag: None,
    };
    assert!(accept(None, broken, now).is_err());
}

#[tokio::test(start_paused = true)]
async fn checks_two_minutes_after_start_then_daily_with_the_etag() {
    let harness = harness(Vec::new()).await;
    let feed = FakeFeed::new(vec![Ok(modified("\"e1\"")), Ok(FeedResponse::NotModified)]);
    let begin = Instant::now();
    let task = start(&harness, feed.clone(), "0.4.0");
    until_calls(&feed, 2).await;
    assert_eq!(feed.gaps_from(begin), [120, 24 * 3600]);
    assert_eq!(feed.etags(), [None, Some("\"e1\"".into())]);
    let update = harness.core.state().update.unwrap();
    assert_eq!(update.version, "0.5.0");
    assert_eq!(update.install, InstallKind::SelfInstalled);
    assert_eq!(update.command, "headroom update");
    let record = stored(&harness).unwrap();
    assert_eq!(record.etag.as_deref(), Some("\"e1\""));
    assert_eq!(record.latest, Some(release()));
    task.abort();
}

#[tokio::test(start_paused = true)]
async fn a_recent_stored_check_is_shown_at_once_and_delays_the_next() {
    let harness = harness(Vec::new()).await;
    let record = CheckRecord {
        checked_at: "2026-09-23T09:00:00Z".parse().unwrap(),
        etag: Some("\"e1\"".into()),
        latest: Some(release()),
    };
    harness
        .storage
        .blocking(|conn| updates::save(conn, &record))
        .unwrap();
    let feed = FakeFeed::new(vec![Ok(FeedResponse::NotModified)]);
    let begin = Instant::now();
    let task = start(&harness, feed.clone(), "0.4.0");
    eventually(|| harness.core.state().update.is_some()).await;
    assert_eq!(feed.calls(), 0);
    until_calls(&feed, 1).await;
    assert_eq!(feed.gaps_from(begin), [23 * 3600]);
    assert_eq!(feed.etags(), [Some("\"e1\"".into())]);
    task.abort();
}

#[tokio::test(start_paused = true)]
async fn rate_limits_and_failures_back_off_silently() {
    let harness = harness(Vec::new()).await;
    let retry_after = Some(SignedDuration::from_mins(90));
    let feed = FakeFeed::new(vec![
        Err(FeedError::RateLimited { retry_after }),
        Err(FeedError::Failed("offline".into())),
        Ok(modified("\"e1\"")),
    ]);
    let begin = Instant::now();
    let task = start(&harness, feed.clone(), "0.4.0");
    until_calls(&feed, 3).await;
    assert_eq!(feed.gaps_from(begin), [120, 90 * 60, 3600]);
    assert!(harness.core.state().update.is_some());
    assert_eq!(stored(&harness).unwrap().checked_at, NOW.parse().unwrap());
    task.abort();
}

#[tokio::test(start_paused = true)]
async fn the_running_release_or_a_newer_one_shows_no_update() {
    let harness = harness(Vec::new()).await;
    let feed = FakeFeed::new(vec![Ok(modified("\"e1\""))]);
    let task = start(&harness, feed.clone(), "0.5.0");
    until_calls(&feed, 1).await;
    eventually(|| stored(&harness).is_some()).await;
    assert_eq!(harness.core.state().update, None);
    task.abort();
}

#[tokio::test(start_paused = true)]
async fn turning_checks_off_stops_requests_and_hides_the_update() {
    let harness = harness(Vec::new()).await;
    harness
        .core
        .update_settings(r#"{"updates":{"check":false}}"#)
        .await
        .unwrap();
    let feed = FakeFeed::new(vec![Ok(modified("\"e1\""))]);
    let task = start(&harness, feed.clone(), "0.4.0");
    tokio::time::sleep(Duration::from_hours(72)).await;
    assert_eq!(feed.calls(), 0);
    harness
        .core
        .update_settings(r#"{"updates":{"check":true}}"#)
        .await
        .unwrap();
    until_calls(&feed, 1).await;
    eventually(|| harness.core.state().update.is_some()).await;
    harness
        .core
        .update_settings(r#"{"updates":{"check":false}}"#)
        .await
        .unwrap();
    assert_eq!(harness.core.state().update, None);
    task.abort();
}

#[test]
fn package_installs_publish_their_own_command() {
    let update = AvailableUpdate {
        release: release(),
        install: Install::Package(Packager::Arch),
    };
    assert!(
        update
            .install
            .command(&update.release)
            .contains("pacman -U")
    );
}
