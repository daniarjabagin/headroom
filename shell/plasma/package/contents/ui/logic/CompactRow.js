.pragma library

.import "FormatTime.js" as FormatTime
.import "I18n.js" as I18n
.import "Quota.js" as Quota

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const WEEK_DAYS = 7;
const WEEKDAYS = [I18n.N("Sun"), I18n.N("Mon"), I18n.N("Tue"), I18n.N("Wed"), I18n.N("Thu"), I18n.N("Fri"), I18n.N("Sat")];

function exactShort(lang, resetsAt, now, timeFormat) {
    const time = FormatTime.clockTime(resetsAt, timeFormat);
    const days = FormatTime.calendarDaysBetween(now, resetsAt);
    if (days <= 0)
        return time;
    const day = days < WEEK_DAYS ? I18n.tr(lang, WEEKDAYS[resetsAt.getDay()]) : FormatTime.monthDay(lang, resetsAt);
    return I18n.tr(lang, "{day} {time}", {
        day,
        time
    });
}

function countdownShort(lang, resetsAt, now, live) {
    const left = resetsAt - now;
    if (left < (live === true ? SECOND : MINUTE))
        return I18n.tr(lang, "soon");
    return FormatTime.duration(lang, left, live);
}

function trailing(lang, window, now, resetFormat, live, timeFormat) {
    if (!Quota.hasData(window))
        return I18n.tr(lang, "No data");
    if (window.resetsAt === null)
        return I18n.tr(lang, "Not started");
    if (window.resetsAt <= now)
        return I18n.tr(lang, "reset pending");
    if (resetFormat === "exact")
        return exactShort(lang, window.resetsAt, now, timeFormat);
    return countdownShort(lang, window.resetsAt, now, live);
}

function meterTip(lang, window, now, display) {
    const note = Quota.paceNote(lang, window, now, display.showForecast);
    const lines = [Quota.trailingText(lang, window, now, display.resetFormat, false, display.timeFormat), note?.text ?? null, Quota.forecast(lang, window, now, display)];
    return lines.filter(line => line !== null && line !== "").join("\n");
}

function valueHint(lang, valueMode) {
    return valueMode === "used" ? I18n.tr(lang, "Click to show what is left") : I18n.tr(lang, "Click to show used");
}

function resetHint(lang, resetFormat) {
    return resetFormat === "exact" ? I18n.tr(lang, "Click to show the countdown") : I18n.tr(lang, "Click to show the reset time");
}
