use serde_json::json;

use super::*;
use crate::payload::PanelBox;
use crate::preferences::model::ClockTime;

fn patch(change: &Change) -> String {
    change.patch().to_string()
}

#[test]
fn display_changes_touch_one_field() {
    assert_eq!(
        patch(&Change::Theme(Theme::Dark)),
        r#"{"display":{"theme":"dark"}}"#
    );
    assert_eq!(
        patch(&Change::Language(Language::System)),
        r#"{"display":{"language":"system"}}"#
    );
    assert_eq!(
        patch(&Change::ValueMode(ValueMode::Used)),
        r#"{"display":{"value_mode":"used"}}"#
    );
    assert_eq!(
        patch(&Change::ResetFormat(ResetFormat::Exact)),
        r#"{"display":{"reset_format":"exact"}}"#
    );
    assert_eq!(
        patch(&Change::Section(Section::AccountSpend, false)),
        r#"{"display":{"show_account_spend":false}}"#
    );
    assert_eq!(
        patch(&Change::CombineAccounts(true)),
        r#"{"display":{"combine_accounts":true}}"#
    );
    assert_eq!(
        patch(&Change::ShowBreakdown(false)),
        r#"{"display":{"show_breakdown":false}}"#
    );
}

#[test]
fn top_level_changes() {
    assert_eq!(
        patch(&Change::ReducedMotion(true)),
        r#"{"reduced_motion":true}"#
    );
    assert_eq!(
        patch(&Change::RefreshInterval(10)),
        r#"{"refresh_interval_secs":60}"#
    );
    assert_eq!(
        patch(&Change::RefreshInterval(900)),
        r#"{"refresh_interval_secs":900}"#
    );
    assert_eq!(
        patch(&Change::CheckUpdates(false)),
        r#"{"updates":{"check":false}}"#
    );
    assert_eq!(
        patch(&Change::Notify(Milestone::WillRunOut, false)),
        r#"{"notifications":{"will_run_out":false}}"#
    );
}

#[test]
fn headline_is_replaced_whole() {
    assert_eq!(
        patch(&Change::Headline(Headline::Auto)),
        r#"{"headline":{"account_id":null,"mode":"auto","window":null}}"#
    );
    assert_eq!(
        patch(&Change::Headline(Headline::Pinned {
            account_id: "codex:1".into(),
            window: "weekly".into()
        })),
        r#"{"headline":{"account_id":"codex:1","mode":"pinned","window":"weekly"}}"#
    );
}

#[test]
fn an_empty_hidden_list_deletes_the_account_entry() {
    assert_eq!(
        patch(&Change::HiddenWindows {
            account_id: "codex:1".into(),
            windows: vec![]
        }),
        r#"{"display":{"hidden_windows":{"codex:1":null}}}"#
    );
    assert_eq!(
        patch(&Change::HiddenWindows {
            account_id: "codex:1".into(),
            windows: vec!["weekly".into()]
        }),
        r#"{"display":{"hidden_windows":{"codex:1":["weekly"]}}}"#
    );
}

#[test]
fn release_0_6_display_keys() {
    let cases = [
        (
            Change::Density(Density::Compact),
            json!({"display": {"density": "compact"}}),
        ),
        (
            Change::TimeFormat(TimeFormat::H12),
            json!({"display": {"time_format": "12h"}}),
        ),
        (
            Change::PanelMode(PanelMode::Several),
            json!({"display": {"panel_mode": "several"}}),
        ),
        (
            Change::PanelIndicator(PanelIndicator::Bar),
            json!({"display": {"panel_indicator": "bar"}}),
        ),
        (
            Change::PanelLabel(PanelLabel::None),
            json!({"display": {"panel_label": "none"}}),
        ),
        (
            Change::SpendPeriod(SpendPeriod::Last7Days),
            json!({"display": {"spend_period": "7d"}}),
        ),
        (
            Change::SpendUnit(SpendUnit::CostPerMtok),
            json!({"display": {"spend_unit": "cost_per_mtok"}}),
        ),
        (
            Change::SpendBreakdown(SpendBreakdown::Projects),
            json!({"display": {"spend_breakdown": "projects"}}),
        ),
        (
            Change::CollapseUnstarred(true),
            json!({"display": {"collapse_unstarred": true}}),
        ),
        (
            Change::HideOnScreenShare(false),
            json!({"display": {"hide_on_screen_share": false}}),
        ),
    ];
    for (change, expected) in cases {
        assert_eq!(change.patch(), expected, "{change:?}");
    }
}

#[test]
fn lists_are_sent_whole_without_duplicates() {
    let limit = |account: &str| PanelLimit {
        account_id: account.into(),
        window: "session".into(),
    };
    assert_eq!(
        Change::PanelLimits(vec![limit("a"), limit("b"), limit("a")]).patch(),
        json!({"display": {"panel_limits": [
            {"account_id": "a", "window": "session"},
            {"account_id": "b", "window": "session"}
        ]}})
    );
    assert_eq!(
        Change::PanelLimits(vec![]).patch(),
        json!({"display": {"panel_limits": []}})
    );
    assert_eq!(
        Change::StarredAccounts(vec!["a".into(), "a".into(), "b".into()]).patch(),
        json!({"display": {"starred_accounts": ["a", "b"]}})
    );
}

#[test]
fn panel_position_carries_both_fields() {
    let position = PanelPosition {
        panel_box: PanelBox::Left,
        index: 2,
    };
    assert_eq!(
        Change::PanelPosition(position).patch(),
        json!({"display": {"panel_position": {"box": "left", "index": 2}}})
    );
}

#[test]
fn notification_thresholds_merge_per_provider() {
    assert_eq!(
        Change::ThresholdPercent(20).patch(),
        json!({"notifications": {"threshold_percent": 20}})
    );
    assert_eq!(
        Change::ThresholdPercent(0).patch(),
        json!({"notifications": {"threshold_percent": 1}})
    );
    let provider = |threshold| Change::ProviderThreshold {
        provider: "copilot".into(),
        threshold,
    };
    assert_eq!(
        provider(Some(0)).patch(),
        json!({"notifications": {"provider_thresholds": {"copilot": 0}}})
    );
    assert_eq!(
        provider(Some(80)).patch(),
        json!({"notifications": {"provider_thresholds": {"copilot": 50}}})
    );
    assert_eq!(
        provider(None).patch(),
        json!({"notifications": {"provider_thresholds": {"copilot": null}}})
    );
}

#[test]
fn quiet_hours_are_sent_as_one_object() {
    let quiet = QuietHours {
        enabled: true,
        from: ClockTime::new(23, 30).unwrap(),
        ..QuietHours::default()
    };
    assert_eq!(
        Change::QuietHours(quiet).patch(),
        json!({"notifications": {"quiet_hours": {
            "enabled": true, "from": "23:30", "to": "08:00", "allow_critical": true
        }}})
    );
}

#[test]
fn release_0_6_top_level_keys() {
    let cases = [
        (
            Change::AdaptiveRefresh(false),
            json!({"adaptive_refresh": false}),
        ),
        (
            Change::StatusPages(true),
            json!({"status_pages": {"enabled": true}}),
        ),
        (
            Change::Shortcut("<Super>u".into()),
            json!({"shortcuts": {"open": "<Super>u"}}),
        ),
        (
            Change::LogLevel(LogLevel::Debug),
            json!({"logging": {"level": "debug"}}),
        ),
        (
            Change::OnboardingCompleted(true),
            json!({"onboarding": {"completed": true}}),
        ),
    ];
    for (change, expected) in cases {
        assert_eq!(change.patch(), expected, "{change:?}");
    }
}
