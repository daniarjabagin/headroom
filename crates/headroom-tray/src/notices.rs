use crate::i18n::{Lang, fill};
use crate::payload::Tone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Warning,
    Error,
    SignIn,
    Info,
}

impl NoticeKind {
    #[must_use]
    pub fn from_tone(tone: Tone) -> Self {
        match tone {
            Tone::Critical => NoticeKind::Error,
            Tone::Warning => NoticeKind::Warning,
            Tone::Good | Tone::Neutral => NoticeKind::Info,
        }
    }
}

const FIXED_TEXTS: [&str; 18] = [
    "Weekly limit shared with Codex Cloud",
    "Offline — showing limits from local logs.",
    "Sign-in expired — open Codex to sign in again. Showing limits from local logs.",
    "Credit balance needs a management key",
    "Credit balance is unavailable right now",
    "No Cline credits left.",
    "Legacy Grok billing has no weekly pool.",
    "Ollama reports no Cloud limits for this account yet.",
    "Could not read the Ollama plan; the usage above is up to date.",
    "Kilo credits are used up",
    "Unlimited credits",
    "No monthly credits on this plan",
    "Balance is not enough for API calls",
    "Balance is used up; API calls fail until you top up",
    "Balance is used up; API requests fail until you top up",
    "Cash balance is negative: the account is in debt",
    "Antigravity reports no quota pools for this account.",
    "The keyring that holds the Antigravity sign-in is locked. Unlock it or start Antigravity.",
];

fn cap_notice(lang: Lang, text: &str) -> Option<String> {
    let amount = text.strip_prefix("Extra usage on, cap ")?;
    (!amount.is_empty()).then(|| {
        fill(
            lang.tr("Extra usage on, cap {amount}"),
            &[("amount", amount)],
        )
    })
}

fn plan_notice(lang: Lang, text: &str, verb: &str, template: &'static str) -> Option<String> {
    let body = text.strip_suffix(" (UTC).")?;
    let (plan, date) = body.rsplit_once(verb)?;
    let valid = !plan.is_empty() && !date.is_empty() && !date.contains(char::is_whitespace);
    valid.then(|| fill(lang.tr(template), &[("plan", plan), ("date", date)]))
}

#[must_use]
pub fn notice_text(lang: Lang, text: &str) -> String {
    if let Some(fixed) = FIXED_TEXTS.iter().find(|fixed| **fixed == text) {
        return lang.tr(fixed).to_owned();
    }
    cap_notice(lang, text)
        .or_else(|| plan_notice(lang, text, " renews on ", "{plan} renews on {date} (UTC)."))
        .or_else(|| plan_notice(lang, text, " ends on ", "{plan} ends on {date} (UTC)."))
        .unwrap_or_else(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fixed_notice_has_a_russian_text() {
        for text in FIXED_TEXTS {
            assert_ne!(notice_text(Lang::Ru, text), text, "{text}");
            assert_eq!(notice_text(Lang::En, text), text);
        }
    }

    #[test]
    fn maps_tones_to_kinds() {
        assert_eq!(NoticeKind::from_tone(Tone::Critical), NoticeKind::Error);
        assert_eq!(NoticeKind::from_tone(Tone::Warning), NoticeKind::Warning);
        assert_eq!(NoticeKind::from_tone(Tone::Good), NoticeKind::Info);
        assert_eq!(NoticeKind::from_tone(Tone::Neutral), NoticeKind::Info);
    }

    #[test]
    fn translates_known_notices() {
        assert_eq!(
            notice_text(Lang::Ru, "No Cline credits left."),
            "Кредиты Cline закончились."
        );
        assert_eq!(
            notice_text(Lang::Ru, "Extra usage on, cap $50.00"),
            "Доп. использование включено, предел $50.00"
        );
        assert_eq!(
            notice_text(Lang::Ru, "Max 5x renews on 2026-10-01 (UTC)."),
            "Max 5x продлевается 2026-10-01 (UTC)."
        );
        assert_eq!(
            notice_text(Lang::Ru, "Pro ends on 2026-10-01 (UTC)."),
            "Pro заканчивается 2026-10-01 (UTC)."
        );
        assert_eq!(notice_text(Lang::Ru, "Something new"), "Something new");
        assert_eq!(
            notice_text(Lang::Ru, "Kilo credits are used up"),
            "Кредиты Kilo закончились"
        );
        assert_eq!(
            notice_text(Lang::En, "Extra usage on, cap $5"),
            "Extra usage on, cap $5"
        );
    }
}
