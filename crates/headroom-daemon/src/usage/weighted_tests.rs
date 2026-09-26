use std::sync::Arc;

use headroom_core::calibration::{Observed, calibrate};
use headroom_core::history::UsageSample;
use headroom_core::provider::Provider;
use headroom_core::tokens::TokenCounts;
use headroom_core::units::{MicroUsd, Percent, Tokens};

use super::*;
use crate::home::UsageHome;
use crate::testing::{CODEX, FakeProvider, account, event, harness, session, snapshot, ts};
use crate::usage::ingest;

struct ModelPrices;

const OPUS: [i64; 3] = [15_000_000, 1_500_000, 75_000_000];
const SONNET: [i64; 3] = [3_000_000, 300_000, 15_000_000];

impl PriceBook for ModelPrices {
    fn cost(&self, event: &UsageEvent) -> Option<MicroUsd> {
        let [input, cache_read, output] = match event.model.as_str() {
            "claude-opus-4-5" => OPUS,
            "claude-sonnet-4-5" => SONNET,
            _ => return None,
        };
        let tokens = &event.tokens;
        let per_million = count(tokens.input) * input
            + count(tokens.cache_read) * cache_read
            + count(tokens.output) * output;
        Some(MicroUsd(per_million / 1_000_000))
    }
}

fn count(tokens: Tokens) -> i64 {
    i64::try_from(tokens.0).unwrap()
}

fn used(at: &str, model: &str, input: u64, cache_read: u64, output: u64) -> UsageEvent {
    UsageEvent {
        tokens: TokenCounts {
            input: Tokens(input),
            cache_read: Tokens(cache_read),
            output: Tokens(output),
            ..TokenCounts::default()
        },
        ..event(at, at, model, 0, 0)
    }
}

#[test]
fn spend_weights_tokens_by_their_price() {
    let at = "2026-09-23T09:00:00Z";
    let reported = UsageEvent {
        reported_cost: Some(MicroUsd(42)),
        ..used(at, "gpt-5.5", 10, 0, 10)
    };
    let cases = [
        (used(at, "claude-opus-4-5", 0, 0, 20_000), Some(1_500_000)),
        (
            used(at, "claude-opus-4-5", 2_000, 1_000_000, 1_000),
            Some(1_605_000),
        ),
        (used(at, "claude-sonnet-4-5", 0, 0, 20_000), Some(300_000)),
        (
            used(at, "claude-sonnet-4-5", 0, 99_000, 1_000),
            Some(44_700),
        ),
        (used(at, "gpt-5.5", 10, 0, 10), None),
        (reported, Some(42)),
    ];
    for (event, expected) in cases {
        let points = spend_points([&event], &ModelPrices);
        let costs: Vec<Option<i64>> = points
            .iter()
            .map(|point| point.cost.map(|cost| cost.0))
            .collect();
        assert_eq!(costs, [expected], "{event:?}");
    }
}

fn polls() -> Vec<UsageSample> {
    ["09:00", "09:10", "09:20", "09:30"]
        .iter()
        .zip([40.0, 41.0, 42.0, 43.0])
        .map(|(at, used)| UsageSample {
            at: ts(&format!("2026-09-23T{at}:00Z")),
            used: Percent::new(used),
        })
        .collect()
}

fn opus_work() -> Vec<UsageEvent> {
    ["09:05", "09:15", "09:25"]
        .iter()
        .map(|at| {
            used(
                &format!("2026-09-23T{at}:00Z"),
                "claude-opus-4-5",
                0,
                0,
                20_000,
            )
        })
        .collect()
}

fn estimate_after(burst: UsageEvent) -> f64 {
    let mut events = opus_work();
    events.push(burst);
    let spend = spend_points(&events, &ModelPrices);
    let calibration = calibrate(&polls(), &spend, ts("2026-09-23T09:30:00Z")).unwrap();
    let observed = Observed {
        used: Percent::new(43.0),
        changed_at: ts("2026-09-23T09:30:00Z"),
        observed_at: ts("2026-09-23T09:30:00Z"),
    };
    calibration
        .estimate_used(observed, &spend, ts("2026-09-23T09:36:00Z"))
        .value()
        - 43.0
}

#[test]
fn the_estimate_follows_the_price_of_what_was_used() {
    let at = "2026-09-23T09:35:00Z";
    let cases = [
        (used(at, "claude-opus-4-5", 0, 0, 20_000), 1.0),
        (used(at, "claude-sonnet-4-5", 0, 0, 20_000), 0.2),
        (used(at, "claude-opus-4-5", 0, 0, 10_000), 0.5),
        (used(at, "claude-opus-4-5", 0, 99_000, 1_000), 0.149),
        (used(at, "claude-sonnet-4-5", 0, 99_000, 1_000), 0.029_8),
        (used(at, "unknown", 0, 0, 20_000), 0.0),
    ];
    for (burst, expected) in cases {
        let rise = estimate_after(burst.clone());
        assert!((rise - expected).abs() < 1e-9, "{burst:?}: {rise}");
    }
}

#[test]
fn merged_spend_is_in_time_order() {
    let point = |at: &str, cost: i64| SpendPoint {
        at: ts(at),
        cost: Some(MicroUsd(cost)),
    };
    let own = [
        point("2026-09-23T09:00:00Z", 1),
        point("2026-09-23T09:20:00Z", 3),
    ];
    let linked = [point("2026-09-23T09:10:00Z", 2)];
    let merged = merge_spend([own.as_slice(), linked.as_slice()]);
    let costs: Vec<Option<i64>> = merged
        .iter()
        .map(|point| point.cost.map(|cost| cost.0))
        .collect();
    assert_eq!(costs, [Some(1), Some(2), Some(3)]);
}

#[tokio::test]
async fn an_ingest_pass_keeps_the_last_day_of_spend_for_the_account() {
    let dir = tempfile::tempdir().unwrap();
    let mut work = account(CODEX, "work");
    work.home = dir.path().to_path_buf();
    let limits = snapshot(
        vec![session(10.0, "2026-09-23T12:00:00Z")],
        "2026-09-23T10:00:00Z",
    );
    let provider = Arc::new(FakeProvider::new(CODEX, vec![work.clone()], limits));
    provider.usage.lock().unwrap().extend([
        event("old", "2026-09-22T07:00:00Z", "gpt-5.5", 100, 10),
        event("b", "2026-09-23T09:30:00Z", "gpt-5.5", 50, 5),
        event("a", "2026-09-23T09:00:00Z", "gpt-5.5", 100, 10),
    ]);
    let dynamic: Arc<dyn Provider> = provider.clone();
    let harness = harness(vec![dynamic]).await;
    let home = harness.core.model().usage_homes.first().unwrap().clone();
    let gone = UsageHome {
        provider: CODEX,
        home: dir.path().join("gone"),
    };
    let stale = SpendPoint {
        at: ts("2026-09-23T09:10:00Z"),
        cost: Some(MicroUsd(1)),
    };
    harness.core.model().history.set_spend(&gone, vec![stale]);
    ingest::pass(&harness.core, &home, &mut None).await;
    let model = harness.core.model();
    let spend = model.account_spend(&work);
    let points: Vec<(Timestamp, Option<i64>)> = spend
        .iter()
        .map(|p| (p.at, p.cost.map(|cost| cost.0)))
        .collect();
    assert_eq!(
        points,
        [
            (ts("2026-09-23T09:00:00Z"), Some(220)),
            (ts("2026-09-23T09:30:00Z"), Some(110)),
        ]
    );
    assert!(model.history.spend_of(&[gone]).is_empty());
}
