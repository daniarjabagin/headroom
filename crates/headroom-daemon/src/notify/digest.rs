use headroom_core::pace::Severity;
use jiff::Timestamp;

use super::evaluator::Milestone;
use super::held::{Cause, HeldAlert};
use super::text::{Alert, Locale, Notification, Urgency, body, fill};

const SHOWN_ITEMS: usize = 4;

struct DigestPhrases {
    title: &'static str,
    more: &'static str,
    almost_out: &'static str,
    cutting_it_close: &'static str,
    will_run_out: &'static str,
    limit_reached: &'static str,
    limit_reset: &'static str,
    subscription_inactive: &'static str,
}

const EN: DigestPhrases = DigestPhrases {
    title: "Headroom — while you were away",
    more: "+{} more",
    almost_out: "under {}% left",
    cutting_it_close: "close to the limit",
    will_run_out: "projected to run out",
    limit_reached: "limit reached",
    limit_reset: "limit reset",
    subscription_inactive: "subscription inactive",
};

const RU: DigestPhrases = DigestPhrases {
    title: "Headroom — пока вас не было",
    more: "и ещё {}",
    almost_out: "осталось меньше {}%",
    cutting_it_close: "лимита едва хватит",
    will_run_out: "лимит скоро закончится",
    limit_reached: "лимит исчерпан",
    limit_reset: "лимит сброшен",
    subscription_inactive: "подписка неактивна",
};

fn phrases(locale: Locale) -> &'static DigestPhrases {
    match locale {
        Locale::En => &EN,
        Locale::Ru => &RU,
    }
}

#[must_use]
pub fn compose_digest(
    locale: Locale,
    mut items: Vec<HeldAlert>,
    now: Timestamp,
) -> Option<Notification> {
    items.sort_by(|a, b| a.held_at.cmp(&b.held_at).then_with(|| a.id().cmp(b.id())));
    match items.as_slice() {
        [] => None,
        [only] => Some(original(locale, only, now)),
        many => Some(summary(locale, many, now)),
    }
}

fn original(locale: Locale, held: &HeldAlert, now: Timestamp) -> Notification {
    let mut notification = held.notification.clone();
    if let Cause::Window {
        alert, observed, ..
    } = &held.cause
    {
        notification.body = body(locale, *alert, observed, now);
    }
    notification
}

fn summary(locale: Locale, items: &[HeldAlert], now: Timestamp) -> Notification {
    let phrases = phrases(locale);
    let mut lines: Vec<String> = items
        .iter()
        .take(SHOWN_ITEMS)
        .map(|held| format!("{} — {}", held.heading, short(phrases, &held.cause)))
        .collect();
    let hidden = items.len().saturating_sub(SHOWN_ITEMS);
    if hidden > 0 {
        lines.push(fill(phrases.more, &hidden.to_string()));
    }
    Notification {
        id: format!("summary/{}", now.as_second()),
        account_id: String::new(),
        title: phrases.title.to_owned(),
        body: lines.join("\n"),
        urgency: items
            .iter()
            .map(|held| held.notification.urgency)
            .max()
            .unwrap_or(Urgency::Normal),
    }
}

fn short(phrases: &DigestPhrases, cause: &Cause) -> String {
    let Cause::Window {
        alert, observed, ..
    } = cause
    else {
        return phrases.subscription_inactive.to_owned();
    };
    let Alert {
        milestone,
        threshold,
    } = *alert;
    match milestone {
        Milestone::AlmostOut => fill(phrases.almost_out, &threshold.to_string()),
        Milestone::CuttingItClose => phrases.cutting_it_close.to_owned(),
        Milestone::WillRunOut if observed.severity == Severity::Spent => {
            phrases.limit_reached.to_owned()
        }
        Milestone::WillRunOut => phrases.will_run_out.to_owned(),
        Milestone::Reset => phrases.limit_reset.to_owned(),
    }
}

#[cfg(test)]
#[path = "digest_tests.rs"]
mod tests;
