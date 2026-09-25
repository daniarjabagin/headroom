use crate::i18n::{Lang, fill};
use crate::preferences::registry::ProviderLinks;

const HTTPS: &str = "https://";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Status,
    Usage,
    Dashboard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickLink {
    pub kind: LinkKind,
    pub url: String,
    pub host: String,
}

impl LinkKind {
    #[must_use]
    pub fn title(self, lang: Lang) -> &'static str {
        lang.tr(match self {
            LinkKind::Status => "Status page",
            LinkKind::Usage => "Usage page",
            LinkKind::Dashboard => "Open dashboard",
        })
    }
}

impl QuickLink {
    #[must_use]
    pub fn tooltip(&self, lang: Lang) -> String {
        fill(
            lang.tr("{title} · {host}"),
            &[("title", self.kind.title(lang)), ("host", &self.host)],
        )
    }
}

#[must_use]
pub fn link_host(url: &str) -> Option<String> {
    let rest = url.strip_prefix(HTTPS)?;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit('@').next()?;
    (!host.is_empty()).then(|| host.to_owned())
}

fn quick_link(kind: LinkKind, url: Option<&String>) -> Option<QuickLink> {
    let url = url?;
    Some(QuickLink {
        kind,
        host: link_host(url)?,
        url: url.clone(),
    })
}

#[must_use]
pub fn quick_links(links: &ProviderLinks) -> Vec<QuickLink> {
    let usage = links
        .usage
        .as_ref()
        .filter(|usage| links.dashboard.as_ref() != Some(*usage));
    [
        quick_link(LinkKind::Status, links.status.as_ref()),
        quick_link(LinkKind::Usage, usage),
        quick_link(LinkKind::Dashboard, links.dashboard.as_ref()),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_come_from_https_links_only() {
        assert_eq!(
            link_host("https://status.claude.com/incidents/1"),
            Some("status.claude.com".into())
        );
        assert_eq!(link_host("https://chatgpt.com"), Some("chatgpt.com".into()));
        assert_eq!(link_host("http://example.com"), None);
        assert_eq!(link_host("https://"), None);
    }

    #[test]
    fn links_are_ordered_and_deduplicated() {
        let links = ProviderLinks {
            status: Some("https://www.githubstatus.com".into()),
            dashboard: Some("https://github.com/settings/copilot".into()),
            usage: Some("https://github.com/settings/copilot".into()),
        };
        let kinds: Vec<LinkKind> = quick_links(&links).iter().map(|link| link.kind).collect();
        assert_eq!(kinds, [LinkKind::Status, LinkKind::Dashboard]);
        let link = &quick_links(&links)[0];
        assert_eq!(link.tooltip(Lang::En), "Status page · www.githubstatus.com");
        assert!(quick_links(&ProviderLinks::default()).is_empty());
    }
}
