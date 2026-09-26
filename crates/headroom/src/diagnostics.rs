use anyhow::{Context, Result};
use headroom_daemon::home::HomeDisplay;
use headroom_daemon::{diagnostics, system_info};
use serde::Deserialize;
use serde_json::json;

#[cfg(target_os = "linux")]
use crate::client::bus;
use crate::client::{self, Daemon};
use crate::logging::daemon_log_path;
use crate::output;
use crate::paths::Globals;

#[derive(Deserialize)]
struct Report {
    text: String,
}

pub async fn print(globals: &Globals) -> Result<()> {
    let text = match client::running_daemon(globals).await? {
        Some(daemon) => report_text(&fetch(&daemon).await?)?,
        None => offline(),
    };
    output::print(&text)
}

async fn fetch(daemon: &Daemon) -> Result<String> {
    match daemon {
        #[cfg(target_os = "linux")]
        Daemon::Bus(proxy) => proxy
            .inner()
            .call("GetDiagnostics", &())
            .await
            .map_err(bus::call_error),
        Daemon::Socket(socket) => Ok(socket
            .call("GetDiagnostics", json!([]))
            .await?
            .get()
            .to_owned()),
    }
}

fn report_text(json: &str) -> Result<String> {
    let report: Report = serde_json::from_str(json)
        .context("the daemon sent a diagnostics report this CLI cannot read")?;
    Ok(report.text)
}

fn offline() -> String {
    let homes = HomeDisplay::new(dirs::home_dir());
    let log_file = daemon_log_path().map(|path| homes.show(&path));
    diagnostics::offline_text(&system_info::detect(), log_file.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_text_is_printed_as_is() {
        let json = r#"{"app_version":"0.6.0","text":"Headroom 0.6.0\nOS: Arch Linux\n"}"#;
        assert_eq!(
            report_text(json).unwrap(),
            "Headroom 0.6.0\nOS: Arch Linux\n"
        );
        assert!(report_text(r#"{"app_version":"0.6.0"}"#).is_err());
    }
}
