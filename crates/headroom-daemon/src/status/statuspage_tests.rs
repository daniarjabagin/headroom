use headroom_core::account::ProviderId;

use super::*;
use crate::status::sources::source_of;

const CLAUDE_OPERATIONAL: &str = include_str!("fixtures/statuspage_claude_operational.json");
const CLAUDE_OUTAGE: &str = include_str!("fixtures/statuspage_claude_outage_constructed.json");
const KILO_INCIDENT: &str = include_str!("fixtures/statuspage_kilo_incident.json");
const GITHUB_UNRELATED: &str =
    include_str!("fixtures/statuspage_github_unrelated_outage_constructed.json");
const CURSOR_MAINTENANCE: &str =
    include_str!("fixtures/statuspage_cursor_maintenance_constructed.json");

fn assess_for(provider: &'static str, page: &str, body: &str) -> Assessment {
    let source = source_of(&ProviderId::from_static(provider)).unwrap();
    assess(source, page, body).unwrap()
}

fn ts(text: &str) -> Timestamp {
    text.parse().unwrap()
}

#[test]
fn an_operational_page_is_none_without_an_event() {
    let assessed = assess_for("claude", "https://status.claude.com", CLAUDE_OPERATIONAL);
    assert_eq!(assessed, Assessment::default());
}

#[test]
fn a_live_incident_on_a_followed_component_is_minor() {
    let assessed = assess_for("kilo", "https://status.kilo.ai", KILO_INCIDENT);
    assert_eq!(
        assessed,
        Assessment {
            indicator: Indicator::Minor,
            event: Some(StatusEvent {
                title: "KiloPass MiniMax coding plan subscriptions issues".into(),
                stage: Some("investigating".into()),
                started_at: Some(ts("2026-09-23T14:36:33.579Z")),
                url: Some("https://stspg.io/zgbjyn8vtk6w".into()),
            }),
        }
    );
}

#[test]
fn a_major_incident_links_to_the_page_without_a_shortlink() {
    let assessed = assess_for("claude", "https://status.claude.com/", CLAUDE_OUTAGE);
    assert_eq!(assessed.indicator, Indicator::Major);
    let event = assessed.event.unwrap();
    assert_eq!(event.title, "Elevated errors on Claude Code");
    assert_eq!(event.stage.as_deref(), Some("investigating"));
    assert_eq!(event.started_at, Some(ts("2026-09-23T09:12:00Z")));
    assert_eq!(
        event.url.as_deref(),
        Some("https://status.claude.com/incidents/constructed02")
    );
}

#[test]
fn an_outage_of_an_unfollowed_component_leaves_the_provider_clear() {
    let assessed = assess_for("copilot", "https://www.githubstatus.com", GITHUB_UNRELATED);
    assert_eq!(assessed, Assessment::default());
}

#[test]
fn only_maintenance_in_progress_on_a_followed_component_counts() {
    let assessed = assess_for("cursor", "https://status.cursor.com", CURSOR_MAINTENANCE);
    assert_eq!(
        assessed,
        Assessment {
            indicator: Indicator::Maintenance,
            event: Some(StatusEvent {
                title: "Database upgrade".into(),
                stage: Some("in_progress".into()),
                started_at: Some(ts("2026-09-23T09:45:00Z")),
                url: Some("https://status.cursor.com/incidents/constructedm1".into()),
            }),
        }
    );
}

#[test]
fn incidents_found_only_through_their_updates_still_count() {
    let body = r#"{
        "components": [{"id": "pjmpxvq2cmr2", "status": "operational"}],
        "incidents": [{
            "id": "x1", "name": "Copilot chat is slow", "status": "monitoring", "impact": "minor",
            "created_at": "2026-09-23T08:00:00Z", "shortlink": "http://stspg.io/x1",
            "components": [],
            "incident_updates": [{"affected_components": [{"code": "pjmpxvq2cmr2"}]}]
        }]
    }"#;
    let assessed = assess_for("copilot", "https://www.githubstatus.com", body);
    assert_eq!(assessed.indicator, Indicator::Minor);
    let event = assessed.event.unwrap();
    assert_eq!(event.started_at, Some(ts("2026-09-23T08:00:00Z")));
    assert_eq!(
        event.url.as_deref(),
        Some("https://www.githubstatus.com/incidents/x1")
    );
}

#[test]
fn a_degraded_component_without_an_incident_has_no_event() {
    let body = r#"{"components": [{"id": "pjmpxvq2cmr2", "status": "major_outage"}]}"#;
    let assessed = assess_for("copilot", "https://www.githubstatus.com", body);
    assert_eq!(assessed.indicator, Indicator::Critical);
    assert_eq!(assessed.event, None);
}

#[test]
fn a_summary_without_components_is_unreadable() {
    let source = source_of(&ProviderId::from_static("claude")).unwrap();
    let error = assess(source, "https://status.claude.com", r#"{"incidents": []}"#);
    assert!(matches!(error, Err(StatusError::Unreadable(_))));
}
