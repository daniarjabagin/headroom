use super::Settings;
use crate::error::SettingsError;

impl Settings {
    #[must_use]
    pub fn is_dismissed(&self, account_id: &str) -> bool {
        self.dismissed_accounts.contains(account_id)
    }

    pub fn dismiss(&mut self, account_id: &str) {
        self.dismissed_accounts.insert(account_id.to_owned());
    }

    pub fn restore(&mut self, provider: Option<&str>) {
        match provider {
            None => self.dismissed_accounts.clear(),
            Some(provider) => self
                .dismissed_accounts
                .retain(|id| !belongs_to(id, provider)),
        }
    }

    pub(super) fn validate_dismissed(&self) -> Result<(), SettingsError> {
        if self
            .dismissed_accounts
            .iter()
            .any(|id| id.trim().is_empty())
        {
            return Err(SettingsError::BlankDismissedAccount);
        }
        Ok(())
    }
}

fn belongs_to(account_id: &str, provider: &str) -> bool {
    account_id
        .split_once(':')
        .is_some_and(|(owner, _)| owner == provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dismissed(ids: &[&str]) -> Settings {
        let mut settings = Settings::default();
        for id in ids {
            settings.dismiss(id);
        }
        settings
    }

    #[test]
    fn dismissals_are_listed_once() {
        let settings = dismissed(&["grok:a", "grok:a", "cline:b"]);
        assert!(settings.is_dismissed("grok:a"));
        assert!(!settings.is_dismissed("grok:b"));
        let json = serde_json::to_value(&settings).unwrap();
        assert_eq!(
            json["dismissed_accounts"],
            serde_json::json!(["cline:b", "grok:a"])
        );
    }

    #[test]
    fn restoring_a_provider_keeps_the_others() {
        let mut settings = dismissed(&["grok:a", "grok:b", "cline:c", "grokker:d"]);
        settings.restore(Some("grok"));
        let left: Vec<_> = settings.dismissed_accounts.iter().cloned().collect();
        assert_eq!(left, ["cline:c", "grokker:d"]);
        settings.restore(None);
        assert!(settings.dismissed_accounts.is_empty());
    }

    #[test]
    fn blank_dismissed_ids_are_rejected() {
        let result = Settings::parse(r#"{"dismissed_accounts":[" "]}"#);
        assert!(matches!(result, Err(SettingsError::BlankDismissedAccount)));
    }

    #[test]
    fn stored_settings_without_dismissals_load_empty() {
        let settings = Settings::from_stored(r#"{"reduced_motion":true}"#).unwrap();
        assert!(settings.reduced_motion);
        assert!(settings.dismissed_accounts.is_empty());
    }

    #[test]
    fn patches_replace_the_dismissed_list_whole() {
        let settings = dismissed(&["grok:a"]);
        let patched = settings
            .patched(r#"{"dismissed_accounts":["cline:b"]}"#)
            .unwrap();
        assert!(!patched.is_dismissed("grok:a"));
        assert!(patched.is_dismissed("cline:b"));
        let cleared = patched.patched(r#"{"dismissed_accounts":null}"#).unwrap();
        assert!(cleared.dismissed_accounts.is_empty());
    }
}
