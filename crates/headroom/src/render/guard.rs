use headroom_core::pace::Tone;
use jiff::Timestamp;
use serde::Serialize;

use super::format::{reset_text, rounded_percent};
use super::style::Palette;
use crate::cli::WindowScope;
use crate::guard::{CheckedLimit, NoData, Report};

const PASS_MARK: &str = "✓";
const FAIL_MARK: &str = "✗";

#[cfg(target_os = "linux")]
const START_HINT: &str = "(systemctl --user start headroom)";
#[cfg(not(target_os = "linux"))]
const START_HINT: &str = "(open the Headroom app)";

#[derive(Serialize)]
struct JsonReport<'a> {
    ok: bool,
    min_percent: u8,
    window: WindowScope,
    checked: &'a [CheckedLimit],
    failing: &'a [CheckedLimit],
}

pub fn json_report(report: &Report) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&JsonReport {
        ok: report.ok(),
        min_percent: report.min_percent,
        window: report.window,
        checked: &report.checked,
        failing: &report.failing,
    })
}

pub fn verdict_text(report: &Report, now: Timestamp, palette: Palette) -> String {
    let lines: Vec<String> = if report.ok() {
        report
            .lowest()
            .map(|lowest| pass_line(report, lowest, palette))
            .into_iter()
            .collect()
    } else {
        report
            .failing
            .iter()
            .map(|limit| fail_line(limit, report.min_percent, now, palette))
            .collect()
    };
    lines
        .into_iter()
        .flat_map(|line| [line, "\n".to_owned()])
        .collect()
}

fn pass_line(report: &Report, lowest: &CheckedLimit, palette: Palette) -> String {
    let subject = match report.window {
        WindowScope::Any => format!("{} {}", lowest.name, lowest.window_label.to_lowercase()),
        WindowScope::Session | WindowScope::Weekly => lowest.name.clone(),
    };
    format!(
        "{} {} · lowest {} {}% left {}",
        palette.tone(PASS_MARK, Tone::Good),
        scope_title(report.window),
        palette.bold(&subject),
        rounded_percent(lowest.remaining_percent),
        palette.dim(&format!("(min {}%)", report.min_percent)),
    )
}

fn scope_title(scope: WindowScope) -> &'static str {
    match scope {
        WindowScope::Any => "Limits ok",
        WindowScope::Session => "Session limits ok",
        WindowScope::Weekly => "Weekly limits ok",
    }
}

fn fail_line(limit: &CheckedLimit, min: u8, now: Timestamp, palette: Palette) -> String {
    let subject = format!("{} {}:", limit.name, limit.window_label.to_lowercase());
    let left = format!("{}% left", below_percent(limit.remaining_percent, min));
    format!(
        "{} {} {} < {min}% · {}",
        palette.tone(FAIL_MARK, Tone::Critical),
        palette.bold(&subject),
        palette.tone(&left, Tone::Critical),
        reset_text(limit.resets_at, now),
    )
}

fn below_percent(remaining: f64, min: u8) -> String {
    if remaining.max(0.0).round() < f64::from(min) {
        rounded_percent(remaining)
    } else {
        format!("{:.1}", (remaining * 10.0).floor() / 10.0)
    }
}

pub fn no_data_text(reason: &NoData, palette: Palette) -> String {
    let detail = match reason {
        NoData::DaemonNotRunning => {
            format!("the daemon is not running {}", palette.dim(START_HINT))
        }
        NoData::Unreadable(error) => error.clone(),
        NoData::NoMatchingLimits => "no visible limit matches these options".to_owned(),
        NoData::NoFreshLimits(excluded) => format!("no fresh limit data · {}", excluded.join("; ")),
    };
    format!(
        "{} no limit data · {detail}",
        palette.tone("headroom:", Tone::Warning)
    )
}
