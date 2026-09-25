use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Diagnostics {
    pub text: String,
    #[serde(default)]
    pub log_file: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("unreadable diagnostics from the Headroom service: {0}")]
pub struct DiagnosticsError(#[from] serde_json::Error);

pub fn parse_diagnostics(json: &str) -> Result<Diagnostics, DiagnosticsError> {
    Ok(serde_json::from_str(json)?)
}

#[must_use]
pub fn expand_home(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None if path == "~" => home.to_path_buf(),
        None => PathBuf::from(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_report_and_log_path() {
        let json = r#"{"app_version":"0.6.0","text":"Headroom 0.6.0\n","log_file":"~/.local/state/headroom/headroom.log","extra":1}"#;
        let diagnostics = parse_diagnostics(json).unwrap();
        assert_eq!(diagnostics.text, "Headroom 0.6.0\n");
        assert_eq!(
            diagnostics.log_file.as_deref(),
            Some("~/.local/state/headroom/headroom.log")
        );
        let without_log = parse_diagnostics(r#"{"text":"x","log_file":null}"#).unwrap();
        assert_eq!(without_log.log_file, None);
        assert!(parse_diagnostics(r#"{"log_file":"x"}"#).is_err());
    }

    #[test]
    fn expands_only_a_leading_tilde() {
        let home = Path::new("/home/ada");
        assert_eq!(
            expand_home("~/.local/state/headroom/headroom.log", home),
            PathBuf::from("/home/ada/.local/state/headroom/headroom.log")
        );
        assert_eq!(expand_home("~", home), PathBuf::from("/home/ada"));
        assert_eq!(expand_home("/var/log/x", home), PathBuf::from("/var/log/x"));
        assert_eq!(expand_home("~ada/x", home), PathBuf::from("~ada/x"));
    }
}
