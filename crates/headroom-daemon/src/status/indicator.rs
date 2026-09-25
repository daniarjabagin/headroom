use headroom_core::pace::Tone;
use serde::{Deserialize, Serialize};

use super::{Assessment, StatusEvent};

/// Ordered from harmless to worst, so the worst of several is their maximum.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Indicator {
    #[default]
    None,
    Maintenance,
    Minor,
    Major,
    Critical,
}

impl Indicator {
    #[must_use]
    pub fn of_component(status: &str) -> Indicator {
        match status {
            "degraded_performance" => Indicator::Minor,
            "partial_outage" => Indicator::Major,
            "major_outage" | "full_outage" => Indicator::Critical,
            "under_maintenance" => Indicator::Maintenance,
            _ => Indicator::None,
        }
    }

    #[must_use]
    pub fn of_impact(impact: &str) -> Indicator {
        match impact {
            "minor" => Indicator::Minor,
            "major" => Indicator::Major,
            "critical" => Indicator::Critical,
            "maintenance" => Indicator::Maintenance,
            other => Indicator::of_component(other),
        }
    }

    #[must_use]
    pub fn tone(self) -> Tone {
        match self {
            Indicator::None => Tone::Neutral,
            Indicator::Maintenance | Indicator::Minor => Tone::Warning,
            Indicator::Major | Indicator::Critical => Tone::Critical,
        }
    }
}

pub fn worst_component<'a>(statuses: impl IntoIterator<Item = &'a str>) -> Indicator {
    statuses
        .into_iter()
        .map(Indicator::of_component)
        .max()
        .unwrap_or_default()
}

pub fn assessment(components: Indicator, events: Vec<(Indicator, StatusEvent)>) -> Assessment {
    let worst = events.into_iter().max_by(|(left, a), (right, b)| {
        left.cmp(right)
            .then_with(|| a.started_at.cmp(&b.started_at))
    });
    let indicator = worst
        .as_ref()
        .map_or(components, |(indicator, _)| components.max(*indicator));
    let event = worst
        .filter(|_| indicator != Indicator::None)
        .map(|(_, event)| event);
    Assessment { indicator, event }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(title: &str, started_at: &str) -> StatusEvent {
        StatusEvent {
            title: title.into(),
            stage: None,
            started_at: Some(started_at.parse().unwrap()),
            url: None,
        }
    }

    #[test]
    fn component_and_impact_names_map_to_indicators() {
        let cases = [
            ("operational", Indicator::None),
            ("degraded_performance", Indicator::Minor),
            ("partial_outage", Indicator::Major),
            ("major_outage", Indicator::Critical),
            ("full_outage", Indicator::Critical),
            ("under_maintenance", Indicator::Maintenance),
            ("something_new", Indicator::None),
        ];
        for (status, expected) in cases {
            assert_eq!(Indicator::of_component(status), expected, "{status}");
        }
        assert_eq!(Indicator::of_impact("none"), Indicator::None);
        assert_eq!(Indicator::of_impact("minor"), Indicator::Minor);
        assert_eq!(Indicator::of_impact("critical"), Indicator::Critical);
        assert_eq!(Indicator::of_impact("partial_outage"), Indicator::Major);
    }

    #[test]
    fn tones_follow_the_contract() {
        assert_eq!(Indicator::None.tone(), Tone::Neutral);
        assert_eq!(Indicator::Minor.tone(), Tone::Warning);
        assert_eq!(Indicator::Maintenance.tone(), Tone::Warning);
        assert_eq!(Indicator::Major.tone(), Tone::Critical);
        assert_eq!(Indicator::Critical.tone(), Tone::Critical);
    }

    #[test]
    fn the_worst_event_explains_the_indicator() {
        let events = vec![
            (Indicator::Minor, event("Slow", "2026-09-23T08:00:00Z")),
            (Indicator::Major, event("Down", "2026-09-23T07:00:00Z")),
            (
                Indicator::Major,
                event("Down again", "2026-09-23T09:00:00Z"),
            ),
        ];
        let assessed = assessment(Indicator::Minor, events);
        assert_eq!(assessed.indicator, Indicator::Major);
        assert_eq!(assessed.event.unwrap().title, "Down again");
    }

    #[test]
    fn components_alone_can_raise_the_indicator_without_an_event() {
        let quiet = assessment(Indicator::Critical, Vec::new());
        assert_eq!(quiet.indicator, Indicator::Critical);
        assert_eq!(quiet.event, None);
        let harmless = assessment(
            Indicator::None,
            vec![(Indicator::None, event("FYI", "2026-09-23T08:00:00Z"))],
        );
        assert_eq!(harmless, Assessment::default());
    }
}
