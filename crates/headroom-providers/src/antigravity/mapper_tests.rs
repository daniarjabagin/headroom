use super::*;
use crate::antigravity::raw::RawSummaryEnvelope;

const LS_SUMMARY: &str = include_str!("fixtures/quota_summary_ls.json");
const CLOUD_SUMMARY: &str = include_str!("fixtures/quota_summary_cloud.json");
const USER_STATUS: &str = include_str!("fixtures/user_status.json");
const CODE_ASSIST: &str = include_str!("fixtures/load_code_assist.json");

fn mapped(text: &str) -> Vec<QuotaWindow> {
    let envelope: RawSummaryEnvelope = serde_json::from_str(text).unwrap();
    windows(envelope.groups().unwrap())
}

fn summary(windows: &[QuotaWindow]) -> Vec<(WindowId, &str, f64, Option<i64>)> {
    windows
        .iter()
        .map(|w| {
            (
                w.id.clone(),
                w.label.as_str(),
                w.used.value(),
                w.period.map(|p| p.as_secs()),
            )
        })
        .collect()
}

#[test]
fn the_four_pools_map_to_windows_in_a_fixed_order() {
    let windows = mapped(LS_SUMMARY);
    assert_eq!(
        summary(&windows),
        [
            (WindowId::Session, "Session", 25.0, Some(18_000)),
            (WindowId::Weekly, "Weekly", 10.0, Some(604_800)),
            (
                WindowId::Model("claude".into()),
                "Claude",
                60.0,
                Some(18_000)
            ),
            (
                WindowId::Model("claude:weekly".into()),
                "Claude weekly",
                0.0,
                Some(604_800)
            ),
        ]
    );
    assert_eq!(
        windows[0].resets_at,
        Some("2026-09-23T13:00:00Z".parse().unwrap())
    );
}

#[test]
fn bare_cloud_payloads_parse_and_bad_buckets_are_dropped_alone() {
    let windows = mapped(CLOUD_SUMMARY);
    assert_eq!(
        summary(&windows),
        [
            (WindowId::Session, "Session", 50.0, Some(18_000)),
            (WindowId::Weekly, "Weekly", 75.0, Some(604_800)),
        ]
    );
    assert_eq!(windows[1].resets_at, None);
}

#[test]
fn a_payload_without_groups_is_not_a_summary() {
    let envelope: RawSummaryEnvelope = serde_json::from_str(r#"{"response":{}}"#).unwrap();
    assert!(envelope.groups().is_none());
    let empty: RawSummaryEnvelope = serde_json::from_str(r#"{"groups":[]}"#).unwrap();
    assert!(windows(empty.groups().unwrap()).is_empty());
}

#[test]
fn out_of_range_fractions_are_clamped() {
    let windows = mapped(
        r#"{"groups":[{"buckets":[{"bucketId":"gemini-5h","remainingFraction":1.4},{"bucketId":"3p-5h","remainingFraction":-0.2}]}]}"#,
    );
    let used: Vec<f64> = windows.iter().map(|w| w.used.value()).collect();
    assert_eq!(used, [0.0, 100.0]);
}

#[test]
fn plans_prefer_the_google_tier() {
    let status: RawUserStatusEnvelope = serde_json::from_str(USER_STATUS).unwrap();
    assert_eq!(language_server_plan(&status).as_deref(), Some("Pro"));
    let windsurf_only: RawUserStatusEnvelope =
        serde_json::from_str(r#"{"userStatus":{"planStatus":{"planInfo":{"planName":"Teams"}}}}"#)
            .unwrap();
    assert_eq!(
        language_server_plan(&windsurf_only).as_deref(),
        Some("Teams")
    );
    assert_eq!(
        language_server_plan(&RawUserStatusEnvelope::default()),
        None
    );
}

#[test]
fn cloud_plans_reduce_to_the_tier_word() {
    let assist: RawCodeAssist = serde_json::from_str(CODE_ASSIST).unwrap();
    assert_eq!(cloud_plan(&assist).as_deref(), Some("Ultra"));
    let free: RawCodeAssist =
        serde_json::from_str(r#"{"currentTier":{"name":"Gemini Code Assist for individuals"}}"#)
            .unwrap();
    assert_eq!(
        cloud_plan(&free).as_deref(),
        Some("Gemini Code Assist for individuals")
    );
    assert_eq!(cloud_plan(&RawCodeAssist::default()), None);
}
