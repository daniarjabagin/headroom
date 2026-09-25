use serde_json::json;

use crate::model::Model;
use crate::state::tests::{assemble_sample, record};
use crate::status::{Assessment, Indicator, ProviderStatus, StatusEvent};
use crate::testing::{CLAUDE, CODEX, ts};

fn status(indicator: Indicator, event: Option<StatusEvent>, fetched_at: &str) -> ProviderStatus {
    ProviderStatus {
        assessment: Assessment { indicator, event },
        fetched_at: ts(fetched_at),
    }
}

fn incident() -> StatusEvent {
    StatusEvent {
        title: "Elevated errors on Claude Code".into(),
        stage: Some("identified".into()),
        started_at: Some(ts("2026-09-23T09:12:00Z")),
        url: Some("https://stspg.io/abc123".into()),
    }
}

fn model(enabled: bool) -> Model {
    let mut model = Model {
        accounts: vec![record(CLAUDE, "main", 0), record(CODEX, "work", 1)],
        ..Model::default()
    };
    model.settings.status_pages.enabled = enabled;
    model.provider_status.insert(
        CLAUDE,
        status(Indicator::Minor, Some(incident()), "2026-09-23T09:55:00Z"),
    );
    model
        .provider_status
        .insert(CODEX, status(Indicator::None, None, "2026-09-23T09:40:00Z"));
    model
}

fn listed(model: &Model) -> serde_json::Value {
    serde_json::to_value(assemble_sample(model).provider_status).unwrap()
}

#[test]
fn statuses_follow_the_contract_in_registry_order() {
    assert_eq!(
        listed(&model(true)),
        json!([
            {
                "provider": "codex", "indicator": "none", "tone": "neutral", "title": null,
                "stage": null, "started_at": null, "url": "https://status.openai.com"
            },
            {
                "provider": "claude", "indicator": "minor", "tone": "warning",
                "title": "Elevated errors on Claude Code", "stage": "identified",
                "started_at": "2026-09-23T09:12:00Z", "url": "https://stspg.io/abc123"
            }
        ])
    );
}

#[test]
fn nothing_is_listed_while_status_pages_are_off() {
    assert_eq!(listed(&model(false)), json!([]));
}

#[test]
fn only_providers_with_an_account_that_is_not_hidden_are_listed() {
    let mut model = model(true);
    model.accounts[1].hidden = true;
    model.accounts.push(record(CODEX, "gone", 2));
    model.accounts[2].gone = true;
    let listed = listed(&model);
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["provider"], "claude");
}

#[test]
fn statuses_older_than_thirty_minutes_are_dropped() {
    let mut model = model(true);
    model.provider_status.insert(
        CODEX,
        status(Indicator::Major, Some(incident()), "2026-09-23T09:29:00Z"),
    );
    let listed = listed(&model);
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["provider"], "claude");
}

#[test]
fn an_event_without_a_link_points_at_the_status_page() {
    let mut model = model(true);
    let unlinked = StatusEvent {
        url: None,
        ..incident()
    };
    model.provider_status.insert(
        CLAUDE,
        status(
            Indicator::Maintenance,
            Some(unlinked),
            "2026-09-23T09:55:00Z",
        ),
    );
    let listed = listed(&model);
    assert_eq!(listed[1]["url"], "https://status.claude.com");
    assert_eq!(listed[1]["tone"], "warning");
}
