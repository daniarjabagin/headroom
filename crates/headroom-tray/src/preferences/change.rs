use serde_json::{Value, json};

use super::model::{Headline, MAX_REFRESH_SECS, MIN_REFRESH_SECS};
use crate::payload::{Language, ResetFormat, Theme, ValueMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Spend,
    AccountSpend,
    Trend,
    Forecast,
}

impl Section {
    pub const ALL: [Section; 4] = [
        Section::Spend,
        Section::AccountSpend,
        Section::Trend,
        Section::Forecast,
    ];

    fn field(self) -> &'static str {
        match self {
            Section::Spend => "show_spend",
            Section::AccountSpend => "show_account_spend",
            Section::Trend => "show_trend",
            Section::Forecast => "show_forecast",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Milestone {
    AlmostOut,
    CuttingItClose,
    WillRunOut,
    Reset,
}

impl Milestone {
    pub const ALL: [Milestone; 4] = [
        Milestone::AlmostOut,
        Milestone::CuttingItClose,
        Milestone::WillRunOut,
        Milestone::Reset,
    ];

    fn field(self) -> &'static str {
        match self {
            Milestone::AlmostOut => "almost_out",
            Milestone::CuttingItClose => "cutting_it_close",
            Milestone::WillRunOut => "will_run_out",
            Milestone::Reset => "reset",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Theme(Theme),
    Language(Language),
    ValueMode(ValueMode),
    ResetFormat(ResetFormat),
    Section(Section, bool),
    CombineAccounts(bool),
    ReducedMotion(bool),
    RefreshInterval(u32),
    Headline(Headline),
    CheckUpdates(bool),
    Notify(Milestone, bool),
    HiddenWindows {
        account_id: String,
        windows: Vec<String>,
    },
}

#[must_use]
pub fn theme_name(theme: Theme) -> &'static str {
    match theme {
        Theme::System => "system",
        Theme::Light => "light",
        Theme::Dark => "dark",
    }
}

#[must_use]
pub fn language_name(language: Language) -> &'static str {
    match language {
        Language::System => "system",
        Language::En => "en",
        Language::Ru => "ru",
    }
}

#[must_use]
pub fn value_mode_name(mode: ValueMode) -> &'static str {
    match mode {
        ValueMode::Left => "left",
        ValueMode::Used => "used",
    }
}

#[must_use]
pub fn reset_format_name(format: ResetFormat) -> &'static str {
    match format {
        ResetFormat::Countdown => "countdown",
        ResetFormat::Exact => "exact",
    }
}

fn display(field: &str, value: &Value) -> Value {
    json!({ "display": { field: value } })
}

fn headline_patch(headline: &Headline) -> Value {
    match headline {
        Headline::Auto => {
            json!({ "headline": { "mode": "auto", "account_id": null, "window": null } })
        }
        Headline::Pinned { account_id, window } => json!({
            "headline": { "mode": "pinned", "account_id": account_id, "window": window }
        }),
    }
}

fn hidden_windows_patch(account_id: &str, windows: &[String]) -> Value {
    let list = if windows.is_empty() {
        Value::Null
    } else {
        json!(windows)
    };
    json!({ "display": { "hidden_windows": { account_id: list } } })
}

impl Change {
    #[must_use]
    pub fn patch(&self) -> Value {
        match self {
            Change::Theme(theme) => display("theme", &json!(theme_name(*theme))),
            Change::Language(language) => display("language", &json!(language_name(*language))),
            Change::ValueMode(mode) => display("value_mode", &json!(value_mode_name(*mode))),
            Change::ResetFormat(format) => {
                display("reset_format", &json!(reset_format_name(*format)))
            }
            Change::Section(section, on) => display(section.field(), &json!(on)),
            Change::CombineAccounts(on) => display("combine_accounts", &json!(on)),
            Change::ReducedMotion(on) => json!({ "reduced_motion": on }),
            Change::RefreshInterval(secs) => json!({
                "refresh_interval_secs": (*secs).clamp(MIN_REFRESH_SECS, MAX_REFRESH_SECS)
            }),
            Change::Headline(headline) => headline_patch(headline),
            Change::CheckUpdates(on) => json!({ "updates": { "check": on } }),
            Change::Notify(milestone, on) => {
                json!({ "notifications": { milestone.field(): on } })
            }
            Change::HiddenWindows {
                account_id,
                windows,
            } => hidden_windows_patch(account_id, windows),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
