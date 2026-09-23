.pragma library

.import "Format.js" as Format

const TREND_DAYS = 30;
const TREND_MIN_SHARE = 0.18;

function lastDays(daily) {
    const days = daily.slice(-TREND_DAYS);
    const padding = [];
    for (let index = days.length; index < TREND_DAYS; index++)
        padding.push({
            date: "",
            totalTokens: 0
        });
    return padding.concat(days);
}

function peakOf(days) {
    return days.reduce((peak, day) => Math.max(peak, day.totalTokens), 0);
}

function barShare(value, peak) {
    if (value <= 0 || peak <= 0)
        return 0;
    return Math.max(TREND_MIN_SHARE, value / peak);
}

function peakDescription(days) {
    const peakDay = days.reduce((best, day) => day.totalTokens > best.totalTokens ? day : best, days[0]);
    if (peakDay.totalTokens === 0)
        return "No usage in the last 30 days";
    return `Peak ${Format.compactTokens(peakDay.totalTokens)} tokens on ${peakDay.date}`;
}
