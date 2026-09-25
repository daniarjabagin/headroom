use super::sources::{StatusSource, System};
use super::{Assessment, StatusError, incidentio, statuspage};

pub fn assess<'a>(
    source: &StatusSource,
    page: &str,
    body: impl Fn(&str) -> Option<&'a str>,
) -> Result<Assessment, StatusError> {
    let endpoints = source.endpoints(page);
    let primary = body(&endpoints.primary).ok_or(StatusError::NotRead)?;
    match source.system {
        System::Statuspage => statuspage::assess(source, page, primary),
        System::IncidentIo { .. } => {
            let widget = endpoints
                .widget
                .as_deref()
                .map(|url| body(url).ok_or(StatusError::NotRead))
                .transpose()?;
            incidentio::assess(source, primary, widget)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use headroom_core::account::ProviderId;

    use super::*;
    use crate::status::Indicator;
    use crate::status::sources::source_of;

    const OPENAI_DEGRADED: &str =
        include_str!("fixtures/incidentio_openai_components_degraded_constructed.json");
    const OPENAI_WIDGET_IDLE: &str = include_str!("fixtures/incidentio_openai_widget_idle.json");
    const KILO_INCIDENT: &str = include_str!("fixtures/statuspage_kilo_incident.json");

    fn assess_with(
        provider: &'static str,
        page: &str,
        bodies: &HashMap<&str, &'static str>,
    ) -> Result<Assessment, StatusError> {
        let source = source_of(&ProviderId::from_static(provider)).unwrap();
        assess(source, page, |url| bodies.get(url).copied())
    }

    #[test]
    fn each_system_reads_its_own_endpoints() {
        let bodies = HashMap::from([
            ("https://status.kilo.ai/api/v2/summary.json", KILO_INCIDENT),
            (
                "https://status.openai.com/api/v2/components.json",
                OPENAI_DEGRADED,
            ),
            (
                "https://status.openai.com/api/v1/summary",
                OPENAI_WIDGET_IDLE,
            ),
        ]);
        let kilo = assess_with("kilo", "https://status.kilo.ai", &bodies).unwrap();
        assert_eq!(kilo.indicator, Indicator::Minor);
        let codex = assess_with("codex", "https://status.openai.com", &bodies).unwrap();
        assert_eq!(codex.indicator, Indicator::Major);
        assert_eq!(codex.event, None);
    }

    #[test]
    fn a_missing_endpoint_means_not_read_yet() {
        let components_only = HashMap::from([(
            "https://status.openai.com/api/v2/components.json",
            OPENAI_DEGRADED,
        )]);
        assert_eq!(
            assess_with("codex", "https://status.openai.com", &components_only),
            Err(StatusError::NotRead)
        );
        assert_eq!(
            assess_with("kilo", "https://status.kilo.ai", &HashMap::new()),
            Err(StatusError::NotRead)
        );
    }
}
