use jiff::Timestamp;

use crate::dates::Locale;
use crate::format::{
    forecast_text, limit_text, percent_reading, reading_percent, reset_text, spare_text,
    window_label,
};
use crate::payload::{Display, Severity, Tone, ValueMode, Window};

#[derive(Debug, Clone, PartialEq)]
pub struct PaceNote {
    pub flame: bool,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuotaView {
    pub label: String,
    pub headline: String,
    pub fill: f64,
    pub tone: Tone,
    pub tick: Option<f64>,
    pub note: Option<PaceNote>,
    pub trailing: String,
    pub forecast: Option<String>,
}

#[must_use]
pub fn quota_view(
    locale: &Locale,
    window: &Window,
    display: &Display,
    now: Timestamp,
) -> QuotaView {
    let percent = reading_percent(window, display.value_mode);
    QuotaView {
        label: window_label(locale.lang, &window.id, &window.label),
        headline: percent_reading(locale.lang, percent, display.value_mode),
        fill: (percent / 100.0).clamp(0.0, 1.0),
        tone: window.tone,
        tick: tick_position(window, display),
        note: pace_note(locale, window, now, display.show_forecast),
        trailing: reset_text(locale, window.resets_at, now, display.reset_format),
        forecast: display
            .show_forecast
            .then(|| forecast_text(locale, window, now, display))
            .flatten(),
    }
}

fn tick_position(window: &Window, display: &Display) -> Option<f64> {
    let even = window.pace.even_pace_percent?;
    let alarming = matches!(window.tone, Tone::Warning | Tone::Critical);
    if !display.show_forecast && !alarming {
        return None;
    }
    let position = match display.value_mode {
        ValueMode::Used => even / 100.0,
        ValueMode::Left => 1.0 - even / 100.0,
    };
    Some(position.clamp(0.0, 1.0))
}

#[must_use]
pub fn pace_note(
    locale: &Locale,
    window: &Window,
    now: Timestamp,
    show_forecast: bool,
) -> Option<PaceNote> {
    let lang = locale.lang;
    let flame = |text: String| Some(PaceNote { flame: true, text });
    match window.pace.severity {
        Severity::Spent => flame(lang.tr("Limit reached").to_owned()),
        Severity::RunningOut if show_forecast => flame(lang.tr("Over pace").to_owned()),
        Severity::RunningOut => flame(limit_text(lang, window.pace.runs_out_at, now)),
        Severity::Close if !show_forecast => window.pace.spare_percent.map(|spare| PaceNote {
            flame: false,
            text: spare_text(lang, spare),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use jiff::tz::TimeZone;

    use super::*;
    use crate::i18n::Lang;
    use crate::payload::Pace;

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    fn window(tone: Tone, severity: Severity) -> Window {
        Window {
            id: "weekly".into(),
            label: "Weekly".into(),
            used_percent: 68.0,
            remaining_percent: 32.0,
            resets_at: Some(at("2026-09-25T23:00:00Z")),
            tone,
            pace: Pace {
                severity,
                even_pace_percent: Some(60.0),
                projected_percent: Some(92.0),
                spare_percent: Some(8.0),
                runs_out_at: Some(at("2026-09-25T11:00:00Z")),
            },
            hidden: false,
        }
    }

    fn view(window: &Window, display: &Display) -> QuotaView {
        let locale = Locale::new(Lang::En, TimeZone::UTC);
        quota_view(&locale, window, display, at("2026-09-23T10:00:00Z"))
    }

    #[test]
    fn left_mode_reading_and_tick() {
        let quota = view(&window(Tone::Good, Severity::Healthy), &Display::default());
        assert_eq!(quota.label, "Weekly");
        assert_eq!(quota.headline, "32% left");
        assert!((quota.fill - 0.32).abs() < 1e-9);
        assert!((quota.tick.unwrap() - 0.4).abs() < 1e-9);
        assert_eq!(quota.trailing, "Resets in 2d 13h");
        assert_eq!(
            quota.forecast.as_deref(),
            Some("At this pace: ~8% left at reset")
        );
        assert_eq!(quota.note, None);
    }

    #[test]
    fn used_mode_flips_fill_and_tick_but_not_tone() {
        let display = Display {
            value_mode: ValueMode::Used,
            ..Display::default()
        };
        let quota = view(&window(Tone::Warning, Severity::Close), &display);
        assert_eq!(quota.headline, "68% used");
        assert!((quota.fill - 0.68).abs() < 1e-9);
        assert!((quota.tick.unwrap() - 0.6).abs() < 1e-9);
        assert_eq!(quota.tone, Tone::Warning);
    }

    #[test]
    fn pace_notes_follow_the_forecast_setting() {
        let quiet = Display {
            show_forecast: false,
            ..Display::default()
        };
        let running = window(Tone::Critical, Severity::RunningOut);
        let note = view(&running, &Display::default()).note.unwrap();
        assert_eq!((note.flame, note.text.as_str()), (true, "Over pace"));
        let note = view(&running, &quiet).note.unwrap();
        assert_eq!(note.text, "Limit in 2d 1h");
        let close = view(&window(Tone::Warning, Severity::Close), &quiet);
        assert_eq!(close.note.unwrap().text, "~8% spare");
        assert_eq!(close.forecast, None);
        let spent = view(&window(Tone::Critical, Severity::Spent), &quiet);
        assert_eq!(spent.note.unwrap().text, "Limit reached");
    }

    #[test]
    fn healthy_tick_hides_without_forecasts() {
        let quiet = Display {
            show_forecast: false,
            ..Display::default()
        };
        assert_eq!(
            view(&window(Tone::Good, Severity::Healthy), &quiet).tick,
            None
        );
        assert!(
            view(&window(Tone::Warning, Severity::Close), &quiet)
                .tick
                .is_some()
        );
    }
}
