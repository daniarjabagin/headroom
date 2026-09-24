use jiff::Timestamp;
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;

use crate::i18n::{Lang, fill};

const WEEK_DAYS: i32 = 7;

#[derive(Debug, Clone)]
pub struct Locale {
    pub lang: Lang,
    pub tz: TimeZone,
}

impl Locale {
    #[must_use]
    pub fn new(lang: Lang, tz: TimeZone) -> Self {
        Self { lang, tz }
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
pub fn clock_time(moment: Timestamp, tz: &TimeZone) -> String {
    let local = moment.to_zoned(tz.clone());
    format!("{:02}:{:02}", local.hour(), local.minute())
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
    let time = clock_time(moment, &locale.tz);
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
        let tz = TimeZone::get("Europe/Berlin").unwrap();
        assert_eq!(clock_time(at("2026-09-23T10:05:00Z"), &tz), "12:05");
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
