.pragma library

.import "FormatSpend.js" as FormatSpend
.import "FormatTime.js" as FormatTime
.import "I18n.js" as I18n

const TREND_DAYS = 30;
const TREND_MIN_SHARE = 0.18;

function lastDays(daily) {
    const days = daily.slice(-TREND_DAYS);
    const padding = [];
    for (let index = days.length; index < TREND_DAYS; index++)
        padding.push({
            date: "",
            totalTokens: 0,
            costMicros: 0,
            partial: false
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

function peakDescription(lang, days) {
    const peakDay = days.reduce((best, day) => day.totalTokens > best.totalTokens ? day : best, days[0]);
    if (peakDay.totalTokens === 0)
        return I18n.tr(lang, "No usage in the last 30 days");
    return I18n.tr(lang, "Peak {tokens} tokens on {date}", {
        tokens: FormatSpend.compactTokens(peakDay.totalTokens),
        date: FormatTime.dayText(lang, peakDay.date)
    });
}

function indexAt(x, step, count) {
    if (step <= 0 || x < 0)
        return -1;
    const index = Math.floor(x / step);
    return index < count ? index : -1;
}
