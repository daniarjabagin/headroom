const MINUTE = 60 * 1000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const MICROS_PER_CENT = 10000;

const DASH = '—';

const UNITS = [
    [1e9, 'B'],
    [1e6, 'M'],
    [1e3, 'K'],
];

function roundPercent(value) {
    return Math.max(0, Math.round(value));
}

export function percentLeft(remainingPercent) {
    if (remainingPercent === null) return DASH;
    return `${roundPercent(remainingPercent)}% left`;
}

export function panelPercent(remainingPercent) {
    return `${roundPercent(remainingPercent)}%`;
}

export function duration(ms) {
    const days = Math.floor(ms / DAY);
    const hours = Math.floor((ms % DAY) / HOUR);
    const minutes = Math.floor((ms % HOUR) / MINUTE);
    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h ${minutes}m`;
    return `${Math.max(1, minutes)}m`;
}

export function resetText(resetsAt, now) {
    if (resetsAt === null) return 'Not started';
    const left = resetsAt - now;
    if (left < MINUTE) return 'Resets soon';
    return `Resets in ${duration(left)}`;
}

export function spareText(sparePercent) {
    return `~${roundPercent(sparePercent)}% spare`;
}

export function leftAtResetText(sparePercent) {
    return `~${roundPercent(sparePercent)}% left at reset`;
}

export function limitText(runsOutAt, now) {
    if (runsOutAt === null || runsOutAt <= now) return 'Limit soon';
    return `Limit in ${duration(runsOutAt - now)}`;
}

export function nextUpdateText(nextRefreshAt, now) {
    const left = nextRefreshAt - now;
    if (left < MINUTE) return 'Next update in <1m';
    return `Next update in ${duration(left)}`;
}

export function agoText(date, now) {
    const elapsed = now - date;
    if (elapsed < MINUTE) return 'just now';
    return `${duration(elapsed)} ago`;
}

export function clockTime(date) {
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    return `${hours}:${minutes}`;
}

const tokenDigits = scaled => (scaled >= 100 ? 0 : 1);
const moneyDigits = scaled => (scaled >= 100 ? 0 : scaled >= 10 ? 1 : 2);

function abbreviate(value, digitsFor) {
    for (const [size, suffix] of UNITS) {
        if (value >= size) {
            const scaled = value / size;
            const text = scaled.toFixed(digitsFor(scaled));
            return `${text.replace(/\.0+$/, '')}${suffix}`;
        }
    }
    return String(value);
}

export function compactTokens(count) {
    return abbreviate(count, tokenDigits);
}

export function exactTokens(count) {
    return String(count).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

function centsOf(micros) {
    return Math.round(micros / MICROS_PER_CENT);
}

export function exactUsd(micros) {
    const cents = centsOf(micros);
    const rest = String(Math.abs(cents % 100)).padStart(2, '0');
    return `$${exactTokens(Math.trunc(cents / 100))}.${rest}`;
}

export function usd(micros) {
    const cents = centsOf(micros);
    if (cents >= 100000) return `$${abbreviate(cents / 100, moneyDigits)}`;
    return exactUsd(micros);
}

export function ringUsd(micros) {
    const cents = centsOf(micros);
    if (cents < 10000) return usd(micros);
    if (cents < 1000000) return `$${Math.round(cents / 100)}`;
    return usd(micros);
}

export function spendLine(totals) {
    if (totals.costMicros === 0 && totals.totalTokens === 0) return 'No data';
    return `${usd(totals.costMicros)} · ${compactTokens(totals.totalTokens)} tokens`;
}
