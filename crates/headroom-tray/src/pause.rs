use crate::i18n::{Lang, fill};

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const DAYS_FROM_HOURS: u64 = 48;

fn rounded(seconds: u64, unit: u64) -> u64 {
    seconds.saturating_add(unit / 2) / unit
}

#[must_use]
pub fn about_duration(lang: Lang, seconds: u64) -> String {
    let minutes = rounded(seconds, MINUTE).max(1);
    if minutes < 60 {
        return fill(
            lang.tr("≈{minutes} min"),
            &[("minutes", &minutes.to_string())],
        );
    }
    let hours = rounded(seconds, HOUR);
    if hours < DAYS_FROM_HOURS {
        return fill(lang.tr("≈{hours} h"), &[("hours", &hours.to_string())]);
    }
    let days = rounded(seconds, DAY);
    fill(lang.tr("≈{days} d"), &[("days", &days.to_string())])
}

#[must_use]
pub fn paused_text(lang: Lang, active_left_seconds: Option<u64>) -> String {
    match active_left_seconds {
        Some(seconds) => fill(
            lang.tr("Paused · lasts {duration} of work"),
            &[("duration", &about_duration(lang, seconds))],
        ),
        None => lang.tr("Paused").to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_round_to_one_unit() {
        let cases = [
            (0, "≈1 min"),
            (29, "≈1 min"),
            (150, "≈3 min"),
            (3_569, "≈59 min"),
            (3_570, "≈1 h"),
            (5_400, "≈2 h"),
            (10_799, "≈3 h"),
            (170_999, "≈47 h"),
            (171_000, "≈2 d"),
            (u64::MAX, "≈213503982334601 d"),
        ];
        for (seconds, expected) in cases {
            assert_eq!(about_duration(Lang::En, seconds), expected, "{seconds}");
        }
        assert_eq!(about_duration(Lang::Ru, 10_800), "≈3 ч");
        assert_eq!(about_duration(Lang::Ru, 600), "≈10 мин");
        assert_eq!(about_duration(Lang::Ru, 259_200), "≈3 д");
    }

    #[test]
    fn paused_line_mentions_the_work_left_when_known() {
        assert_eq!(
            paused_text(Lang::En, Some(10_800)),
            "Paused · lasts ≈3 h of work"
        );
        assert_eq!(
            paused_text(Lang::Ru, Some(10_800)),
            "Пауза · хватит ≈3 ч работы"
        );
        assert_eq!(paused_text(Lang::En, None), "Paused");
        assert_eq!(paused_text(Lang::Ru, None), "Пауза");
    }
}
