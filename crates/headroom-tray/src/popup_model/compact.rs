use jiff::Timestamp;

use crate::dates::{Locale, short_moment};
use crate::format::{duration, millis_between, reset_text};
use crate::payload::{Display, ResetFormat, ValueMode};
use crate::quota::QuotaView;

#[must_use]
pub fn compact_reset(
    locale: &Locale,
    resets_at: Option<Timestamp>,
    now: Timestamp,
    format: ResetFormat,
) -> String {
    let lang = locale.lang;
    let Some(reset) = resets_at else {
        return lang.tr("Not started").to_owned();
    };
    let left = millis_between(now, reset);
    if left <= 0 {
        return lang.tr("reset pending").to_owned();
    }
    match format {
        ResetFormat::Countdown => duration(lang, left, false),
        ResetFormat::Exact => short_moment(reset, now, locale),
    }
}

#[must_use]
pub fn meter_tooltip(
    locale: &Locale,
    view: &QuotaView,
    resets_at: Option<Timestamp>,
    now: Timestamp,
    display: &Display,
) -> String {
    let reset = reset_text(locale, resets_at, now, display.reset_format);
    let lines: Vec<&str> = view
        .note
        .as_ref()
        .map(|note| note.text.as_str())
        .into_iter()
        .chain(std::iter::once(reset.as_str()))
        .chain(view.forecast.as_deref())
        .collect();
    lines.join("\n")
}

#[must_use]
pub fn value_hint(lang: crate::i18n::Lang, mode: ValueMode) -> &'static str {
    lang.tr(match mode {
        ValueMode::Left => "Click to show used",
        ValueMode::Used => "Click to show what's left",
    })
}

#[must_use]
pub fn reset_hint(lang: crate::i18n::Lang, format: ResetFormat) -> &'static str {
    lang.tr(match format {
        ResetFormat::Countdown => "Click to show the reset time",
        ResetFormat::Exact => "Click to show the countdown",
    })
}

#[cfg(test)]
mod tests {
    use jiff::tz::TimeZone;

    use super::*;
    use crate::i18n::Lang;
    use crate::quota::PaceNote;

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    fn locale() -> Locale {
        Locale::new(Lang::En, TimeZone::UTC)
    }

    #[test]
    fn compact_resets_have_no_prefix() {
        let now = at("2026-09-23T10:00:00Z");
        let reset = Some(at("2026-09-23T13:11:00Z"));
        let countdown = compact_reset(&locale(), reset, now, ResetFormat::Countdown);
        assert_eq!(countdown, "3h 11m");
        let exact = compact_reset(&locale(), reset, now, ResetFormat::Exact);
        assert_eq!(exact, "13:11");
        assert_eq!(
            compact_reset(&locale(), None, now, ResetFormat::Countdown),
            "Not started"
        );
        assert_eq!(
            compact_reset(&locale(), Some(now), now, ResetFormat::Exact),
            "reset pending"
        );
    }

    #[test]
    fn the_meter_tooltip_carries_the_hidden_lines() {
        let view = QuotaView {
            label: "Weekly".into(),
            headline: "32% left".into(),
            fill: 0.32,
            tone: crate::payload::Tone::Warning,
            tick: None,
            note: Some(PaceNote {
                flame: true,
                text: "Over pace".into(),
            }),
            trailing: String::new(),
            forecast: Some("At this pace: runs out in 2d 1h".into()),
        };
        let text = meter_tooltip(
            &locale(),
            &view,
            Some(at("2026-09-25T23:00:00Z")),
            at("2026-09-23T10:00:00Z"),
            &Display::default(),
        );
        assert_eq!(
            text,
            "Over pace\nResets in 2d 13h\nAt this pace: runs out in 2d 1h"
        );
    }

    #[test]
    fn hints_name_the_other_mode() {
        assert_eq!(value_hint(Lang::En, ValueMode::Left), "Click to show used");
        assert_eq!(
            reset_hint(Lang::En, ResetFormat::Exact),
            "Click to show the countdown"
        );
    }
}
