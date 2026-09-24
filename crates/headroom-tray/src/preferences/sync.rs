use serde_json::Value;

use super::change::Change;
use super::model::{Settings, SettingsError, decode_settings, merge_patch, settings_from};

#[derive(Debug, Default)]
pub struct SettingsSync {
    raw: Option<Value>,
    settings: Option<Settings>,
    in_flight: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Written {
    Pending,
    Settled,
}

impl SettingsSync {
    #[must_use]
    pub fn settings(&self) -> Option<&Settings> {
        self.settings.as_ref()
    }

    pub fn receive(&mut self, json: &str) -> Result<bool, SettingsError> {
        if self.in_flight > 0 {
            return Ok(false);
        }
        let raw = decode_settings(json)?;
        let settings = settings_from(&raw)?;
        let changed = self.settings.as_ref() != Some(&settings);
        self.raw = Some(raw);
        self.settings = Some(settings);
        Ok(changed)
    }

    pub fn apply(&mut self, change: &Change) -> String {
        let patch = change.patch();
        if let Some(raw) = self.raw.as_mut() {
            merge_patch(raw, &patch);
            match settings_from(raw) {
                Ok(settings) => self.settings = Some(settings),
                Err(error) => tracing::warn!(%error, "a local settings patch did not parse"),
            }
        }
        self.in_flight += 1;
        patch.to_string()
    }

    pub fn written(&mut self) -> Written {
        self.in_flight = self.in_flight.saturating_sub(1);
        if self.in_flight == 0 {
            Written::Settled
        } else {
            Written::Pending
        }
    }

    pub fn forget(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::Theme;

    #[test]
    fn local_changes_show_at_once_and_hide_stale_reads() {
        let mut sync = SettingsSync::default();
        assert!(sync.receive(r#"{"display":{"theme":"light"}}"#).unwrap());
        assert_eq!(sync.settings().unwrap().display.theme, Theme::Light);
        let patch = sync.apply(&Change::Theme(Theme::Dark));
        assert_eq!(patch, r#"{"display":{"theme":"dark"}}"#);
        assert_eq!(sync.settings().unwrap().display.theme, Theme::Dark);
        sync.apply(&Change::CombineAccounts(true));
        assert!(!sync.receive(r#"{"display":{"theme":"light"}}"#).unwrap());
        assert_eq!(sync.settings().unwrap().display.theme, Theme::Dark);
        assert_eq!(sync.written(), Written::Pending);
        assert_eq!(sync.written(), Written::Settled);
        assert!(
            sync.receive(r#"{"display":{"theme":"dark","combine_accounts":true}}"#)
                .is_ok()
        );
        assert!(sync.settings().unwrap().display.combine_accounts);
    }

    #[test]
    fn unchanged_settings_report_no_change() {
        let mut sync = SettingsSync::default();
        assert!(sync.receive("{}").unwrap());
        assert!(!sync.receive("{}").unwrap());
        assert!(sync.receive("[]").is_err());
        assert_eq!(sync.written(), Written::Settled);
        sync.forget();
        assert!(sync.settings().is_none());
    }
}
