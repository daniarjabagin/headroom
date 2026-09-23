.pragma library

.import "Format.js" as Format

function hasData(window) {
    return window.remainingPercent !== null;
}

function fillFraction(window) {
    if (!hasData(window))
        return 0;
    return Math.min(1, Math.max(0, window.remainingPercent / 100));
}

function meterTone(window) {
    return hasData(window) ? window.tone : "neutral";
}

function tickPosition(window, alwaysShowPacing) {
    const even = window.pace.evenPacePercent;
    if (even === null || !hasData(window))
        return null;
    if (!alwaysShowPacing && window.tone !== "warning" && window.tone !== "critical")
        return null;
    return Math.min(1, Math.max(0, 1 - even / 100));
}

function paceNote(window, now, alwaysShowPacing) {
    const pace = window.pace;
    if (pace.severity === "spent")
        return {
            flame: true,
            text: "Limit reached"
        };
    if (pace.severity === "running_out")
        return {
            flame: true,
            text: Format.limitText(pace.runsOutAt, now)
        };
    if (pace.severity === "close" && pace.sparePercent !== null)
        return {
            flame: false,
            text: Format.spareText(pace.sparePercent)
        };
    if (alwaysShowPacing && pace.severity === "healthy" && pace.sparePercent !== null)
        return {
            flame: false,
            text: Format.leftAtResetText(pace.sparePercent)
        };
    return null;
}

function trailingText(window, now) {
    if (!hasData(window))
        return "No data";
    return Format.resetText(window.resetsAt, now);
}
