use jiff::Timestamp;
use serde::Deserialize;

use super::indicator::{self, Indicator};
use super::sources::StatusSource;
use super::{Assessment, StatusError, StatusEvent};

const HTTPS: &str = "https://";
const MAINTENANCE_PREFIX: &str = "maintenance_";

#[derive(Debug, Deserialize)]
struct Components {
    components: Vec<Component>,
}

#[derive(Debug, Deserialize)]
struct Component {
    id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct Widget {
    #[serde(default)]
    ongoing_incidents: Vec<WidgetItem>,
    #[serde(default)]
    in_progress_maintenances: Vec<WidgetItem>,
}

#[derive(Debug, Deserialize)]
struct WidgetItem {
    name: String,
    status: String,
    url: Option<String>,
    current_worst_impact: Option<String>,
    started_at: Option<Timestamp>,
    starts_at: Option<Timestamp>,
    #[serde(default)]
    affected_components: Vec<AffectedComponent>,
}

#[derive(Debug, Deserialize)]
struct AffectedComponent {
    id: String,
    current_status: Option<String>,
}

pub fn assess(
    source: &StatusSource,
    components: &str,
    widget: Option<&str>,
) -> Result<Assessment, StatusError> {
    let listed: Components = serde_json::from_str(components)?;
    let components = indicator::worst_component(
        listed
            .components
            .iter()
            .filter(|component| source.follows(&component.id))
            .map(|component| component.status.as_str()),
    );
    let events = match widget {
        Some(body) => widget_events(source, &serde_json::from_str(body)?),
        None => Vec::new(),
    };
    Ok(indicator::assessment(components, events))
}

fn widget_events(source: &StatusSource, widget: &Widget) -> Vec<(Indicator, StatusEvent)> {
    let incidents = widget
        .ongoing_incidents
        .iter()
        .filter(|item| item.touches(source))
        .map(|item| (item.incident_indicator(source), item.event()));
    let maintenances = widget
        .in_progress_maintenances
        .iter()
        .filter(|item| item.touches(source))
        .map(|item| (Indicator::Maintenance, item.event()));
    incidents.chain(maintenances).collect()
}

impl WidgetItem {
    fn touches(&self, source: &StatusSource) -> bool {
        self.affected_components
            .iter()
            .any(|component| source.follows(&component.id))
    }

    fn incident_indicator(&self, source: &StatusSource) -> Indicator {
        let followed = indicator::worst_component(
            self.affected_components
                .iter()
                .filter(|component| source.follows(&component.id))
                .filter_map(|component| component.current_status.as_deref()),
        );
        let worst = self
            .current_worst_impact
            .as_deref()
            .map_or(Indicator::None, Indicator::of_component);
        match followed {
            Indicator::None if worst == Indicator::None => Indicator::Minor,
            Indicator::None => worst,
            followed => followed,
        }
    }

    fn event(&self) -> StatusEvent {
        let stage = self
            .status
            .strip_prefix(MAINTENANCE_PREFIX)
            .unwrap_or(&self.status);
        StatusEvent {
            title: self.name.clone(),
            stage: Some(stage.to_owned()),
            started_at: self.started_at.or(self.starts_at),
            url: self.url.as_deref().and_then(tidy_url),
        }
    }
}

fn tidy_url(url: &str) -> Option<String> {
    let rest = url.strip_prefix(HTTPS)?;
    let mut path = rest.to_owned();
    while path.contains("//") {
        path = path.replace("//", "/");
    }
    Some(format!("{HTTPS}{path}"))
}

#[cfg(test)]
#[path = "incidentio_tests.rs"]
mod tests;
