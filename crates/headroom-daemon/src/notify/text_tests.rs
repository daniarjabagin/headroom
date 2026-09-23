use headroom_core::pace::Tone;

use super::*;
use crate::testing::{session, ts, weekly};

const NOW: &str = "2026-09-23T10:00:00Z";

fn observed(severity: Severity, remaining: f64) -> Observation {
    Observation {
        remaining,
        severity,
        tone: Tone::Critical,
        resets_at: Some(ts("2026-09-23T10:42:00Z")),
        runs_out_at: Some(ts("2026-09-23T10:20:00Z")),
    }
}

fn body_of(locale: Locale, milestone: Milestone, observation: &Observation) -> String {
    let window = session(50.0, "2026-09-23T10:42:00Z");
    let subject = Subject {
        provider: ProviderKind::Codex,
        account_name: Some("Work"),
        window: &window,
    };
    compose(locale, milestone, &subject, observation, ts(NOW)).body
}

#[test]
fn bodies_are_translated_for_every_milestone() {
    let mut no_projection = observed(Severity::RunningOut, 30.0);
    no_projection.runs_out_at = None;
    let cases = [
        (
            Milestone::AlmostOut,
            observed(Severity::Close, 8.0),
            "Under 10% left · resets in 42m",
            "Осталось меньше 10% · сброс через 42 мин",
        ),
        (
            Milestone::CuttingItClose,
            observed(Severity::Close, 40.0),
            "Projected to finish close to the limit · resets in 42m",
            "По прогнозу лимита едва хватит до сброса · сброс через 42 мин",
        ),
        (
            Milestone::WillRunOut,
            observed(Severity::RunningOut, 30.0),
            "Projected to run out in 20m · resets in 42m",
            "По прогнозу лимит закончится через 20 мин · сброс через 42 мин",
        ),
        (
            Milestone::WillRunOut,
            no_projection,
            "Projected to run out before the reset · resets in 42m",
            "По прогнозу лимит закончится до сброса · сброс через 42 мин",
        ),
        (
            Milestone::WillRunOut,
            observed(Severity::Spent, 0.0),
            "Limit reached · resets in 42m",
            "Лимит исчерпан · сброс через 42 мин",
        ),
        (
            Milestone::Reset,
            observed(Severity::Untracked, 100.0),
            "Limit reset · 100% left",
            "Лимит сброшен · осталось 100%",
        ),
    ];
    for (milestone, observation, english, russian) in cases {
        assert_eq!(body_of(Locale::En, milestone, &observation), english);
        assert_eq!(body_of(Locale::Ru, milestone, &observation), russian);
    }
}

#[test]
fn titles_translate_known_windows_only() {
    let session = session(50.0, NOW);
    let weekly = weekly(50.0, NOW);
    let mut opus = weekly.clone();
    opus.id = WindowId::Model("opus".into());
    opus.label = "Opus".into();
    let cases = [
        (
            &session,
            Some("Work"),
            "Codex · Work — Session",
            "Codex · Work — Сессия",
        ),
        (&weekly, None, "Codex — Weekly", "Codex — Неделя"),
        (&opus, None, "Codex — Opus", "Codex — Opus"),
    ];
    let observation = observed(Severity::Close, 8.0);
    for (window, account_name, english, russian) in cases {
        let subject = Subject {
            provider: ProviderKind::Codex,
            account_name,
            window,
        };
        let text = |locale| {
            compose(
                locale,
                Milestone::AlmostOut,
                &subject,
                &observation,
                ts(NOW),
            )
        };
        assert_eq!(text(Locale::En).title, english);
        assert_eq!(text(Locale::Ru).title, russian);
    }
}

#[test]
fn claude_titles_name_the_provider() {
    let window = weekly(50.0, NOW);
    let subject = Subject {
        provider: ProviderKind::Claude,
        account_name: None,
        window: &window,
    };
    let observation = observed(Severity::Untracked, 100.0);
    let text = compose(
        Locale::En,
        Milestone::Reset,
        &subject,
        &observation,
        ts(NOW),
    );
    assert_eq!(text.title, "Claude — Weekly");
}

#[test]
fn countdown_formats_compactly_in_both_languages() {
    let cases = [
        (30, "<1m", "<1 мин"),
        (42 * 60, "42m", "42 мин"),
        (3 * 3_600, "3h", "3 ч"),
        (80 * 60, "1h 20m", "1 ч 20 мин"),
        (2 * 86_400, "2d", "2 д"),
        (2 * 86_400 + 3 * 3_600 + 59, "2d 3h", "2 д 3 ч"),
        (-5, "<1m", "<1 мин"),
    ];
    for (secs, english, russian) in cases {
        let duration = SignedDuration::from_secs(secs);
        assert_eq!(countdown(Locale::En, duration), english);
        assert_eq!(countdown(Locale::Ru, duration), russian);
    }
}

#[test]
fn language_setting_overrides_the_system_locale() {
    assert_eq!(Locale::resolve(Language::System, Locale::Ru), Locale::Ru);
    assert_eq!(Locale::resolve(Language::System, Locale::En), Locale::En);
    assert_eq!(Locale::resolve(Language::En, Locale::Ru), Locale::En);
    assert_eq!(Locale::resolve(Language::Ru, Locale::En), Locale::Ru);
}

#[test]
fn system_locale_follows_the_first_set_variable() {
    let cases: [([Option<&str>; 3], Locale); 7] = [
        ([None, None, Some("ru_RU.UTF-8")], Locale::Ru),
        ([None, Some("ru_KZ.UTF-8"), Some("en_US.UTF-8")], Locale::Ru),
        ([None, Some("en_US.UTF-8"), Some("ru_RU.UTF-8")], Locale::En),
        ([Some("C"), Some("ru_RU.UTF-8"), None], Locale::En),
        ([Some(""), None, Some("ru")], Locale::Ru),
        ([None, None, Some("de_DE.UTF-8")], Locale::En),
        ([None, None, None], Locale::En),
    ];
    for (values, expected) in cases {
        assert_eq!(Locale::from_env_values(values), expected, "{values:?}");
    }
}

#[test]
fn lapse_notifications_are_translated() {
    let en = compose_lapse(Locale::En, ProviderKind::Codex, Some("Work"));
    assert_eq!(en.title, "Codex · Work — subscription inactive");
    assert_eq!(en.body, "Limits are unavailable until the plan is renewed.");
    let ru = compose_lapse(Locale::Ru, ProviderKind::Codex, Some("Work"));
    assert_eq!(ru.title, "Codex · Work — подписка неактивна");
    assert_eq!(
        ru.body,
        "Данные о лимитах недоступны, пока подписка не продлена."
    );
    let anonymous = compose_lapse(Locale::En, ProviderKind::Claude, None);
    assert_eq!(anonymous.title, "Claude — subscription inactive");
}
