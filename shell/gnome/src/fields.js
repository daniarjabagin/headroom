export const TONES = new Set(['good', 'warning', 'critical', 'neutral']);

const HTTPS_URL = /^https:\/\/\S+$/;

export function isObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export function text(value) {
    return typeof value === 'string' && value.length > 0 ? value : null;
}

export function number(value) {
    return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

export function count(value) {
    return number(value) ?? 0;
}

export function integer(value) {
    return Number.isInteger(value) ? value : null;
}

export function list(value) {
    return Array.isArray(value) ? value.filter(isObject) : [];
}

export function timestamp(value) {
    const ms = typeof value === 'string' ? Date.parse(value) : NaN;
    return Number.isNaN(ms) ? null : new Date(ms);
}

export function oneOf(allowed, value, fallback) {
    return allowed.has(value) ? value : fallback;
}

export function httpsUrl(value) {
    return typeof value === 'string' && HTTPS_URL.test(value) ? value : null;
}

export function providerOf(raw) {
    const provider = text(raw.provider) ?? 'unknown';
    return { provider, providerName: text(raw.provider_name) ?? provider };
}
