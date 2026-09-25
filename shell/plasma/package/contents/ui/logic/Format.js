.pragma library

.import "DaemonText.js" as DaemonText
.import "FormatTime.js" as FormatTime
.import "I18n.js" as I18n

const DASH = "—";

const SHORT_WINDOW_LABELS = {
    session: I18n.N("S"),
    weekly: I18n.N("W")
};

const WINDOW_LABELS = {
    session: I18n.N("Session"),
    weekly: I18n.N("Weekly")
};

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
    return DaemonText.label(lang, windowLabel) ?? windowId ?? "";
}

function windowLabel(lang, window) {
    const known = WINDOW_LABELS[window.id];
    return known ? I18n.tr(lang, known) : DaemonText.label(lang, window.label);
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
        duration: FormatTime.duration(lang, runsOutAt - now)
    });
}

function runOutForecast(lang, window, now, resetFormat, timeFormat) {
    const runsOutAt = window.pace.runsOutAt;
    if (runsOutAt === null && isPooled(window))
        return I18n.tr(lang, "At this pace: runs out before reset");
    if (runsOutAt === null || runsOutAt <= now)
        return I18n.tr(lang, "At this pace: runs out any minute");
    const runsOut = I18n.tr(lang, "runs out in {duration}", {
        duration: FormatTime.duration(lang, runsOutAt - now)
    });
    if (window.resetsAt === null)
        return I18n.tr(lang, "At this pace: {runsOut}", {
            runsOut
        });
    return I18n.tr(lang, "At this pace: {runsOut} · {resets}", {
        runsOut,
        resets: FormatTime.resetPhrase(lang, window.resetsAt, now, resetFormat, false, timeFormat)
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
        return runOutForecast(lang, window, now, display.resetFormat, display.timeFormat);
    if ((pace.severity === "healthy" || pace.severity === "close") && pace.sparePercent !== null)
        return atResetForecast(lang, window, display.valueMode);
    return null;
}
