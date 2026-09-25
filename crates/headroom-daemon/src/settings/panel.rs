use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::SettingsError;

pub const MAX_PANEL_LIMITS: usize = 3;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelMode {
    #[default]
    Headline,
    Several,
    Icon,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelIndicator {
    #[default]
    Ring,
    Bar,
    None,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelLabel {
    #[default]
    Percent,
    Window,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelLimit {
    pub account_id: String,
    pub window: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelBox {
    Left,
    Center,
    #[default]
    Right,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelPosition {
    #[serde(rename = "box")]
    pub panel_box: PanelBox,
    pub index: u32,
}

pub(super) fn validate_limits(limits: &[PanelLimit]) -> Result<(), SettingsError> {
    let blank =
        |limit: &PanelLimit| limit.account_id.trim().is_empty() || limit.window.trim().is_empty();
    if limits.iter().any(blank) {
        return Err(SettingsError::BlankPanelLimit);
    }
    let distinct = limits.iter().collect::<HashSet<_>>().len();
    if distinct > MAX_PANEL_LIMITS {
        return Err(SettingsError::TooManyPanelLimits {
            found: distinct,
            max: MAX_PANEL_LIMITS,
        });
    }
    Ok(())
}

pub(super) fn dedup_in_order<T: Clone + Eq + std::hash::Hash>(items: &mut Vec<T>) {
    let mut seen = HashSet::new();
    items.retain(|item| seen.insert(item.clone()));
}
