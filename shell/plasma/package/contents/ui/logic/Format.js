.pragma library

.import "I18n.js" as I18n

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const WEEK_DAYS = 7;
const MICROS_PER_CENT = 10000;

const DASH = "—";

const UNITS = [[1e9, "B"], [1e6, "M"], [1e3, "K"]];

const SHORT_WINDOW_LABELS = {
    session: I18n.N("S"),
    weekly: I18n.N("W")
};

const WINDOW_LABELS = {
    session: I18n.N("Session"),
    weekly: I18n.N("Weekly")
};

const WEEKDAYS =[I18n.N("Sun"), I18n.N("Mon"), I18n.N("Tue"), I18n.N("Wed"), I18n.N("Thu"), I18n.N("Fri"), I18n.N("Sat")];

const MONTHS = [I18n.N("Jan"), I18n.N("Feb"), I18n.N("Mar"), I18n.N("Apr"), I18n.N("May"), I18n.N("Jun"), I18n.N("Jul"), I18n.N("Aug"), I18n.N("Sep"), I18n.N("Oct"), I18n.N("Nov"), I18n.N("Dec")];

function roundPercent(value) {
    return Math.max(0, Math.round(value));
}

function percentLeft(lang, remainingPercent) {
    if (remainingPercent === null)
        return DASH;
    return I18n.tr(lang, "{percent}% left", {
        percent: roundPercent(remainingPercent)
    });
}

function percentUsed(lang, usedPercent) {
    if (usedPercent === null)
        return DASH;
    return I18n.tr(lang, "{percent}% used", {
        percent: roundPercent(usedPercent)
    });
}

function readingFor(lang, percent, valueMode) {
    return valueMode === "used" ? percentUsed(lang, percent) : percentLeft(lang, percent);
}

function isPooled(window) {
    return (window.segments?.length ?? 0) > 1;
}

function capacityReading(lang, percent, capacityPercent, valueMode) {
    if (percent === null)
        return DASH;
    const values = {
        percent: roundPercent(percent),
        capacity: roundPercent(capacityPercent)
    };
    if (valueMode === "used")
        return I18n.tr(lang, "{percent}% used of {capacity}%", values);
    return I18n.tr(lang, "{percent}% left of {capacity}%", values);
}

function panelCount(count) {
    return `\u00d7${count}`;
}

function panelPercent(percent) {
    return `${roundPercent(percent)}%`;
}

function shortWindowLabel(lang, windowId, windowLabel) {
    const short = SHORT_WINDOW_LABELS[windowId];
    if (short)
        return I18n.tr(lang, short);
    return windowLabel ?? windowId ?? "";
}

function windowLabel(lang, window) {
    const known = WINDOW_LABELS[window.id];
    return known ? I18n.tr(lang, known) : window.label;
}

