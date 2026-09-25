use headroom_core::pace::Tone;
use jiff::Timestamp;

use super::test_fetch::{
    CLAUDE_OUTAGE, CLAUDE_URL, FakeFetch, OPENAI_COMPONENTS_URL, OPENAI_DEGRADED, OPENAI_WIDGET,
    OPENAI_WIDGET_URL, cached, harness_using, modified, set_enabled, start, until_calls,
};
use super::*;
use crate::status::Indicator;
use crate::testing::{CLAUDE, CODEX, eventually};

fn ts(text: &str) -> Timestamp {
    text.parse().unwrap()
}

#[tokio::test(start_paused = true)]
async fn nothing_is_requested_while_status_pages_are_off() {
    let harness = harness_using(&[CLAUDE, CODEX]).await;
    let fetch = FakeFetch::new(Vec::new());
    let task = start(&harness, fetch.clone());
    tokio::time::sleep(Duration::from_hours(3)).await;
    assert_eq!(fetch.count(), 0);
    assert_eq!(harness.core.state().provider_status, []);
    task.abort();
}

#[tokio::test(start_paused = true)]
async fn only_providers_in_use_are_polled_every_five_minutes_with_the_etag() {
    let harness = harness_using(&[CLAUDE]).await;
    set_enabled(&harness, true);
    let fetch = FakeFetch::new(vec![(
        CLAUDE_URL,
        vec![
            Ok(modified(CLAUDE_OUTAGE, Some("W/\"e1\""))),
            Ok(Fetched::NotModified),
        ],
    )]);
    let begin = Instant::now();
    let task = start(&harness, fetch.clone());
    until_calls(&fetch, 1).await;
    harness.clock.advance(SignedDuration::from_mins(5));
    until_calls(&fetch, 2).await;
    assert_eq!(fetch.gaps_from(begin), [15, 300]);
    assert_eq!(fetch.urls(), [CLAUDE_URL, CLAUDE_URL]);
    assert_eq!(fetch.etags(), [None, Some("W/\"e1\"".into())]);
    let statuses = harness.core.state().provider_status;
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].provider, CLAUDE);
    assert_eq!(statuses[0].indicator, Indicator::Major);
    assert_eq!(statuses[0].tone, Tone::Critical);
    let page = &cached(&harness)[0];
    assert_eq!(page.fetched_at, ts("2026-09-23T10:05:00Z"));
    assert_eq!(page.etag.as_deref(), Some("W/\"e1\""));
    assert_eq!(page.body, CLAUDE_OUTAGE);
    task.abort();
}

#[tokio::test(start_paused = true)]
async fn incident_io_pages_read_components_and_the_widget() {
    let harness = harness_using(&[CODEX]).await;
    set_enabled(&harness, true);
    let fetch = FakeFetch::new(vec![
        (
            OPENAI_COMPONENTS_URL,
            vec![Ok(modified(OPENAI_DEGRADED, None))],
        ),
        (OPENAI_WIDGET_URL, vec![Ok(modified(OPENAI_WIDGET, None))]),
    ]);
    let task = start(&harness, fetch.clone());
    until_calls(&fetch, 2).await;
    assert_eq!(fetch.urls(), [OPENAI_COMPONENTS_URL, OPENAI_WIDGET_URL]);
    eventually(|| !harness.core.state().provider_status.is_empty()).await;
    let status = &harness.core.state().provider_status[0];
    assert_eq!(status.provider, CODEX);
    assert_eq!(status.title.as_deref(), Some("Increased Codex CLI errors"));
    task.abort();
}

#[tokio::test(start_paused = true)]
async fn failures_back_off_and_rate_limits_hold_the_page() {
    let harness = harness_using(&[CLAUDE]).await;
    set_enabled(&harness, true);
    let fetch = FakeFetch::new(vec![(
        CLAUDE_URL,
        vec![
            Err(FetchError::Failed("offline".into())),
            Err(FetchError::Failed("offline".into())),
            Err(FetchError::RateLimited {
                retry_after: Some(SignedDuration::from_mins(20)),
            }),
            Ok(modified(CLAUDE_OUTAGE, None)),
        ],
    )]);
    let begin = Instant::now();
    let task = start(&harness, fetch.clone());
    until_calls(&fetch, 4).await;
    assert_eq!(fetch.gaps_from(begin), [15, 300, 600, 1200]);
    eventually(|| !harness.core.state().provider_status.is_empty()).await;
    task.abort();
}

#[tokio::test(start_paused = true)]
async fn a_cached_status_is_shown_at_once_and_delays_the_first_round() {
    let harness = harness_using(&[CLAUDE]).await;
    set_enabled(&harness, true);
    let page = CachedPage {
        url: CLAUDE_URL.into(),
        fetched_at: ts("2026-09-23T09:58:00Z"),
        etag: Some("W/\"e0\"".into()),
        body: CLAUDE_OUTAGE.into(),
    };
    harness
        .storage
        .blocking(|conn| status_cache::save(conn, &page))
        .unwrap();
    let fetch = FakeFetch::new(vec![(CLAUDE_URL, vec![Ok(Fetched::NotModified)])]);
    let begin = Instant::now();
    let task = start(&harness, fetch.clone());
    eventually(|| !harness.core.state().provider_status.is_empty()).await;
    assert_eq!(fetch.count(), 0);
    until_calls(&fetch, 1).await;
    assert_eq!(fetch.gaps_from(begin), [180]);
    assert_eq!(fetch.etags(), [Some("W/\"e0\"".into())]);
    task.abort();
}

#[tokio::test(start_paused = true)]
async fn turning_status_pages_on_polls_at_once() {
    let harness = harness_using(&[CLAUDE]).await;
    let fetch = FakeFetch::new(vec![(CLAUDE_URL, vec![Ok(modified(CLAUDE_OUTAGE, None))])]);
    let task = start(&harness, fetch.clone());
    tokio::time::sleep(Duration::from_hours(1)).await;
    assert_eq!(fetch.count(), 0);
    let enabled_at = Instant::now();
    set_enabled(&harness, true);
    until_calls(&fetch, 1).await;
    assert_eq!(fetch.gaps_from(enabled_at), [0]);
    task.abort();
}
