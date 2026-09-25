.pragma library

.import "I18n.js" as I18n

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const WEEK_DAYS = 7;
const LOCALE_SHORT_FORMAT = 1;
const QUOTED = /'[^']*'/g;
const MERIDIEM = /[aA]/;

const WEEKDAYS = [I18n.N("Sun"), I18n.N("Mon"), I18n.N("Tue"), I18n.N("Wed"), I18n.N("Thu"), I18n.N("Fri"), I18n.N("Sat")];

const MONTHS = [I18n.N("Jan"), I18n.N("Feb"), I18n.N("Mar"), I18n.N("Apr"), I18n.N("May"), I18n.N("Jun"), I18n.N("Jul"), I18n.N("Aug"), I18n.N("Sep"), I18n.N("Oct"), I18n.N("Nov"), I18n.N("Dec")];

function twoDigits(value) {
    return String(value).padStart(2, "0");
}

function hourCycle(timeFormat, localePattern) {
    if (timeFormat === "12h" || timeFormat === "24h")
        return timeFormat;
    return MERIDIEM.test(String(localePattern ?? "").replace(QUOTED, "")) ? "12h" : "24h";
}

function localeHourCycle(timeFormat) {
    if (timeFormat === "12h" || timeFormat === "24h")
        return timeFormat;
    return hourCycle(timeFormat, Qt.locale().timeFormat(LOCALE_SHORT_FORMAT));
}

function clockIn(date, cycle) {
    const minutes = twoDigits(date.getMinutes());
    if (cycle !== "12h")
        return `${twoDigits(date.getHours())}:${minutes}`;
    const hours = date.getHours();
    return `${hours % 12 === 0 ? 12 : hours % 12}:${minutes} ${hours < 12 ? "AM" : "PM"}`;
}

function clockTime(date, timeFormat) {
    return clockIn(date, localeHourCycle(timeFormat));
}

function secondsPart(lang, ms) {
    const minutes = Math.floor(ms / MINUTE);
    const seconds = Math.floor((ms % MINUTE) / SECOND);
    if (minutes === 0)
        return I18n.tr(lang, "{seconds}s", {
            seconds: Math.max(1, seconds)
        });
    return I18n.tr(lang, "{minutes}m {seconds}s", {
        minutes,
        seconds: twoDigits(seconds)
    });
}

function duration(lang, ms, withSeconds) {
    const days = Math.floor(ms / DAY);
    const hours = Math.floor((ms % DAY) / HOUR);
    const minutes = Math.floor((ms % HOUR) / MINUTE);
    if (days > 0)
        return I18n.tr(lang, "{days}d {hours}h", {
            days,
            hours
        });
    if (hours > 0)
        return I18n.tr(lang, "{hours}h {minutes}m", {
            hours,
            minutes
        });
    if (withSeconds === true)
        return secondsPart(lang, ms);
    return I18n.tr(lang, "{minutes}m", {
        minutes: Math.max(1, minutes)
    });
}

function startOfDay(date) {
    return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function calendarDaysBetween(from, to) {
    return Math.round((startOfDay(to) - startOfDay(from)) / DAY);
}

function monthDay(lang, date) {
    return I18n.tr(lang, "{month} {day}", {
        month: I18n.tr(lang, MONTHS[date.getMonth()]),
        day: date.getDate()
    });
}

function exactMoment(lang, date, now, timeFormat) {
    const time = clockTime(date, timeFormat);
    const days = calendarDaysBetween(now, date);
    if (days <= 0)
        return I18n.tr(lang, "today at {time}", {
            time
        });
    if (days === 1)
        return I18n.tr(lang, "tomorrow at {time}", {
            time
        });
    const day = days < WEEK_DAYS ? I18n.tr(lang, WEEKDAYS[date.getDay()]) : monthDay(lang, date);
    return I18n.tr(lang, "{day} at {time}", {
        day,
        time
    });
}

function exactResetPhrase(lang, resetsAt, now, timeFormat) {
    if (resetsAt <= now)
        return I18n.tr(lang, "reset pending");
    return I18n.tr(lang, "resets {moment}", {
        moment: exactMoment(lang, resetsAt, now, timeFormat)
    });
}

function resetPhrase(lang, resetsAt, now, resetFormat, live, timeFormat) {
    if (resetFormat === "exact")
        return exactResetPhrase(lang, resetsAt, now, timeFormat);
    const left = resetsAt - now;
    if (left < (live === true ? SECOND : MINUTE))
        return I18n.tr(lang, "resets soon");
    return I18n.tr(lang, "resets in {duration}", {
        duration: duration(lang, left, live)
    });
}

function capitalized(text) {
    return text.charAt(0).toUpperCase() + text.slice(1);
}

function resetText(lang, resetsAt, now, resetFormat, live, timeFormat) {
    if (resetsAt === null)
        return I18n.tr(lang, "Not started");
    return capitalized(resetPhrase(lang, resetsAt, now, resetFormat ?? "countdown", live, timeFormat));
}

function nextUpdateText(lang, nextRefreshAt, now) {
    const left = nextRefreshAt - now;
    if (left < MINUTE)
        return I18n.tr(lang, "Next update in <1m");
    return I18n.tr(lang, "Next update in {duration}", {
        duration: duration(lang, left)
    });
}

function agoText(lang, date, now) {
    const elapsed = now - date;
    if (elapsed < MINUTE)
        return I18n.tr(lang, "just now");
    return I18n.tr(lang, "{duration} ago", {
        duration: duration(lang, elapsed)
    });
}

function isoDate(value) {
    const match = /^(\d{4})-(\d\d)-(\d\d)$/.exec(value);
    return match ? new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3])) : null;
}

function dayText(lang, value) {
    const date = isoDate(value);
    return date === null ? value : monthDay(lang, date);
}
