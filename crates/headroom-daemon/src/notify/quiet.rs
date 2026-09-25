use jiff::civil::Time;
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

use crate::settings::{ClockTime, QuietHours};

#[must_use]
pub fn is_quiet(now_local: Time, hours: QuietHours) -> bool {
    let Some((from, to)) = range(hours) else {
        return false;
    };
    if from < to {
        from <= now_local && now_local < to
    } else {
        now_local >= from || now_local < to
    }
}

#[must_use]
pub fn is_quiet_at(now: Timestamp, tz: &TimeZone, hours: QuietHours) -> bool {
    is_quiet(now.to_zoned(tz.clone()).time(), hours)
}

#[must_use]
pub fn quiet_ends(now: Timestamp, tz: &TimeZone, hours: QuietHours) -> Option<Timestamp> {
    let (_, to) = range(hours)?;
    let local = now.to_zoned(tz.clone());
    if !is_quiet(local.time(), hours) {
        return None;
    }
    let today = end_on(&local, to)?;
    if today > now {
        return Some(today);
    }
    let tomorrow = local.tomorrow().ok()?;
    end_on(&tomorrow, to)
}

fn end_on(day: &Zoned, to: Time) -> Option<Timestamp> {
    let end = day.date().to_datetime(to).to_zoned(day.time_zone().clone());
    end.ok().map(|end| end.timestamp())
}

fn range(hours: QuietHours) -> Option<(Time, Time)> {
    if !hours.enabled || hours.from == hours.to {
        return None;
    }
    Some((local_time(hours.from)?, local_time(hours.to)?))
}

fn local_time(clock: ClockTime) -> Option<Time> {
    Time::strptime("%H:%M", clock.to_string()).ok()
}

#[cfg(test)]
#[path = "quiet_tests.rs"]
mod tests;
