use crate::format::{round_percent, window_label};
use crate::i18n::Lang;
use crate::payload::{PanelItem, PanelLabel, PanelMode, State};

pub const APP_TITLE: &str = "Headroom";
const SEPARATOR: &str = " · ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayText {
    pub title: String,
    pub tooltip_title: String,
}

fn percent(item: &PanelItem) -> String {
    format!("{}%", round_percent(item.value_percent))
}

fn window_text(lang: Lang, item: &PanelItem) -> String {
    let headline = &item.headline;
    let window = window_label(lang, &headline.window, &headline.window_label);
    format!("{} {window} {}", headline.provider_name, percent(item))
}

fn joined(items: &[PanelItem], text: impl Fn(&PanelItem) -> String) -> String {
    items.iter().map(text).collect::<Vec<_>>().join(SEPARATOR)
}

fn shown_items(state: Option<&State>) -> Option<(&State, Vec<PanelItem>)> {
    let state = state?;
    if state.display.panel_mode == PanelMode::Icon {
        return None;
    }
    let items = state.resolved_panel_items();
    (!items.is_empty()).then_some((state, items))
}

impl TrayText {
    #[must_use]
    pub fn plain() -> Self {
        Self {
            title: APP_TITLE.to_owned(),
            tooltip_title: APP_TITLE.to_owned(),
        }
    }

    #[must_use]
    pub fn new(state: Option<&State>, lang: Lang) -> Self {
        let Some((state, items)) = shown_items(state) else {
            return Self::plain();
        };
        let title = match state.display.panel_label {
            PanelLabel::Percent => joined(&items, percent),
            PanelLabel::Window => joined(&items, |item| window_text(lang, item)),
            PanelLabel::None => APP_TITLE.to_owned(),
        };
        let tooltip_title = joined(&items, |item| {
            format!("{} {}", item.headline.provider_name, percent(item))
        });
        Self {
            title,
            tooltip_title,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::payload::parse_state;

    const FULL: &str = include_str!("../../../headroom-daemon/src/state/snapshots/state_full.json");

    fn state(display: &Value) -> State {
        let mut raw: Value = serde_json::from_str(FULL).unwrap();
        for (key, value) in display.as_object().unwrap() {
            raw["display"][key] = value.clone();
        }
        let second = json!({
            "account_id": "codex:1", "provider": "codex", "provider_name": "Codex",
            "window": "weekly", "window_label": "Weekly", "used_percent": 60.4,
            "remaining_percent": 39.6, "value_percent": 39.6, "tone": "warning"
        });
        raw["panel_items"].as_array_mut().unwrap().push(second);
        parse_state(&raw.to_string()).unwrap()
    }

    #[test]
    fn tooltip_title_lists_every_item() {
        let text = TrayText::new(Some(&state(&json!({}))), Lang::En);
        assert_eq!(text.tooltip_title, "Claude 8% · Codex 40%");
        assert_eq!(text.title, "8% · 40%");
    }

    #[test]
    fn title_follows_the_panel_label() {
        let window = TrayText::new(Some(&state(&json!({"panel_label": "window"}))), Lang::En);
        assert_eq!(window.title, "Claude Session 8% · Codex Weekly 40%");
        let russian = TrayText::new(Some(&state(&json!({"panel_label": "window"}))), Lang::Ru);
        assert_ne!(russian.title, window.title);
        let none = TrayText::new(Some(&state(&json!({"panel_label": "none"}))), Lang::En);
        assert_eq!(none.title, APP_TITLE);
    }

    #[test]
    fn icon_mode_and_missing_data_show_the_app_name() {
        let icon = TrayText::new(Some(&state(&json!({"panel_mode": "icon"}))), Lang::En);
        assert_eq!(icon, TrayText::plain());
        assert_eq!(TrayText::new(None, Lang::En), TrayText::plain());
    }
}
