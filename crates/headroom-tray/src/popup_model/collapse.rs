use crate::account::{is_signed_out, lacks_subscription, shown_windows, shows_error_notice};
use crate::combined::CombinedGroup;
use crate::i18n::{Lang, fill};
use crate::payload::{Account, Tone};

pub const MORE_GLYPHS: usize = 3;
const ATTENTION_FORMS: [&str; 2] = ["{count} needs attention", "{count} need attention"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attention {
    Notice,
    Tone(Tone),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoreMember<'a> {
    pub provider: &'a str,
    pub name: &'a str,
    pub attention: Option<Attention>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoreAttention {
    pub mark: Attention,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoreSummary {
    pub title: String,
    pub names: String,
    pub providers: Vec<String>,
    pub attention: Option<MoreAttention>,
}

fn has_notice(account: &Account, offline: bool) -> bool {
    is_signed_out(account) || lacks_subscription(account) || shows_error_notice(account, offline)
}

fn rank(tone: Tone) -> u8 {
    match tone {
        Tone::Critical => 2,
        Tone::Warning => 1,
        Tone::Good | Tone::Neutral => 0,
    }
}

fn worst_tone(tones: impl Iterator<Item = Tone>) -> Option<Attention> {
    tones
        .filter(|tone| rank(*tone) > 0)
        .max_by_key(|tone| rank(*tone))
        .map(Attention::Tone)
}

#[must_use]
pub fn account_attention(account: &Account, offline: bool) -> Option<Attention> {
    if has_notice(account, offline) {
        return Some(Attention::Notice);
    }
    worst_tone(shown_windows(account).into_iter().map(|window| window.tone))
}

#[must_use]
pub fn group_attention(
    group: &CombinedGroup,
    accounts: &[&Account],
    offline: bool,
) -> Option<Attention> {
    let noticed = accounts
        .iter()
        .filter(|account| group.account_ids.contains(&account.id))
        .any(|account| has_notice(account, offline));
    if noticed {
        return Some(Attention::Notice);
    }
    worst_tone(group.windows.iter().map(|window| window.tone))
}

fn weight(attention: Attention) -> u8 {
    match attention {
        Attention::Notice => 3,
        Attention::Tone(tone) => rank(tone),
    }
}

fn more_attention(lang: Lang, members: &[MoreMember]) -> Option<MoreAttention> {
    let marks: Vec<Attention> = members.iter().filter_map(|m| m.attention).collect();
    let mark = marks.iter().copied().max_by_key(|mark| weight(*mark))?;
    let count = u64::try_from(marks.len()).unwrap_or(u64::MAX);
    Some(MoreAttention {
        mark,
        text: fill(
            lang.tr_plural(ATTENTION_FORMS, count),
            &[("count", &count.to_string())],
        ),
    })
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
pub fn more_summary(lang: Lang, members: &[MoreMember]) -> Option<MoreSummary> {
    if members.is_empty() {
        return None;
    }
    let count = members.len().to_string();
    let names = unique(members.iter().map(|member| member.name));
    let providers = unique(members.iter().map(|member| member.provider));
    Some(MoreSummary {
        title: fill(lang.tr("{count} more"), &[("count", &count)]),
        names: names.join(", "),
        providers: providers
            .into_iter()
            .take(MORE_GLYPHS)
            .map(str::to_owned)
            .collect(),
        attention: more_attention(lang, members),
    })
}

#[cfg(test)]
#[path = "collapse_tests.rs"]
mod tests;
