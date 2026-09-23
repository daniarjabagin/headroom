import { _, fill } from './i18n.js';

const MINUTE = 60 * 1000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const WEEK_DAYS = 7;
const MICROS_PER_CENT = 10000;

const DASH = '—';

const UNITS = [
    [1e9, 'B'],
    [1e6, 'M'],
    [1e3, 'K'],
];

const SHORT_WINDOW_LABELS = { session: 'S', weekly: 'W' };

function roundPercent(value) {
    return Math.max(0, Math.round(value));
}

export function percentLeft(remainingPercent) {
    if (remainingPercent === null) return DASH;
    return fill(_('{percent}% left'), { percent: roundPercent(remainingPercent) });
}

export function percentUsed(usedPercent) {
    if (usedPercent === null) return DASH;
    return fill(_('{percent}% used'), { percent: roundPercent(usedPercent) });
}

export function reading(window, valueMode) {
    return valueMode === 'used' ? percentUsed(window.usedPercent) : percentLeft(window.remainingPercent);
}

export function panelPercent(percent) {
    return `${roundPercent(percent)}%`;
}

export function shortWindowLabel(windowId, windowLabel) {
    return SHORT_WINDOW_LABELS[windowId] ?? windowLabel ?? windowId ?? '';
}

export function duration(ms) {
    const days = Math.floor(ms / DAY);
    const hours = Math.floor((ms % DAY) / HOUR);
    const minutes = Math.floor((ms % HOUR) / MINUTE);
    if (days > 0) return fill(_('{days}d {hours}h'), { days, hours });
    if (hours > 0) return fill(_('{hours}h {minutes}m'), { hours, minutes });
    return fill(_('{minutes}m'), { minutes: Math.max(1, minutes) });
}

export function clockTime(date) {
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    return `${hours}:${minutes}`;
}

function startOfDay(date) {
    return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function calendarDaysBetween(from, to) {
    return Math.round((startOfDay(to) - startOfDay(from)) / DAY);
}

const WEEKDAYS = () => [_('Sun'), _('Mon'), _('Tue'), _('Wed'), _('Thu'), _('Fri'), _('Sat')];
const MONTHS = () => [
    _('Jan'),
    _('Feb'),
    _('Mar'),
    _('Apr'),
    _('May'),
    _('Jun'),
    _('Jul'),
    _('Aug'),
    _('Sep'),
    _('Oct'),
    _('Nov'),
    _('Dec'),
];

function exactMoment(date, now) {
    const time = clockTime(date);
    const days = calendarDaysBetween(now, date);
    if (days <= 0) return fill(_('today at {time}'), { time });
    if (days === 1) return fill(_('tomorrow at {time}'), { time });
    if (days < WEEK_DAYS) return fill(_('{day} at {time}'), { day: WEEKDAYS()[date.getDay()], time });
    const day = `${MONTHS()[date.getMonth()]} ${date.getDate()}`;
    return fill(_('{day} at {time}'), { day, time });
}

export function resetPhrase(resetsAt, now, resetFormat) {
    if (resetFormat === 'exact') return fill(_('resets {moment}'), { moment: exactMoment(resetsAt, now) });
    const left = resetsAt - now;
    if (left < MINUTE) return _('resets soon');
    return fill(_('resets in {duration}'), { duration: duration(left) });
}

function capitalized(text) {
    return text.charAt(0).toUpperCase() + text.slice(1);
}

export function resetText(resetsAt, now, resetFormat = 'countdown') {
    if (resetsAt === null) return _('Not started');
    return capitalized(resetPhrase(resetsAt, now, resetFormat));
}

export function spareText(sparePercent) {
    return fill(_('~{percent}% spare'), { percent: roundPercent(sparePercent) });
}

export function limitText(runsOutAt, now) {
    if (runsOutAt === null || runsOutAt <= now) return _('Limit soon');
    return fill(_('Limit in {duration}'), { duration: duration(runsOutAt - now) });
}

function runOutForecast(window, now, resetFormat) {
    const { runsOutAt } = window.pace;
    if (runsOutAt === null || runsOutAt <= now) return _('At this pace: runs out any minute');
    const runsOut = fill(_('runs out in {duration}'), { duration: duration(runsOutAt - now) });
    if (window.resetsAt === null) return fill(_('At this pace: {runsOut}'), { runsOut });
    const resets = resetPhrase(window.resetsAt, now, resetFormat);
    return fill(_('At this pace: {runsOut} · {resets}'), { runsOut, resets });
}

function atResetForecast(pace, valueMode) {
    if (valueMode === 'used' && pace.projectedPercent !== null)
        return fill(_('At this pace: ~{percent}% used at reset'), { percent: roundPercent(pace.projectedPercent) });
    return fill(_('At this pace: ~{percent}% left at reset'), { percent: roundPercent(pace.sparePercent) });
}

export function forecastText(window, now, display) {
    const { severity, sparePercent } = window.pace;
    if (window.remainingPercent === null) return null;
    if (severity === 'running_out') return runOutForecast(window, now, display.resetFormat);
    if ((severity === 'healthy' || severity === 'close') && sparePercent !== null)
        return atResetForecast(window.pace, display.valueMode);
    return null;
}

export function nextUpdateText(nextRefreshAt, now) {
    const left = nextRefreshAt - now;
    if (left < MINUTE) return _('Next update in <1m');
    return fill(_('Next update in {duration}'), { duration: duration(left) });
}

export function agoText(date, now) {
    const elapsed = now - date;
    if (elapsed < MINUTE) return _('just now');
    return fill(_('{duration} ago'), { duration: duration(elapsed) });
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
    if (totals.costMicros === 0 && totals.totalTokens === 0) return _('No data');
    return fill(_('{cost} · {tokens} tokens'), {
        cost: usd(totals.costMicros),
        tokens: compactTokens(totals.totalTokens),
    });
}
