import { clockTime } from './dates.js';
import { agoText } from './format.js';
import { _, fill } from './i18n.js';

export const CHECK_TIMEOUT_MS = 90_000;

const STATUSES = new Set(['up_to_date', 'available', 'failed', 'rate_limited', 'disabled']);
const HIDING_STATUSES = new Set(['unsupported', 'disabled']);
const UNSUPPORTED_ERRORS = new Set([
    'org.freedesktop.DBus.Error.UnknownMethod',
    'org.freedesktop.DBus.Error.NotSupported',
]);
const SEPARATOR = ' · ';
const HIDDEN = Object.freeze({ visible: false });

function isObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function text(value) {
    return typeof value === 'string' && value.length > 0 ? value : null;
}

function timestamp(value) {
    const ms = typeof value === 'string' ? Date.parse(value) : NaN;
    return Number.isNaN(ms) ? null : new Date(ms);
}

function result(status, raw = {}) {
    return {
        status,
        checkedAt: timestamp(raw.checked_at),
        version: text(raw.version),
        until: timestamp(raw.until),
    };
}

function decode(json) {
    try {
        return JSON.parse(json);
    } catch {
        return null;
    }
}

export function parseUpdateCheck(raw) {
    if (!isObject(raw)) return null;
    return { checkedAt: timestamp(raw.checked_at) };
}

export function parseCheckResult(json) {
    const raw = decode(json);
    if (!isObject(raw) || !STATUSES.has(raw.status)) return result('failed');
    return result(raw.status, raw);
}

export function failedCheck(remoteErrorName) {
    return result(UNSUPPORTED_ERRORS.has(remoteErrorName) ? 'unsupported' : 'failed');
}

function installedText(appVersion) {
    return appVersion ? fill(_('Headroom {version}'), { version: appVersion }) : 'Headroom';
}

function checkedText(checkedAt, now) {
    return checkedAt ? fill(_('checked {ago}'), { ago: agoText(checkedAt, now) }) : null;
}

function joined(...parts) {
    return parts.filter(part => part).join(SEPARATOR);
}

function holdsRateLimit(outcome, now) {
    return outcome?.status === 'rate_limited' && (outcome.until === null || outcome.until > now);
}

function rateLimitText(until) {
    return until ? fill(_('Try again at {time}'), { time: clockTime(until) }) : _('Try again later');
}

function wording({ appVersion, checkedAt, result: outcome, checking, now }) {
    const installed = installedText(appVersion);
    const checked = checkedText(checkedAt, now);
    if (checking) return { title: _('Checking for updates…'), subtitle: joined(installed, checked) };
    if (outcome?.status === 'failed')
        return { title: _("Couldn't check for updates"), subtitle: joined(installed, checked), retry: true };
    if (holdsRateLimit(outcome, now)) return { title: _('GitHub rate limit'), subtitle: rateLimitText(outcome.until) };
    if (outcome?.status === 'available' && outcome.version)
        return {
            title: fill(_('Headroom {version} is available'), { version: outcome.version }),
            subtitle: joined(installed, checked),
        };
    if (!checkedAt) return { title: _('Not checked yet'), subtitle: installed };
    return { title: _("You're up to date"), subtitle: joined(installed, checked) };
}

export function checkRow({ enabled, appVersion, update, updateCheck, result: outcome, checking, now }) {
    if (!enabled || updateCheck === null || update !== null) return HIDDEN;
    if (HIDING_STATUSES.has(outcome?.status)) return HIDDEN;
    const checkedAt = updateCheck.checkedAt ?? outcome?.checkedAt ?? null;
    const words = wording({ appVersion, checkedAt, result: outcome, checking, now });
    return {
        visible: true,
        title: words.title,
        subtitle: words.subtitle,
        busy: checking,
        action: words.retry ? _('Try again') : _('Check now'),
    };
}
