use crate::i18n::{Lang, fill};

pub const MORE_GLYPHS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoreSummary {
    pub title: String,
    pub names: String,
    pub providers: Vec<String>,
}

fn unique<'a>(items: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut seen: Vec<&str> = Vec::new();
    for item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    seen
}

#[must_use]
pub fn more_summary(lang: Lang, members: &[(&str, &str)]) -> Option<MoreSummary> {
    if members.is_empty() {
        return None;
    }
    let count = members.len().to_string();
    let names = unique(members.iter().map(|(_, name)| *name));
    let providers = unique(members.iter().map(|(provider, _)| *provider));
    Some(MoreSummary {
        title: fill(lang.tr("{count} more"), &[("count", &count)]),
        names: names.join(", "),
        providers: providers
            .into_iter()
            .take(MORE_GLYPHS)
            .map(str::to_owned)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_collapsed_cards() {
        let members = [
            ("copilot", "Copilot"),
            ("grok", "Grok"),
            ("warp", "Warp"),
            ("grok", "Grok"),
            ("kimi", "Kimi"),
        ];
        let summary = more_summary(Lang::En, &members).unwrap();
        assert_eq!(summary.title, "5 more");
        assert_eq!(summary.names, "Copilot, Grok, Warp, Kimi");
        assert_eq!(summary.providers, ["copilot", "grok", "warp"]);
        assert_eq!(more_summary(Lang::Ru, &members).unwrap().title, "Ещё 5");
        assert!(more_summary(Lang::En, &[]).is_none());
    }
}
