use jiff::Timestamp;

use crate::format::{duration, millis_between};
use crate::i18n::{Lang, fill};
use crate::payload::{ProviderStatus, StatusIndicator, Tone};

const MINUTE_MS: i64 = 60_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    pub critical: bool,
    pub title: String,
    pub kind: String,
    pub detail: Option<String>,
    pub started_at: Option<Timestamp>,
    pub url: String,
}

fn kind_text(lang: Lang, indicator: StatusIndicator, has_title: bool) -> &'static str {
    lang.tr(match indicator {
        StatusIndicator::Minor if has_title => "Incident",
        StatusIndicator::Minor | StatusIndicator::None => "Degraded performance",
        StatusIndicator::Major => "Partial outage",
        StatusIndicator::Critical => "Major outage",
        StatusIndicator::Maintenance => "Maintenance",
    })
}

fn stage_text(lang: Lang, stage: &str) -> String {
    let known = match stage {
        "investigating" => Some("Investigating"),
        "identified" => Some("Identified"),
        "monitoring" => Some("Monitoring"),
        "in_progress" => Some("In progress"),
        "verifying" => Some("Verifying"),
        "scheduled" => Some("Scheduled"),
        _ => None,
    };
    known.map_or_else(|| stage.replace('_', " "), |text| lang.tr(text).to_owned())
}

#[must_use]
pub fn status_view(lang: Lang, status: &ProviderStatus) -> Option<StatusView> {
    if status.is_clear() {
        return None;
    }
    let incident = status.title.as_deref().filter(|title| !title.is_empty());
    let kind = kind_text(lang, status.indicator, incident.is_some()).to_owned();
    let title = incident.map_or_else(|| kind.clone(), |name| format!("{kind} · {name}"));
    Some(StatusView {
        critical: status.tone == Tone::Critical,
        title,
        kind,
        detail: status.stage.as_deref().map(|stage| stage_text(lang, stage)),
        started_at: status.started_at,
        url: status.url.clone(),
    })
}

#[must_use]
pub fn elapsed_text(lang: Lang, since: Timestamp, now: Timestamp) -> String {
    let elapsed = millis_between(since, now).max(MINUTE_MS);
    duration(lang, elapsed, false)
}

#[must_use]
pub fn started_text(lang: Lang, since: Timestamp, now: Timestamp) -> String {
    fill(
        lang.tr("Started {duration} ago"),
        &[("duration", &elapsed_text(lang, since, now))],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(indicator: StatusIndicator, tone: Tone, title: Option<&str>) -> ProviderStatus {
        ProviderStatus {
            provider: "claude".into(),
            indicator,
            tone,
            title: title.map(Into::into),
            stage: Some("investigating".into()),
            started_at: Some("2026-09-23T09:22:00Z".parse().unwrap()),
            url: "https://status.claude.com".into(),
        }
    }

    #[test]
    fn clear_pages_show_nothing() {
        assert_eq!(
            status_view(
                Lang::En,
                &status(StatusIndicator::None, Tone::Neutral, None)
            ),
            None
        );
    }

    #[test]
    fn incidents_and_outages_name_their_kind() {
        let minor = status(
            StatusIndicator::Minor,
            Tone::Warning,
            Some("Elevated errors on Claude API"),
        );
        let view = status_view(Lang::En, &minor).unwrap();
        assert_eq!(view.title, "Incident · Elevated errors on Claude API");
        assert_eq!(view.detail.as_deref(), Some("Investigating"));
        assert!(!view.critical);
        let outage = status(StatusIndicator::Critical, Tone::Critical, None);
        let view = status_view(Lang::En, &outage).unwrap();
        assert_eq!(view.title, "Major outage");
        assert!(view.critical);
        let degraded = status(StatusIndicator::Minor, Tone::Warning, None);
        assert_eq!(
            status_view(Lang::En, &degraded).unwrap().kind,
            "Degraded performance"
        );
    }

    #[test]
    fn unknown_stages_stay_readable() {
        assert_eq!(stage_text(Lang::En, "post_incident"), "post incident");
        assert_eq!(stage_text(Lang::Ru, "identified"), "Причина найдена");
    }

    #[test]
    fn started_counts_minutes() {
        let since: Timestamp = "2026-09-23T09:22:00Z".parse().unwrap();
        let now: Timestamp = "2026-09-23T10:00:00Z".parse().unwrap();
        assert_eq!(started_text(Lang::En, since, now), "Started 38m ago");
        assert_eq!(elapsed_text(Lang::En, now, now), "1m");
    }
}
