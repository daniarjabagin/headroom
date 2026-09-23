import { exactMoment } from './dates.js';
import { _, fill } from './i18n.js';

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

const DASH = '—';

const WINDOW_LABELS = {
    session: () => _('Session'),
    weekly: () => _('Weekly'),
};

const SHORT_WINDOW_LABELS = {
    session: () => _('S'),
    weekly: () => _('W'),
};

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

export function readingPercent(window, valueMode) {
    if (valueMode === 'used') return window.usedPercent ?? 100 - window.remainingPercent;
    return window.remainingPercent;
}

export function percentReading(percent, valueMode) {
    return valueMode === 'used' ? percentUsed(percent) : percentLeft(percent);
}

export function panelPercent(percent) {
    return `${roundPercent(percent)}%`;
}

export function windowLabel(windowId, label) {
    return WINDOW_LABELS[windowId]?.() ?? label ?? windowId ?? '';
}

export function shortWindowLabel(windowId, label) {
    return SHORT_WINDOW_LABELS[windowId]?.() ?? label ?? windowId ?? '';
}

function twoDigits(value) {
    return String(value).padStart(2, '0');
}

function shortDuration(ms) {
    const minutes = Math.floor(ms / MINUTE);
    const seconds = Math.floor((ms % MINUTE) / SECOND);
    if (minutes === 0) return fill(_('{seconds}s'), { seconds: Math.max(1, seconds) });
    return fill(_('{minutes}m {seconds}s'), { minutes, seconds: twoDigits(seconds) });
}

export function duration(ms, withSeconds = false) {
    if (withSeconds && ms < HOUR) return shortDuration(ms);
    const days = Math.floor(ms / DAY);
    const hours = Math.floor((ms % DAY) / HOUR);
    const minutes = Math.floor((ms % HOUR) / MINUTE);
    if (days > 0) return fill(_('{days}d {hours}h'), { days, hours });
    if (hours > 0) return fill(_('{hours}h {minutes}m'), { hours, minutes });
    return fill(_('{minutes}m'), { minutes: Math.max(1, minutes) });
}

export function isCountdownLive(resetsAt, now) {
    return resetsAt !== null && resetsAt > now && resetsAt - now < HOUR;
}

export function resetPhrase(resetsAt, now, resetFormat, withSeconds = false) {
    if (resetsAt <= now) return _('reset pending');
    if (resetFormat === 'exact') return fill(_('resets {moment}'), { moment: exactMoment(resetsAt, now) });
    const left = resetsAt - now;
    if (left < (withSeconds ? SECOND : MINUTE)) return _('resets soon');
    return fill(_('resets in {duration}'), { duration: duration(left, withSeconds) });
}

function capitalized(text) {
    return text.charAt(0).toUpperCase() + text.slice(1);
}

export function resetText(resetsAt, now, resetFormat = 'countdown', withSeconds = false) {
    if (resetsAt === null) return _('Not started');
    return capitalized(resetPhrase(resetsAt, now, resetFormat, withSeconds));
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
