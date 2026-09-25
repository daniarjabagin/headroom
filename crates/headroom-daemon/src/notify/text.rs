use headroom_core::pace::Severity;
use headroom_core::quota::{QuotaWindow, WindowId};
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

use super::evaluator::{Milestone, Observation};
use crate::model::window_key;
use crate::settings::Language;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Locale {
    #[default]
    En,
    Ru,
}

impl Locale {
    #[must_use]
    pub fn resolve(language: Language, system: Locale) -> Locale {
        match language {
            Language::System => system,
            Language::En => Locale::En,
            Language::Ru => Locale::Ru,
        }
    }

    #[must_use]
    pub fn from_env_values<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Locale {
        let chosen = values
            .into_iter()
            .flatten()
            .find(|value| !value.trim().is_empty());
        match chosen {
            Some(value) if value.trim().to_ascii_lowercase().starts_with("ru") => Locale::Ru,
            _ => Locale::En,
        }
    }

    #[must_use]
    pub fn from_env() -> Locale {
        let read = |name: &str| std::env::var(name).ok();
        let values = [read("LC_ALL"), read("LC_MESSAGES"), read("LANG")];
        Locale::from_env_values(values.iter().map(Option::as_deref))
    }

    fn phrases(self) -> &'static Phrases {
        match self {
            Locale::En => &EN,
            Locale::Ru => &RU,
        }
    }
}

struct Phrases {
    almost_out: &'static str,
    cutting_it_close: &'static str,
    limit_reached: &'static str,
    runs_out_in: &'static str,
    runs_out_before_reset: &'static str,
    limit_reset: &'static str,
    resets_in: &'static str,
    subscription_inactive: &'static str,
    subscription_inactive_body: &'static str,
    session: Option<&'static str>,
    weekly: Option<&'static str>,
    units: Units,
}

struct Units {
    minute: &'static str,
    hour: &'static str,
    day: &'static str,
    gap: &'static str,
}

const EN: Phrases = Phrases {
    almost_out: "Under {}% left",
    cutting_it_close: "Projected to finish close to the limit",
    limit_reached: "Limit reached",
    runs_out_in: "Projected to run out in {}",
    runs_out_before_reset: "Projected to run out before the reset",
    limit_reset: "Limit reset · {}% left",
    resets_in: "resets in {}",
    subscription_inactive: "subscription inactive",
    subscription_inactive_body: "Limits are unavailable until the plan is renewed.",
    session: None,
    weekly: None,
    units: Units {
        minute: "m",
        hour: "h",
        day: "d",
        gap: "",
    },
};

