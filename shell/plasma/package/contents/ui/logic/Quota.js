.pragma library

.import "Format.js" as Format
.import "I18n.js" as I18n

function hasData(window) {
    return window.remainingPercent !== null;
}

function usedOf(window) {
    return window.usedPercent ?? 100 - window.remainingPercent;
}

function shownPercent(window, valueMode) {
    if (!hasData(window))
        return null;
    return valueMode === "used" ? usedOf(window) : window.remainingPercent;
}

function fillFraction(window, valueMode) {
    if (!hasData(window))
        return 0;
    return Math.min(1, Math.max(0, shownPercent(window, valueMode) / 100));
}

function meterTone(window) {
    return hasData(window) ? window.tone : "neutral";
}

function tickPosition(window, display) {
    const even = window.pace.evenPacePercent;
    if (even === null || !hasData(window))
        return null;
    if (!display.showForecast && window.tone !== "warning" && window.tone !== "critical")
        return null;
    const position = display.valueMode === "used" ? even / 100 : 1 - even / 100;
    return Math.min(1, Math.max(0, position));
}

function paceNote(lang, window, now, showForecast) {
    const pace = window.pace;
    if (pace.severity === "spent")
        return {
            flame: true,
            text: I18n.tr(lang, "Limit reached")
        };
    if (pace.severity === "running_out")
        return {
            flame: true,
            text: showForecast ? I18n.tr(lang, "Over pace") : Format.limitText(lang, pace.runsOutAt, now)
        };
    if (pace.severity === "close" && pace.sparePercent !== null && !showForecast)
        return {
            flame: false,
            text: Format.spareText(lang, pace.sparePercent)
        };
    return null;
}

function trailingText(lang, window, now, resetFormat, live) {
    if (!hasData(window))
        return I18n.tr(lang, "No data");
    return Format.resetText(lang, window.resetsAt, now, resetFormat, live);
}

function forecast(lang, window, now, display) {
    if (!display.showForecast)
        return null;
    return Format.forecastText(lang, window, now, display);
}
