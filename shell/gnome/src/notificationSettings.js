import { isObject } from './fields.js';
import { flag } from './settingsValues.js';

export const MIN_THRESHOLD_PERCENT = 1;
export const MAX_THRESHOLD_PERCENT = 50;
export const THRESHOLD_CHOICES = [5, 10, 20, 30];
export const PROVIDER_THRESHOLD_OFF = 0;

const DEFAULT_THRESHOLD_PERCENT = 10;
const CLOCK = /^([01]\d|2[0-3]):[0-5]\d$/;
const DEFAULT_QUIET_HOURS = Object.freeze({ enabled: false, from: '22:00', to: '08:00', allowCritical: true });

export function isThreshold(value) {
    return Number.isInteger(value) && value >= MIN_THRESHOLD_PERCENT && value <= MAX_THRESHOLD_PERCENT;
}

export function isProviderThreshold(value) {
    return Number.isInteger(value) && value >= PROVIDER_THRESHOLD_OFF && value <= MAX_THRESHOLD_PERCENT;
}

export function isClockTime(value) {
    return typeof value === 'string' && CLOCK.test(value);
}

function parseProviderThresholds(raw) {
    if (!isObject(raw)) return {};
    const entries = Object.entries(raw).filter(
        ([provider, threshold]) => provider.length > 0 && isProviderThreshold(threshold)
    );
    return Object.fromEntries(entries);
}

function clockOr(value, fallback) {
    return isClockTime(value) ? value : fallback;
}

export function parseQuietHours(raw) {
    const quiet = isObject(raw) ? raw : {};
    const hours = {
        enabled: flag(quiet.enabled, DEFAULT_QUIET_HOURS.enabled),
        from: clockOr(quiet.from, DEFAULT_QUIET_HOURS.from),
        to: clockOr(quiet.to, DEFAULT_QUIET_HOURS.to),
        allowCritical: flag(quiet.allow_critical, DEFAULT_QUIET_HOURS.allowCritical),
    };
    if (hours.enabled && hours.from === hours.to) return { ...hours, enabled: false };
    return hours;
}

export function parseNotifications(raw) {
    const notifications = isObject(raw) ? raw : {};
    return {
        almostOut: flag(notifications.almost_out, true),
        cuttingItClose: flag(notifications.cutting_it_close, true),
        willRunOut: flag(notifications.will_run_out, true),
        reset: flag(notifications.reset, false),
        thresholdPercent: isThreshold(notifications.threshold_percent)
            ? notifications.threshold_percent
            : DEFAULT_THRESHOLD_PERCENT,
        providerThresholds: parseProviderThresholds(notifications.provider_thresholds),
        quietHours: parseQuietHours(notifications.quiet_hours),
    };
}
