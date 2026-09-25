use jiff::Timestamp;
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;

use crate::i18n::{Lang, fill};
use crate::payload::TimeFormat;

const WEEK_DAYS: i32 = 7;
const TWELVE_HOUR_LOCALES: [&str; 12] = [
    "en_US", "en_CA", "en_AU", "en_NZ", "en_PH", "en_IN", "hi_IN", "ar_", "ko_KR", "es_MX",
    "es_US", "ur_PK",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Clock {
    #[default]
    H24,
    H12,
}

impl Clock {
    #[must_use]
    pub fn resolve(format: TimeFormat, system: Clock) -> Self {
        match format {
            TimeFormat::H12 => Clock::H12,
            TimeFormat::H24 => Clock::H24,
            TimeFormat::Auto => system,
        }
    }

    #[must_use]
    pub fn from_clock_format(value: &str) -> Option<Self> {
        match value {
            "12h" => Some(Clock::H12),
            "24h" => Some(Clock::H24),
            _ => None,
        }
    }

    #[must_use]
    pub fn from_locale(locale: Option<&str>) -> Self {
        match locale {
            Some(name)
                if TWELVE_HOUR_LOCALES
                    .iter()
                    .any(|prefix| name.starts_with(prefix)) =>
            {
                Clock::H12
            }
            _ => Clock::H24,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Locale {
    pub lang: Lang,
    pub tz: TimeZone,
    pub clock: Clock,
}

impl Locale {
    #[must_use]
    pub fn new(lang: Lang, tz: TimeZone) -> Self {
        Self {
            lang,
            tz,
            clock: Clock::H24,
        }
    }

    #[must_use]
    pub fn with_clock(self, clock: Clock) -> Self {
        Self { clock, ..self }
    }
}

const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTHS_RU: [&str; 12] = [
    "янв.",
    "февр.",
    "мар.",
    "апр.",
    "мая",
    "июн.",
    "июл.",
    "авг.",
    "сент.",
    "окт.",
    "нояб.",
    "дек.",
];
const WEEKDAYS_ON_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const WEEKDAYS_ON_RU: [&str; 7] = [
    "в понедельник",
    "во вторник",
    "в среду",
    "в четверг",
    "в пятницу",
    "в субботу",
    "в воскресенье",
];
const WEEKDAYS_SHORT_RU: [&str; 7] = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];

fn weekday_index(weekday: Weekday) -> usize {
    usize::try_from(weekday.to_monday_zero_offset()).unwrap_or(0)
}

fn month_index(date: Date) -> usize {
    usize::try_from(date.month() - 1).unwrap_or(0)
}

#[must_use]
pub fn hour_minute(hour: i8, minute: i8, clock: Clock) -> String {
    match clock {
        Clock::H24 => format!("{hour:02}:{minute:02}"),
        Clock::H12 => {
            let suffix = if hour < 12 { "AM" } else { "PM" };
            let twelve = match hour % 12 {
                0 => 12,
                other => other,
            };
            format!("{twelve}:{minute:02}\u{a0}{suffix}")
        }
    }
}

#[must_use]
pub fn clock_time(moment: Timestamp, locale: &Locale) -> String {
    let local = moment.to_zoned(locale.tz.clone());
    hour_minute(local.hour(), local.minute(), locale.clock)
}

fn month_day(lang: Lang, date: Date) -> String {
    let month = match lang {
        Lang::En => MONTHS_EN[month_index(date)],
        Lang::Ru => MONTHS_RU[month_index(date)],
    };
    fill(
        lang.tr("{month} {date}"),
        &[("month", month), ("date", &date.day().to_string())],
    )
}

fn weekday_on(lang: Lang, weekday: Weekday) -> &'static str {
    match lang {
        Lang::En => WEEKDAYS_ON_EN[weekday_index(weekday)],
        Lang::Ru => WEEKDAYS_ON_RU[weekday_index(weekday)],
    }
}

fn weekday_short(lang: Lang, weekday: Weekday) -> &'static str {
    match lang {
        Lang::En => WEEKDAYS_ON_EN[weekday_index(weekday)],
        Lang::Ru => WEEKDAYS_SHORT_RU[weekday_index(weekday)],
    }
}

fn calendar_days_between(from: Date, to: Date) -> i32 {
    from.until(to).map_or(0, |span| span.get_days())
}

#[must_use]
pub fn exact_moment(moment: Timestamp, now: Timestamp, locale: &Locale) -> String {
    let lang = locale.lang;
    let time = clock_time(moment, locale);
    let date = moment.to_zoned(locale.tz.clone()).date();
    let today = now.to_zoned(locale.tz.clone()).date();
    let days = calendar_days_between(today, date);
    if days <= 0 {
        fill(lang.tr("today at {time}"), &[("time", &time)])
    } else if days == 1 {
        fill(lang.tr("tomorrow at {time}"), &[("time", &time)])
    } else if days < WEEK_DAYS {
        let weekday = weekday_on(lang, date.weekday());
        fill(
            lang.tr("{weekday} at {time}"),
            &[("weekday", weekday), ("time", &time)],
        )
    } else {
        let day = month_day(lang, date);
        fill(
            lang.tr("{day} at {time}"),
            &[("day", &day), ("time", &time)],
        )
    }
}

#[must_use]
pub fn day_title(lang: Lang, date: Date) -> String {
    format!(
        "{}, {}",
        weekday_short(lang, date.weekday()),
        month_day(lang, date)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn berlin(lang: Lang) -> Locale {
        Locale::new(lang, TimeZone::get("Europe/Berlin").unwrap())
    }

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    #[test]
    fn clock_time_uses_the_time_zone() {
        let locale = berlin(Lang::En);
        assert_eq!(clock_time(at("2026-09-23T10:05:00Z"), &locale), "12:05");
        let twelve = locale.with_clock(Clock::H12);
        assert_eq!(
            clock_time(at("2026-09-23T10:05:00Z"), &twelve),
            "12:05\u{a0}PM"
        );
    }

    #[test]
    fn twelve_hour_clock_edges() {
        let cases = [
            (0, 0, "12:00\u{a0}AM"),
            (9, 7, "9:07\u{a0}AM"),
            (12, 30, "12:30\u{a0}PM"),
            (23, 59, "11:59\u{a0}PM"),
        ];
        for (hour, minute, expected) in cases {
            assert_eq!(hour_minute(hour, minute, Clock::H12), expected);
        }
        assert_eq!(hour_minute(7, 5, Clock::H24), "07:05");
    }

    #[test]
    fn the_clock_follows_the_setting_then_the_system() {
        assert_eq!(Clock::resolve(TimeFormat::H12, Clock::H24), Clock::H12);
        assert_eq!(Clock::resolve(TimeFormat::H24, Clock::H12), Clock::H24);
        assert_eq!(Clock::resolve(TimeFormat::Auto, Clock::H12), Clock::H12);
        assert_eq!(Clock::from_clock_format("12h"), Some(Clock::H12));
        assert_eq!(Clock::from_clock_format("24h"), Some(Clock::H24));
        assert_eq!(Clock::from_clock_format("x"), None);
        assert_eq!(Clock::from_locale(Some("en_US.UTF-8")), Clock::H12);
        assert_eq!(Clock::from_locale(Some("en_GB.UTF-8")), Clock::H24);
        assert_eq!(Clock::from_locale(Some("ru_RU.UTF-8")), Clock::H24);
        assert_eq!(Clock::from_locale(Some("C")), Clock::H24);
        assert_eq!(Clock::from_locale(None), Clock::H24);
    }

    #[test]
    fn exact_moment_in_twelve_hours() {
        let locale = berlin(Lang::En).with_clock(Clock::H12);
        assert_eq!(
            exact_moment(
                at("2026-09-23T21:30:00Z"),
                at("2026-09-23T20:00:00Z"),
                &locale
            ),
            "today at 11:30\u{a0}PM"
        );
    }

    #[test]
    fn exact_moment_by_calendar_day() {
        let now = at("2026-09-23T20:00:00Z");
        let locale = berlin(Lang::En);
        let cases = [
            ("2026-09-23T21:30:00Z", "today at 23:30"),
            ("2026-09-23T22:30:00Z", "tomorrow at 00:30"),
            ("2026-09-26T08:00:00Z", "Sat at 10:00"),
            ("2026-10-05T08:00:00Z", "Oct 5 at 10:00"),
        ];
        for (moment, expected) in cases {
            assert_eq!(exact_moment(at(moment), now, &locale), expected);
        }
    }

    #[test]
    fn exact_moment_in_russian() {
        let now = at("2026-09-23T08:00:00Z");
        let locale = berlin(Lang::Ru);
        assert_eq!(
            exact_moment(at("2026-09-26T08:00:00Z"), now, &locale),
            "в субботу в 10:00"
        );
        assert_eq!(
            exact_moment(at("2026-10-05T08:00:00Z"), now, &locale),
            "5 окт. в 10:00"
        );
    }

    #[test]
    fn day_titles() {
        let date = Date::new(2026, 9, 23).unwrap();
        assert_eq!(day_title(Lang::En, date), "Wed, Sep 23");
        assert_eq!(day_title(Lang::Ru, date), "ср, 23 сент.");
    }
}
