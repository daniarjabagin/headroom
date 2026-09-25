use headroom_core::account::ProviderId;

const STATUSPAGE_SUMMARY: &str = "/api/v2/summary.json";
const INCIDENT_IO_COMPONENTS: &str = "/api/v2/components.json";
const INCIDENT_IO_WIDGET: &str = "/api/v1/summary";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Statuspage,
    IncidentIo { widget: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSource {
    pub provider: ProviderId,
    pub system: System,
    pub components: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    pub primary: String,
    pub widget: Option<String>,
}

static SOURCES: [StatusSource; 11] = [
    source(
        "codex",
        System::IncidentIo { widget: true },
        &[
            "01JVCV8YSWZFRSM1G5CVP253SK",
            "01KMP3KP5MGE23B80K1EK4S8PV",
            "01KMKFAMWKNQ84Z1766MV08ZDE",
            "01KMP3KP5M8X0EBTVW6KN327EE",
            "01KMKFAMWKQ81YWSE1Z18R6VHR",
        ],
    ),
    source(
        "claude",
        System::Statuspage,
        &["rwppv331jlwc", "k8w3r06qmzrp", "yyzkbfz2thpt"],
    ),
    source(
        "kimi",
        System::Statuspage,
        &["rf64wcbxt3r2", "x0zsqgy57b75", "z2zfp65lvb2z"],
    ),
    source("minimax", System::Statuspage, &["pr0d8qr59svt"]),
    source(
        "devin",
        System::Statuspage,
        &["q72cy1kjpk4r", "d87cp5jknh1c", "h6z52njyz22z"],
    ),
    source(
        "copilot",
        System::Statuspage,
        &["pjmpxvq2cmr2", "cnnb39dkkk82"],
    ),
    source(
        "cursor",
        System::Statuspage,
        &[
            "rflc60xp5jp2",
            "vsny1qv7v86c",
            "mwv1g9sc7kdh",
            "jh0714rgjgt4",
        ],
    ),
    source(
        "kilo",
        System::Statuspage,
        &["kdk5t7ljzmm1", "rkvq7kvz2zlh"],
    ),
    source(
        "warp",
        System::Statuspage,
        &["7z5yq9wznb4f", "z7lnjzhqtty5", "8z327vhmw78l"],
    ),
    source(
        "poe",
        System::IncidentIo { widget: false },
        &["01K8PEESTJRMR8PTME35ZVN6R8", "01K8PEESTJ47BQJEDKF3EFJ5T9"],
    ),
    source("moonshot", System::Statuspage, &["8psr5dfdld0s"]),
];

const fn source(
    provider: &'static str,
    system: System,
    components: &'static [&'static str],
) -> StatusSource {
    StatusSource {
        provider: ProviderId::from_static(provider),
        system,
        components,
    }
}

#[cfg(test)]
pub fn source_of(provider: &ProviderId) -> Option<&'static StatusSource> {
    SOURCES.iter().find(|source| &source.provider == provider)
}

pub fn all() -> impl Iterator<Item = &'static StatusSource> {
    SOURCES.iter()
}

/// Providers whose status page Headroom reads; each needs a `links.status` in its descriptor.
pub fn followed_providers() -> impl Iterator<Item = &'static ProviderId> {
    SOURCES.iter().map(|source| &source.provider)
}

impl StatusSource {
    #[must_use]
    pub fn follows(&self, component: &str) -> bool {
        self.components.contains(&component)
    }

    #[must_use]
    pub fn endpoints(&self, page: &str) -> Endpoints {
        let base = page.trim_end_matches('/');
        match self.system {
            System::Statuspage => Endpoints {
                primary: format!("{base}{STATUSPAGE_SUMMARY}"),
                widget: None,
            },
            System::IncidentIo { widget } => Endpoints {
                primary: format!("{base}{INCIDENT_IO_COMPONENTS}"),
                widget: widget.then(|| format!("{base}{INCIDENT_IO_WIDGET}")),
            },
        }
    }
}

impl Endpoints {
    pub fn urls(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.primary.as_str()).chain(self.widget.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_source_follows_distinct_providers_and_components() {
        let mut providers: Vec<_> = followed_providers().map(ProviderId::as_str).collect();
        providers.sort_unstable();
        providers.dedup();
        assert_eq!(providers.len(), SOURCES.len());
        assert!(SOURCES.iter().all(|source| !source.components.is_empty()));
    }

    #[test]
    fn endpoints_follow_the_system() {
        let claude = source_of(&ProviderId::from_static("claude")).unwrap();
        assert_eq!(
            claude.endpoints("https://status.claude.com/"),
            Endpoints {
                primary: "https://status.claude.com/api/v2/summary.json".into(),
                widget: None,
            }
        );
        let codex = source_of(&ProviderId::from_static("codex")).unwrap();
        let urls: Vec<_> = codex
            .endpoints("https://status.openai.com")
            .urls()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            urls,
            [
                "https://status.openai.com/api/v2/components.json",
                "https://status.openai.com/api/v1/summary"
            ]
        );
        let poe = source_of(&ProviderId::from_static("poe")).unwrap();
        assert_eq!(poe.endpoints("https://status.poe.com").widget, None);
        assert!(source_of(&ProviderId::from_static("grok")).is_none());
    }
}
