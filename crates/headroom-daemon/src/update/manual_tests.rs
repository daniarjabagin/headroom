use super::test_feed::{FakeFeed, NOW, modified, release, start, until_calls};
use super::*;
use crate::error::CommandError;
use crate::rescan;
use crate::service::Service;
use crate::testing::{eventually, harness};
use crate::update::CheckStatus;

#[tokio::test(start_paused = true)]
async fn a_manual_check_runs_at_once_and_moves_the_daily_one() {
    let harness = harness(Vec::new()).await;
    let feed = FakeFeed::new(vec![Ok(modified("\"e1\"")), Ok(FeedResponse::NotModified)]);
    let begin = Instant::now();
    let running = start(&harness, feed.clone(), "0.4.0");
    let outcome = running.checks.check().await.unwrap();
    assert_eq!(outcome.status, CheckStatus::Available);
    assert_eq!(outcome.version.as_deref(), Some("0.5.0"));
    assert_eq!(outcome.checked_at, Some(NOW.parse().unwrap()));
    assert_eq!(outcome.until, None);
    let check = harness.core.state().update_check.unwrap();
    assert_eq!(check.checked_at, Some(NOW.parse().unwrap()));
    until_calls(&feed, 2).await;
    assert_eq!(feed.gaps_from(begin), [0, 24 * 3600]);
    running.abort();
}

#[tokio::test(start_paused = true)]
async fn a_second_manual_check_within_a_minute_reuses_the_result() {
    let harness = harness(Vec::new()).await;
    let feed = FakeFeed::new(vec![Ok(modified("\"e1\"")), Ok(FeedResponse::NotModified)]);
    let running = start(&harness, feed.clone(), "0.5.0");
    let first = running.checks.check().await.unwrap();
    assert_eq!(first.status, CheckStatus::UpToDate);
    harness.clock.advance(SignedDuration::from_secs(59));
    assert_eq!(running.checks.check().await.unwrap(), first);
    assert_eq!(feed.calls(), 1);
    harness.clock.advance(SignedDuration::from_secs(1));
    let again = running.checks.check().await.unwrap();
    assert_eq!(feed.calls(), 2);
    assert_eq!(feed.etags(), [None, Some("\"e1\"".into())]);
    assert_eq!(again.status, CheckStatus::UpToDate);
    assert_eq!(
        again.checked_at,
        Some("2026-09-23T10:01:00Z".parse().unwrap())
    );
    running.abort();
}

#[tokio::test(start_paused = true)]
async fn a_rate_limit_hold_is_honoured_by_manual_checks() {
    let harness = harness(Vec::new()).await;
    let retry_after = Some(SignedDuration::from_mins(90));
    let feed = FakeFeed::new(vec![
        Err(FeedError::RateLimited { retry_after }),
        Ok(modified("\"e1\"")),
    ]);
    let running = start(&harness, feed.clone(), "0.4.0");
    let limited = running.checks.check().await.unwrap();
    let until: Timestamp = "2026-09-23T11:30:00Z".parse().unwrap();
    assert_eq!(limited.status, CheckStatus::RateLimited);
    assert_eq!(limited.until, Some(until));
    assert_eq!(limited.checked_at, None);
    harness.clock.advance(SignedDuration::from_mins(89));
    let held = running.checks.check().await.unwrap();
    assert_eq!(held.status, CheckStatus::RateLimited);
    assert_eq!(held.until, Some(until));
    assert_eq!(feed.calls(), 1);
    harness.clock.advance(SignedDuration::from_mins(1));
    let checked = running.checks.check().await.unwrap();
    assert_eq!(checked.status, CheckStatus::Available);
    assert_eq!(feed.calls(), 2);
    running.abort();
}

#[tokio::test(start_paused = true)]
async fn a_failed_manual_check_reports_the_last_successful_one() {
    let harness = harness(Vec::new()).await;
    let feed = FakeFeed::new(vec![
        Ok(modified("\"e1\"")),
        Err(FeedError::Failed("offline".into())),
    ]);
    let running = start(&harness, feed.clone(), "0.4.0");
    running.checks.check().await.unwrap();
    harness.clock.advance(SignedDuration::from_mins(5));
    let failed = running.checks.check().await.unwrap();
    assert_eq!(failed.status, CheckStatus::Failed);
    assert_eq!(failed.checked_at, Some(NOW.parse().unwrap()));
    assert_eq!(failed.version.as_deref(), Some("0.5.0"));
    running.abort();
}

#[tokio::test(start_paused = true)]
async fn manual_checks_while_checks_are_off_send_no_request() {
    let harness = harness(Vec::new()).await;
    harness
        .core
        .update_settings(r#"{"updates":{"check":false}}"#)
        .await
        .unwrap();
    let feed = FakeFeed::new(vec![Ok(modified("\"e1\""))]);
    let running = start(&harness, feed.clone(), "0.4.0");
    let outcome = running.checks.check().await.unwrap();
    assert_eq!(outcome.status, CheckStatus::Disabled);
    assert_eq!(outcome.checked_at, None);
    assert_eq!(feed.calls(), 0);
    assert_eq!(harness.core.state().update_check, None);
    running.abort();
}

#[tokio::test(start_paused = true)]
async fn concurrent_manual_checks_share_one_request() {
    let harness = harness(Vec::new()).await;
    let feed = FakeFeed::new(vec![Ok(modified("\"e1\"")), Ok(modified("\"e2\""))]);
    let running = start(&harness, feed.clone(), "0.4.0");
    let (a, b, c) = tokio::join!(
        running.checks.check(),
        running.checks.check(),
        running.checks.check()
    );
    let a = a.unwrap();
    assert_eq!(a.status, CheckStatus::Available);
    assert_eq!(b.unwrap(), a);
    assert_eq!(c.unwrap(), a);
    assert_eq!(feed.calls(), 1);
    running.abort();
}

#[tokio::test(start_paused = true)]
async fn the_state_shows_the_stored_check_time_at_start() {
    let harness = harness(Vec::new()).await;
    let record = CheckRecord {
        checked_at: "2026-09-23T09:00:00Z".parse().unwrap(),
        etag: None,
        latest: Some(release()),
    };
    harness
        .storage
        .blocking(|conn| updates::save(conn, &record))
        .unwrap();
    let running = start(&harness, FakeFeed::new(Vec::new()), "0.5.0");
    eventually(|| harness.core.state().update_check.is_some()).await;
    let check = harness.core.state().update_check.unwrap();
    assert_eq!(check.checked_at, Some(record.checked_at));
    running.abort();
}

#[tokio::test(start_paused = true)]
async fn the_service_asks_the_running_checker_or_refuses_without_one() {
    let harness = harness(Vec::new()).await;
    let (rescans, _requests) = rescan::channel();
    let plain = Service::new(harness.core.clone(), rescans.clone());
    assert!(matches!(
        plain.check_for_updates().await,
        Err(CommandError::UpdateChecksUnavailable)
    ));
    let feed = FakeFeed::new(vec![Ok(modified("\"e1\""))]);
    let running = start(&harness, feed.clone(), "0.4.0");
    let service =
        Service::new(harness.core.clone(), rescans).with_update_checks(running.checks.clone());
    let outcome = service.check_for_updates().await.unwrap();
    assert_eq!(outcome.status, CheckStatus::Available);
    assert_eq!(feed.calls(), 1);
    running.abort();
}