function twoDigits(value) {
    return String(value).padStart(2, "0");
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

function clockTime(date) {
    return `${twoDigits(date.getHours())}:${twoDigits(date.getMinutes())}`;
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

function exactMoment(lang, date, now) {
    const time = clockTime(date);
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

function exactResetPhrase(lang, resetsAt, now) {
    if (resetsAt <= now)
        return I18n.tr(lang, "reset pending");
    return I18n.tr(lang, "resets {moment}", {
        moment: exactMoment(lang, resetsAt, now)
    });
}

function resetPhrase(lang, resetsAt, now, resetFormat, live) {
    if (resetFormat === "exact")
        return exactResetPhrase(lang, resetsAt, now);
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

function resetText(lang, resetsAt, now, resetFormat, live) {
    if (resetsAt === null)
        return I18n.tr(lang, "Not started");
    return capitalized(resetPhrase(lang, resetsAt, now, resetFormat ?? "countdown", live));
}

function spareText(lang, sparePercent) {
    return I18n.tr(lang, "~{percent}% spare", {
        percent: roundPercent(sparePercent)
    });
}

function limitText(lang, runsOutAt, now) {
    if (runsOutAt === null || runsOutAt <= now)
        return I18n.tr(lang, "Limit soon");
    return I18n.tr(lang, "Limit in {duration}", {
        duration: duration(lang, runsOutAt - now)
    });
}

function runOutForecast(lang, window, now, resetFormat) {
    const runsOutAt = window.pace.runsOutAt;
    if (runsOutAt === null && isPooled(window))
        return I18n.tr(lang, "At this pace: runs out before reset");
    if (runsOutAt === null || runsOutAt <= now)
        return I18n.tr(lang, "At this pace: runs out any minute");
    const runsOut = I18n.tr(lang, "runs out in {duration}", {
        duration: duration(lang, runsOutAt - now)
    });
    if (window.resetsAt === null)
        return I18n.tr(lang, "At this pace: {runsOut}", {
            runsOut
        });
    return I18n.tr(lang, "At this pace: {runsOut} · {resets}", {
        runsOut,
        resets: resetPhrase(lang, window.resetsAt, now, resetFormat, false)
    });
}

function capacityForecast(lang, window, valueMode) {
    const pace = window.pace;
    const capacity = roundPercent(window.capacityPercent);
    if (valueMode === "used" && pace.projectedPercent !== null)
        return I18n.tr(lang, "At this pace: ~{percent}% of {capacity}% used at reset", {
            percent: roundPercent(pace.projectedPercent),
            capacity
        });
    return I18n.tr(lang, "At this pace: ~{percent}% of {capacity}% left at reset", {
        percent: roundPercent(pace.sparePercent),
        capacity
    });
}

function atResetForecast(lang, window, valueMode) {
    const pace = window.pace;
    if (isPooled(window))
        return capacityForecast(lang, window, valueMode);
    if (valueMode === "used" && pace.projectedPercent !== null)
        return I18n.tr(lang, "At this pace: ~{percent}% used at reset", {
            percent: roundPercent(pace.projectedPercent)
        });
    return I18n.tr(lang, "At this pace: ~{percent}% left at reset", {
        percent: roundPercent(pace.sparePercent)
    });
}

function forecastText(lang, window, now, display) {
    const pace = window.pace;
    if (window.remainingPercent === null)
        return null;
    if (pace.severity === "running_out")
        return runOutForecast(lang, window, now, display.resetFormat);
    if ((pace.severity === "healthy" || pace.severity === "close") && pace.sparePercent !== null)
        return atResetForecast(lang, window, display.valueMode);
    return null;
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

function tokenDigits(scaled) {
    return scaled >= 100 ? 0 : 1;
}

function moneyDigits(scaled) {
    if (scaled >= 100)
        return 0;
    return scaled >= 10 ? 1 : 2;
}

function abbreviate(value, digitsFor) {
    for (const [size, suffix] of UNITS) {
        if (value >= size) {
            const scaled = value / size;
            const digits = scaled.toFixed(digitsFor(scaled));
            return `${digits.replace(/\.0+$/, "")}${suffix}`;
        }
    }
    return String(value);
}

function compactTokens(count) {
    return abbreviate(count, tokenDigits);
}

function exactTokens(count) {
    return String(count).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

function tokenCount(lang, count) {
    return I18n.trn(lang, "{tokens} token", "{tokens} tokens", count, {
        tokens: exactTokens(count)
    });
}

function centsOf(micros) {
    return Math.round(micros / MICROS_PER_CENT);
}

function exactUsd(micros) {
    const cents = centsOf(micros);
    const rest = String(Math.abs(cents % 100)).padStart(2, "0");
    return `$${exactTokens(Math.trunc(cents / 100))}.${rest}`;
}

function usd(micros) {
    const cents = centsOf(micros);
    if (cents >= 100000)
        return `$${abbreviate(cents / 100, moneyDigits)}`;
    return exactUsd(micros);
}

function ringUsd(micros) {
    const cents = centsOf(micros);
    if (cents < 10000)
        return usd(micros);
    if (cents < 1000000)
        return `$${Math.round(cents / 100)}`;
    return usd(micros);
}

function spendLine(lang, totals) {
    if (totals.costMicros === 0 && totals.totalTokens === 0)
        return I18n.tr(lang, "No data");
    return I18n.tr(lang, "{cost} · {tokens} tokens", {
        cost: usd(totals.costMicros),
        tokens: compactTokens(totals.totalTokens)
    });
}

function spendTooltip(lang, totals) {
    if (totals.totalTokens === 0)
        return "";
    const partial = totals.partial ? I18n.tr(lang, " · some models unpriced") : "";
    return `${exactUsd(totals.costMicros)} · ${tokenCount(lang, totals.totalTokens)}${partial}`;
}

function balanceValue(lang, balance) {
    if (balance.kind === "usd" && balance.usdMicros !== null)
        return usd(balance.usdMicros);
    if (balance.kind === "count" && balance.value !== null)
        return `${exactTokens(balance.value)} ${balance.unit ?? ""}`.trim();
    return I18n.tr(lang, "No data");
}

function isoDate(value) {
    const match = /^(\d{4})-(\d\d)-(\d\d)$/.exec(value);
    return match ? new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3])) : null;
}

function dayText(lang, value) {
    const date = isoDate(value);
    return date === null ? value : monthDay(lang, date);
}

function dayTooltip(lang, day) {
    if (day.date === "")
        return "";
    const date = dayText(lang, day.date);
    if (day.totalTokens === 0)
        return I18n.tr(lang, "{date} · no usage", {
            date
        });
    return I18n.tr(lang, "{date} · {tokens} tokens · {cost}", {
        date,
        tokens: compactTokens(day.totalTokens),
        cost: usd(day.costMicros)
    });
}
