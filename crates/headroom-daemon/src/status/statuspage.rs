use jiff::Timestamp;
use serde::Deserialize;

use super::indicator::{self, Indicator};
use super::sources::StatusSource;
use super::{Assessment, StatusError, StatusEvent};

const OPEN_MAINTENANCE: [&str; 2] = ["in_progress", "verifying"];

#[derive(Debug, Deserialize)]
struct Summary {
    components: Vec<Component>,
    #[serde(default)]
    incidents: Vec<Incident>,
    #[serde(default)]
    scheduled_maintenances: Vec<Incident>,
}

#[derive(Debug, Deserialize)]
struct Component {
    id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct Incident {
    id: String,
    name: String,
    status: String,
    impact: Option<String>,
    created_at: Option<Timestamp>,
    started_at: Option<Timestamp>,
    scheduled_for: Option<Timestamp>,
    shortlink: Option<String>,
    #[serde(default)]
    components: Vec<ComponentRef>,
    #[serde(default, rename = "incident_updates")]
    updates: Vec<IncidentUpdate>,
}

#[derive(Debug, Deserialize)]
struct ComponentRef {
    id: String,
}

#[derive(Debug, Deserialize)]
struct IncidentUpdate {
    affected_components: Option<Vec<AffectedComponent>>,
}

#[derive(Debug, Deserialize)]
struct AffectedComponent {
    code: String,
}

pub fn assess(source: &StatusSource, page: &str, body: &str) -> Result<Assessment, StatusError> {
    let summary: Summary = serde_json::from_str(body)?;
    let components = indicator::worst_component(
        summary
            .components
            .iter()
            .filter(|component| source.follows(&component.id))
            .map(|component| component.status.as_str()),
    );
    let incidents = summary
        .incidents
        .iter()
        .filter(|incident| incident.touches(source))
        .map(|incident| (incident.indicator(), incident.event(page)));
    let maintenances = summary
        .scheduled_maintenances
        .iter()
        .filter(|maintenance| OPEN_MAINTENANCE.contains(&maintenance.status.as_str()))
        .filter(|maintenance| maintenance.touches(source))
        .map(|maintenance| (Indicator::Maintenance, maintenance.event(page)));
    Ok(indicator::assessment(
        components,
        incidents.chain(maintenances).collect(),
    ))
}

impl Incident {
    fn touches(&self, source: &StatusSource) -> bool {
        let listed = self.components.iter().map(|component| &component.id);
        let updated = self
            .updates
            .iter()
            .flat_map(|update| update.affected_components.iter().flatten())
            .map(|affected| &affected.code);
        listed.chain(updated).any(|id| source.follows(id))
    }

    fn indicator(&self) -> Indicator {
        self.impact
            .as_deref()
            .map_or(Indicator::None, Indicator::of_impact)
    }

    fn event(&self, page: &str) -> StatusEvent {
        let shortlink = self
            .shortlink
            .clone()
            .filter(|link| link.starts_with("https://"));
        let page = page.trim_end_matches('/');
        StatusEvent {
            title: self.name.clone(),
            stage: Some(self.status.clone()),
            started_at: self.started_at.or(self.scheduled_for).or(self.created_at),
            url: Some(shortlink.unwrap_or_else(|| format!("{page}/incidents/{}", self.id))),
        }
    }
}

#[cfg(test)]
#[path = "statuspage_tests.rs"]
mod tests;
