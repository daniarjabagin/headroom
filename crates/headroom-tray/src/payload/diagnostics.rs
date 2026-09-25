use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Diagnostics {
    pub app_version: String,
    #[serde(default)]
    pub log_level: Option<String>,
    #[serde(default)]
    pub log_level_source: Option<String>,
    #[serde(default)]
    pub log_file: Option<String>,
    pub text: String,
}

pub fn parse_diagnostics(json: &str) -> Result<Diagnostics, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_report() {
        let json = r#"{
          "app_version": "0.6.0", "os": "Arch Linux", "desktop": "GNOME (wayland)", "uptime_secs": 7530,
          "transports": ["dbus"], "log_level": "info", "log_level_source": "settings",
          "log_file": "~/.local/state/headroom/headroom.log",
          "providers": [{"provider": "claude", "accounts": 1, "usage_homes": 1}],
          "accounts": [],
          "text": "Headroom 0.6.0\nOS: Arch Linux\n"
        }"#;
        let report = parse_diagnostics(json).unwrap();
        assert_eq!(report.app_version, "0.6.0");
        assert_eq!(
            report.log_file.as_deref(),
            Some("~/.local/state/headroom/headroom.log")
        );
        assert_eq!(report.log_level_source.as_deref(), Some("settings"));
        assert!(report.text.starts_with("Headroom 0.6.0"));
        let bare = parse_diagnostics(r#"{"app_version":"0.6.0","log_file":null,"text":"x"}"#);
        assert_eq!(bare.unwrap().log_file, None);
        assert!(parse_diagnostics(r#"{"app_version":"0.6.0"}"#).is_err());
    }
}
