use headroom_core::pace::Tone;

use super::*;
use crate::testing::{session, ts, weekly};

const NOW: &str = "2026-09-23T10:00:00Z";

fn at(milestone: Milestone) -> Alert {
    Alert {
        milestone,
        threshold: 10,
    }
}

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
        account_id: "codex:work",
        provider_name: "Codex",
        account_name: Some("Work"),
        window: &window,
    };
    compose(locale, at(milestone), &subject, observation, ts(NOW)).body
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
fn almost_out_names_the_threshold() {
    let window = session(50.0, "2026-09-23T10:42:00Z");
    let subject = Subject {
        account_id: "codex:work",
        provider_name: "Codex",
        account_name: Some("Work"),
        window: &window,
    };
    let cases = [
        (
            5,
            "Under 5% left · resets in 42m",
            "Осталось меньше 5% · сброс через 42 мин",
        ),
        (
            20,
            "Under 20% left · resets in 42m",
            "Осталось меньше 20% · сброс через 42 мин",
        ),
        (
            30,
            "Under 30% left · resets in 42m",
            "Осталось меньше 30% · сброс через 42 мин",
        ),
    ];
    for (threshold, english, russian) in cases {
        let alert = Alert {
            milestone: Milestone::AlmostOut,
            threshold,
        };
        let observation = observed(Severity::Close, 1.0);
        let text = |locale| compose(locale, alert, &subject, &observation, ts(NOW)).body;
        assert_eq!(text(Locale::En), english);
        assert_eq!(text(Locale::Ru), russian);
    }
}

#[test]
fn headings_name_provider_account_and_window() {
    let session = session(50.0, NOW);
    let weekly = weekly(50.0, NOW);
    let subject = |account_name, window| Subject {
        account_id: "codex:work",
        provider_name: "Codex",
        account_name,
        window,
    };
    assert_eq!(
        heading(Locale::En, &subject(Some("Work"), &session)),
        "Codex · Work · Session"
    );
    assert_eq!(
        heading(Locale::Ru, &subject(None, &weekly)),
        "Codex · Неделя"
    );
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
            account_id: "codex:work",
            provider_name: "Codex",
            account_name,
            window,
        };
        let text = |locale| {
            compose(
                locale,
                at(Milestone::AlmostOut),
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
        account_id: "codex:work",
        provider_name: "Claude",
        account_name: None,
        window: &window,
    };
    let observation = observed(Severity::Untracked, 100.0);
    let text = compose(
        Locale::En,
        at(Milestone::Reset),
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
    let en = compose_lapse(Locale::En, "codex:work", "Codex", Some("Work"));
    assert_eq!(en.title, "Codex · Work — subscription inactive");
    assert_eq!(en.body, "Limits are unavailable until the plan is renewed.");
    let ru = compose_lapse(Locale::Ru, "codex:work", "Codex", Some("Work"));
    assert_eq!(ru.title, "Codex · Work — подписка неактивна");
    assert_eq!(
        ru.body,
        "Данные о лимитах недоступны, пока подписка не продлена."
    );
    let anonymous = compose_lapse(Locale::En, "claude:x", "Claude", None);
    assert_eq!(anonymous.title, "Claude — subscription inactive");
}

#[test]
fn alerts_carry_a_stable_id_and_an_urgency() {
    let window = session(50.0, "2026-09-23T10:42:00Z");
    let subject = Subject {
        account_id: "codex:work",
        provider_name: "Codex",
        account_name: None,
        window: &window,
    };
    let cases = [
        (
            Milestone::AlmostOut,
            Severity::Close,
            "almost_out",
            Urgency::Normal,
        ),
        (
            Milestone::CuttingItClose,
            Severity::Close,
            "cutting_it_close",
            Urgency::Normal,
        ),
        (
            Milestone::WillRunOut,
            Severity::RunningOut,
            "will_run_out",
            Urgency::Normal,
        ),
        (
            Milestone::WillRunOut,
            Severity::Spent,
            "will_run_out",
            Urgency::Critical,
        ),
        (Milestone::Reset, Severity::Untracked, "reset", Urgency::Low),
    ];
    for (milestone, severity, key, expected) in cases {
        let text = compose(
            Locale::En,
            at(milestone),
            &subject,
            &observed(severity, 5.0),
            ts(NOW),
        );
        assert_eq!(text.id, format!("codex:work/session/{key}"));
        assert_eq!(text.account_id, "codex:work");
        assert_eq!(text.urgency, expected);
    }
    let lapse = compose_lapse(Locale::En, "codex:work", "Codex", None);
    assert_eq!(lapse.id, "codex:work/subscription_inactive");
    assert_eq!(lapse.urgency.as_str(), "normal");
}
