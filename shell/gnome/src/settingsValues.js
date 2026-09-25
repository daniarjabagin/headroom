export const MIN_REFRESH_SECS = 60;
export const MAX_REFRESH_SECS = 3600;
export const LOG_LEVELS = ['error', 'warn', 'info', 'debug'];
export const SHORTCUT_MAX_CHARS = 64;

const DEFAULT_REFRESH_SECS = 300;
const ACCELERATOR = /^(<[A-Za-z]+>)*[A-Za-z0-9_]+$/;

export class SettingsError extends Error {}

export function flag(value, fallback) {
    return typeof value === 'boolean' ? value : fallback;
}

export function choice(allowed, value, fallback) {
    return allowed.includes(value) ? value : fallback;
}

export function nonEmpty(value) {
    return typeof value === 'string' && value.length > 0 ? value : null;
}

export function distinctIds(raw) {
    return [...new Set(Array.isArray(raw) ? raw.filter(nonEmpty) : [])];
}

export function refreshInterval(value) {
    if (!Number.isInteger(value)) return DEFAULT_REFRESH_SECS;
    return Math.min(MAX_REFRESH_SECS, Math.max(MIN_REFRESH_SECS, value));
}

export function isAccelerator(value) {
    if (value === '') return true;
    return typeof value === 'string' && value.length <= SHORTCUT_MAX_CHARS && ACCELERATOR.test(value);
}
