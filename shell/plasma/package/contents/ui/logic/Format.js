.pragma library

const MINUTE = 60 * 1000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const MICROS_PER_CENT = 10000;

const DASH = "—";

const UNITS = [[1e9, "B"], [1e6, "M"], [1e3, "K"]];

function roundPercent(value) {
    return Math.max(0, Math.round(value));
}

function percentLeft(remainingPercent) {
    if (remainingPercent === null)
        return DASH;
    return `${roundPercent(remainingPercent)}% left`;
}

function panelPercent(remainingPercent) {
    return `${roundPercent(remainingPercent)}%`;
}

function duration(ms) {
    const days = Math.floor(ms / DAY);
    const hours = Math.floor((ms % DAY) / HOUR);
    const minutes = Math.floor((ms % HOUR) / MINUTE);
    if (days > 0)
        return `${days}d ${hours}h`;
    if (hours > 0)
        return `${hours}h ${minutes}m`;
    return `${Math.max(1, minutes)}m`;
}

function resetText(resetsAt, now) {
    if (resetsAt === null)
        return "Not started";
    const left = resetsAt - now;
    if (left < MINUTE)
        return "Resets soon";
    return `Resets in ${duration(left)}`;
}

function spareText(sparePercent) {
    return `~${roundPercent(sparePercent)}% spare`;
}

function leftAtResetText(sparePercent) {
    return `~${roundPercent(sparePercent)}% left at reset`;
}

function limitText(runsOutAt, now) {
    if (runsOutAt === null || runsOutAt <= now)
        return "Limit soon";
    return `Limit in ${duration(runsOutAt - now)}`;
}

function nextUpdateText(nextRefreshAt, now) {
    const left = nextRefreshAt - now;
    if (left < MINUTE)
        return "Next update in <1m";
    return `Next update in ${duration(left)}`;
}

function agoText(date, now) {
    const elapsed = now - date;
    if (elapsed < MINUTE)
        return "just now";
    return `${duration(elapsed)} ago`;
}

function clockTime(date) {
    const hours = String(date.getHours()).padStart(2, "0");
    const minutes = String(date.getMinutes()).padStart(2, "0");
    return `${hours}:${minutes}`;
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

function spendLine(totals) {
    if (totals.costMicros === 0 && totals.totalTokens === 0)
        return "No data";
    return `${usd(totals.costMicros)} · ${compactTokens(totals.totalTokens)} tokens`;
}

function spendTooltip(totals) {
    if (totals.totalTokens === 0)
        return "";
    const partial = totals.partial ? " · some models unpriced" : "";
    return `${exactUsd(totals.costMicros)} · ${exactTokens(totals.totalTokens)} tokens${partial}`;
}

function balanceValue(balance) {
    if (balance.kind === "usd" && balance.usdMicros !== null)
        return usd(balance.usdMicros);
    if (balance.kind === "count" && balance.value !== null)
        return `${exactTokens(balance.value)} ${balance.unit ?? ""}`.trim();
    return "No data";
}
