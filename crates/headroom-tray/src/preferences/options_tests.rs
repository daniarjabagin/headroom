use super::*;
use crate::payload::PanelLimit;
use crate::payload::parse_state;
use crate::preferences::choices::panel_limit_choices;

const FULL: &str = include_str!("../../../headroom-daemon/src/state/snapshots/state_full.json");

fn values<T: Clone>(choices: &[Choice<T>]) -> Vec<T> {
    choices.iter().map(|choice| choice.value.clone()).collect()
}

fn labels<T>(choices: &[Choice<T>]) -> Vec<&str> {
    choices.iter().map(|choice| choice.label.as_str()).collect()
}

#[test]
fn simple_option_lists_in_both_languages() {
    assert_eq!(labels(&density_choices(Lang::En)), ["Normal", "Compact"]);
    assert_eq!(
        labels(&density_choices(Lang::Ru)),
        ["Обычная", "Компактная"]
    );
    assert_eq!(
        values(&time_format_choices(Lang::En)),
        [TimeFormat::Auto, TimeFormat::H24, TimeFormat::H12]
    );
    assert_eq!(
        labels(&time_format_choices(Lang::Ru)),
        ["Автоматически", "24 часа", "12 часов"]
    );
    assert_eq!(panel_mode_choices(Lang::Ru)[2].label, "Только значок");
    assert_eq!(panel_indicator_choices(Lang::Ru)[1].label, "Полоса");
    assert_eq!(
        spend_unit_choices(Lang::Ru)[2].label,
        "Цена за 1 млн токенов"
    );
    assert_eq!(log_level_choices(Lang::Ru)[0].label, "Только ошибки");
    assert_eq!(
        values(&log_level_choices(Lang::En)),
        [
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug
        ]
    );
}

#[test]
fn every_option_label_is_translated() {
    let russian = [
        labels(&density_choices(Lang::Ru)).join("|"),
        labels(&time_format_choices(Lang::Ru)).join("|"),
        labels(&panel_mode_choices(Lang::Ru)).join("|"),
        labels(&panel_indicator_choices(Lang::Ru)).join("|"),
        labels(&panel_label_choices(
            Lang::Ru,
            Capabilities { release_0_6: true },
        ))
        .join("|"),
        labels(&spend_period_choices(Lang::Ru, None)).join("|"),
        labels(&spend_unit_choices(Lang::Ru)).join("|"),
        labels(&spend_breakdown_choices(Lang::Ru, None)).join("|"),
        labels(&log_level_choices(Lang::Ru)).join("|"),
        labels(&provider_threshold_choices(Lang::Ru, 10, None)).join("|"),
    ];
    for text in russian {
        assert!(!text.chars().any(|c| c.is_ascii_alphabetic()), "{text}");
    }
}

#[test]
fn the_none_label_needs_a_0_6_daemon() {
    let old = panel_label_choices(Lang::En, Capabilities::default());
    assert_eq!(values(&old), [PanelLabel::Percent, PanelLabel::Window]);
    let new = panel_label_choices(Lang::En, Capabilities { release_0_6: true });
    assert_eq!(
        new.last().map(|choice| choice.value),
        Some(PanelLabel::None)
    );
}

#[test]
fn spend_choices_follow_what_the_daemon_sends() {
    let state = parse_state(FULL).unwrap();
    assert_eq!(spend_period_choices(Lang::En, Some(&state.spend)).len(), 4);
    assert_eq!(
        spend_breakdown_choices(Lang::En, Some(&state.spend)).len(),
        2
    );
    let mut old = state.spend.clone();
    old.last_7_days = None;
    old.today.projects = None;
    assert_eq!(
        values(&spend_period_choices(Lang::En, Some(&old))),
        [
            SpendPeriod::Today,
            SpendPeriod::Yesterday,
            SpendPeriod::Last30Days
        ]
    );
    assert_eq!(
        values(&spend_breakdown_choices(Lang::En, Some(&old))),
        [SpendBreakdown::Models]
    );
}

#[test]
fn thresholds_offer_presets_and_keep_a_custom_value() {
    assert_eq!(values(&threshold_choices(Lang::En, 10)), [5, 10, 20, 30]);
    assert_eq!(
        values(&threshold_choices(Lang::En, 15)),
        [5, 10, 15, 20, 30]
    );
    assert_eq!(
        threshold_choices(Lang::Ru, 5)[0].label,
        "Осталось меньше 5%"
    );
    let provider = provider_threshold_choices(Lang::En, 10, Some(45));
    assert_eq!(
        values(&provider),
        [
            None,
            Some(0),
            Some(5),
            Some(10),
            Some(20),
            Some(30),
            Some(45)
        ]
    );
    assert_eq!(provider[0].label, "Same as all (10%)");
    assert_eq!(provider[1].label, "Off");
    assert_eq!(provider_threshold_choices(Lang::En, 10, Some(0)).len(), 6);
}

#[test]
fn quiet_times_step_by_half_an_hour() {
    let choices = quiet_time_choices(Clock::H24, ClockTime::DEFAULT_FROM);
    assert_eq!(choices.len(), 48);
    assert_eq!(choices[1].label, "00:30");
    let custom = ClockTime::new(22, 15).unwrap();
    let twelve = quiet_time_choices(Clock::H12, custom);
    assert_eq!(twelve.len(), 49);
    let index = twelve
        .iter()
        .position(|choice| choice.value == custom)
        .unwrap();
    assert_eq!(twelve[index].label, "10:15\u{a0}PM");
    assert_eq!(twelve[index - 1].value, ClockTime::new(22, 0).unwrap());
}

#[test]
fn panel_limits_list_windows_and_keep_missing_ones() {
    let state = parse_state(FULL).unwrap();
    let gone = PanelLimit {
        account_id: "gone:1".into(),
        window: "weekly".into(),
    };
    let choices = panel_limit_choices(Lang::En, Some(&state), std::slice::from_ref(&gone));
    assert!(choices.len() > 1);
    assert_eq!(choices.last().map(|choice| &choice.value), Some(&gone));
    assert_eq!(
        choices.last().map(|choice| choice.label.as_str()),
        Some("Pinned limit (not available now)")
    );
}