const RU: Phrases = Phrases {
    almost_out: "Осталось меньше {}%",
    cutting_it_close: "По прогнозу лимита едва хватит до сброса",
    limit_reached: "Лимит исчерпан",
    runs_out_in: "По прогнозу лимит закончится через {}",
    runs_out_before_reset: "По прогнозу лимит закончится до сброса",
    limit_reset: "Лимит сброшен · осталось {}%",
    resets_in: "сброс через {}",
    subscription_inactive: "подписка неактивна",
    subscription_inactive_body: "Данные о лимитах недоступны, пока подписка не продлена.",
    session: Some("Сессия"),
    weekly: Some("Неделя"),
    units: Units {
        minute: "мин",
        hour: "ч",
        day: "д",
        gap: " ",
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Low,
    Normal,
    Critical,
}

impl Urgency {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Urgency::Low => "low",
            Urgency::Normal => "normal",
            Urgency::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub account_id: String,
    pub title: String,
    pub body: String,
    pub urgency: Urgency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alert {
    pub milestone: Milestone,
    pub threshold: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subject<'a> {
    pub account_id: &'a str,
    pub provider_name: &'a str,
    pub account_name: Option<&'a str>,
    pub window: &'a QuotaWindow,
}

#[must_use]
pub fn compose(
    locale: Locale,
    alert: Alert,
    subject: &Subject<'_>,
    observed: &Observation,
    now: Timestamp,
) -> Notification {
    let window = window_key(&subject.window.id);
    Notification {
        id: format!(
            "{}/{window}/{}",
            subject.account_id,
            milestone_key(alert.milestone)
        ),
        account_id: subject.account_id.to_owned(),
        title: title(locale, subject),
        body: body(locale, alert, observed, now),
        urgency: urgency(alert.milestone, observed),
    }
}

#[must_use]
pub fn heading(locale: Locale, subject: &Subject<'_>) -> String {
    let window = window_label(locale, subject.window);
    match subject.account_name {
        Some(name) => format!("{} · {name} · {window}", subject.provider_name),
        None => format!("{} · {window}", subject.provider_name),
    }
}

#[must_use]
pub fn compose_lapse(
    locale: Locale,
    account_id: &str,
    provider_name: &str,
    account_name: Option<&str>,
) -> Notification {
    let phrases = locale.phrases();
    Notification {
        id: format!("{account_id}/subscription_inactive"),
        account_id: account_id.to_owned(),
        title: headed(provider_name, account_name, phrases.subscription_inactive),
        body: phrases.subscription_inactive_body.to_owned(),
        urgency: Urgency::Normal,
    }
}

fn milestone_key(milestone: Milestone) -> &'static str {
    match milestone {
        Milestone::Reset => "reset",
        Milestone::WillRunOut => "will_run_out",
        Milestone::CuttingItClose => "cutting_it_close",
        Milestone::AlmostOut => "almost_out",
    }
}

fn urgency(milestone: Milestone, observed: &Observation) -> Urgency {
    match milestone {
        Milestone::Reset => Urgency::Low,
        Milestone::WillRunOut if observed.severity == Severity::Spent => Urgency::Critical,
        Milestone::WillRunOut | Milestone::CuttingItClose | Milestone::AlmostOut => Urgency::Normal,
    }
}

fn title(locale: Locale, subject: &Subject<'_>) -> String {
    let window = window_label(locale, subject.window);
    headed(subject.provider_name, subject.account_name, window)
}

fn headed(provider: &str, account_name: Option<&str>, topic: &str) -> String {
    match account_name {
        Some(name) => format!("{provider} · {name} — {topic}"),
        None => format!("{provider} — {topic}"),
    }
}

fn window_label(locale: Locale, window: &QuotaWindow) -> &str {
    let phrases = locale.phrases();
    let translated = match window.id {
        WindowId::Session => phrases.session,
        WindowId::Weekly => phrases.weekly,
        WindowId::Model(_) | WindowId::Other(_) => None,
    };
    translated.unwrap_or(&window.label)
}

#[must_use]
pub fn body(locale: Locale, alert: Alert, observed: &Observation, now: Timestamp) -> String {
    let phrases = locale.phrases();
    let milestone = alert.milestone;
    let headline = match milestone {
        Milestone::AlmostOut => fill(phrases.almost_out, &alert.threshold.to_string()),
        Milestone::CuttingItClose => phrases.cutting_it_close.to_owned(),
        Milestone::WillRunOut => running_out(locale, observed, now),
        Milestone::Reset => fill(phrases.limit_reset, &whole_percent(observed.remaining)),
    };
    match resets_in(locale, observed, now) {
        Some(countdown) if milestone != Milestone::Reset => {
            format!("{headline} · {}", fill(phrases.resets_in, &countdown))
        }
        _ => headline,
    }
}

fn running_out(locale: Locale, observed: &Observation, now: Timestamp) -> String {
    let phrases = locale.phrases();
    if observed.severity == Severity::Spent {
        return phrases.limit_reached.to_owned();
    }
    match observed.runs_out_at {
        Some(at) => fill(
            phrases.runs_out_in,
            &countdown(locale, at.duration_since(now)),
        ),
        None => phrases.runs_out_before_reset.to_owned(),
    }
}

fn resets_in(locale: Locale, observed: &Observation, now: Timestamp) -> Option<String> {
    observed
        .resets_at
        .map(|at| at.duration_since(now))
        .filter(SignedDuration::is_positive)
        .map(|left| countdown(locale, left))
}

pub(super) fn fill(template: &str, value: &str) -> String {
    template.replacen("{}", value, 1)
}

fn whole_percent(value: f64) -> String {
    format!("{:.0}", value.floor().clamp(0.0, 100.0))
}

#[must_use]
pub fn countdown(locale: Locale, duration: SignedDuration) -> String {
    let Units {
        minute,
        hour,
        day,
        gap,
    } = locale.phrases().units;
    let minutes = duration.as_secs().max(0) / 60;
    let (days, hours, mins) = (minutes / 1_440, minutes / 60 % 24, minutes % 60);
    match (days, hours, mins) {
        (0, 0, 0) => format!("<1{gap}{minute}"),
        (0, 0, m) => format!("{m}{gap}{minute}"),
        (0, h, 0) => format!("{h}{gap}{hour}"),
        (0, h, m) => format!("{h}{gap}{hour} {m}{gap}{minute}"),
        (d, 0, _) => format!("{d}{gap}{day}"),
        (d, h, _) => format!("{d}{gap}{day} {h}{gap}{hour}"),
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
