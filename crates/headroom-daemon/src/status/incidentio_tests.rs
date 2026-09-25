use headroom_core::account::ProviderId;

use super::*;
use crate::status::sources::source_of;

const OPENAI_COMPONENTS: &str = include_str!("fixtures/incidentio_openai_components.json");
const OPENAI_DEGRADED: &str =
    include_str!("fixtures/incidentio_openai_components_degraded_constructed.json");
const OPENAI_WIDGET_IDLE: &str = include_str!("fixtures/incidentio_openai_widget_idle.json");
const OPENAI_WIDGET_ONGOING: &str =
    include_str!("fixtures/incidentio_openai_widget_ongoing_constructed.json");
const POE_COMPONENTS: &str = include_str!("fixtures/incidentio_poe_components.json");

fn source(provider: &'static str) -> &'static StatusSource {
    source_of(&ProviderId::from_static(provider)).unwrap()
}

#[test]
fn idle_pages_are_none() {
    let codex = assess(source("codex"), OPENAI_COMPONENTS, Some(OPENAI_WIDGET_IDLE)).unwrap();
    assert_eq!(codex, Assessment::default());
    let poe = assess(source("poe"), POE_COMPONENTS, None).unwrap();
    assert_eq!(poe, Assessment::default());
}

#[test]
fn an_ongoing_incident_on_a_followed_component_explains_the_indicator() {
    let assessed = assess(
        source("codex"),
        OPENAI_DEGRADED,
        Some(OPENAI_WIDGET_ONGOING),
    )
    .unwrap();
    assert_eq!(
        assessed,
        Assessment {
            indicator: Indicator::Major,
            event: Some(StatusEvent {
                title: "Increased Codex CLI errors".into(),
                stage: Some("identified".into()),
                started_at: None,
                url: Some("https://status.openai.com/incidents/01KCONSTRUCTEDINCIDENT0001".into()),
            }),
        }
    );
}

#[test]
fn maintenance_in_progress_is_reported_when_nothing_is_worse() {
    let widget: serde_json::Value = serde_json::from_str(OPENAI_WIDGET_ONGOING).unwrap();
    let maintenance_only = serde_json::json!({
        "ongoing_incidents": [],
        "in_progress_maintenances": widget["in_progress_maintenances"],
    })
    .to_string();
    let assessed = assess(source("codex"), OPENAI_COMPONENTS, Some(&maintenance_only)).unwrap();
    assert_eq!(assessed.indicator, Indicator::Maintenance);
    let event = assessed.event.unwrap();
    assert_eq!(event.title, "Codex Web database maintenance");
    assert_eq!(event.stage.as_deref(), Some("in_progress"));
    assert_eq!(
        event.started_at,
        Some("2026-09-23T09:00:00Z".parse().unwrap())
    );
}

#[test]
fn incidents_elsewhere_on_the_page_are_ignored() {
    let widget: serde_json::Value = serde_json::from_str(OPENAI_WIDGET_ONGOING).unwrap();
    let images_only = serde_json::json!({
        "ongoing_incidents": [widget["ongoing_incidents"][1]],
    })
    .to_string();
    let assessed = assess(source("codex"), OPENAI_COMPONENTS, Some(&images_only)).unwrap();
    assert_eq!(assessed, Assessment::default());
}

#[test]
fn an_incident_without_impact_details_is_at_least_minor() {
    let widget = r#"{"ongoing_incidents": [{
        "name": "Codex API errors", "status": "investigating",
        "affected_components": [{"id": "01KMP3KP5MGE23B80K1EK4S8PV"}]
    }]}"#;
    let assessed = assess(source("codex"), OPENAI_COMPONENTS, Some(widget)).unwrap();
    assert_eq!(assessed.indicator, Indicator::Minor);
    assert_eq!(assessed.event.unwrap().url, None);
}

#[test]
fn only_https_links_are_kept_and_tidied() {
    assert_eq!(
        tidy_url("https://status.poe.com//incidents/1"),
        Some("https://status.poe.com/incidents/1".into())
    );
    assert_eq!(tidy_url("http://status.poe.com/incidents/1"), None);
}

#[test]
fn broken_bodies_are_unreadable() {
    let error = assess(source("poe"), "[]", None);
    assert!(matches!(error, Err(StatusError::Unreadable(_))));
    let error = assess(source("codex"), OPENAI_COMPONENTS, Some("{"));
    assert!(matches!(error, Err(StatusError::Unreadable(_))));
}
