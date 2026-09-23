export const THEMES = ['system', 'light', 'dark'];
export const VALUE_MODES = ['left', 'used'];
export const RESET_FORMATS = ['countdown', 'exact'];
export const PANEL_LABELS = ['percent', 'window'];
export const LANGUAGES = ['system', 'en', 'ru'];
export const MIN_REFRESH_SECS = 60;
export const MAX_REFRESH_SECS = 3600;

const DEFAULT_REFRESH_SECS = 300;

export class SettingsError extends Error {}

function isObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function flag(value, fallback) {
    return typeof value === 'boolean' ? value : fallback;
}

function choice(allowed, value, fallback) {
    return allowed.includes(value) ? value : fallback;
}

function nonEmpty(value) {
    return typeof value === 'string' && value.length > 0 ? value : null;
}

function refreshInterval(value) {
    if (!Number.isInteger(value)) return DEFAULT_REFRESH_SECS;
    return Math.min(MAX_REFRESH_SECS, Math.max(MIN_REFRESH_SECS, value));
}

function parseHiddenWindows(raw) {
    if (!isObject(raw)) return {};
    const entries = Object.entries(raw)
        .map(([accountId, windows]) => [accountId, Array.isArray(windows) ? windows.filter(nonEmpty) : []])
        .filter(([, windows]) => windows.length > 0)
        .map(([accountId, windows]) => [accountId, [...new Set(windows)]]);
    return Object.fromEntries(entries);
}

export function parseDisplay(raw) {
    const display = isObject(raw) ? raw : {};
    return {
        theme: choice(THEMES, display.theme, 'system'),
        language: choice(LANGUAGES, display.language, 'system'),
        valueMode: choice(VALUE_MODES, display.value_mode, 'left'),
        resetFormat: choice(RESET_FORMATS, display.reset_format, 'countdown'),
        panelLabel: choice(PANEL_LABELS, display.panel_label, 'percent'),
        showSpend: flag(display.show_spend, true),
        showAccountSpend: flag(display.show_account_spend, true),
        showTrend: flag(display.show_trend, true),
        showForecast: flag(display.show_forecast, true),
        hiddenWindows: parseHiddenWindows(display.hidden_windows),
    };
}

function parseNotifications(raw) {
    const notifications = isObject(raw) ? raw : {};
    return {
        almostOut: flag(notifications.almost_out, true),
        cuttingItClose: flag(notifications.cutting_it_close, true),
        willRunOut: flag(notifications.will_run_out, true),
        reset: flag(notifications.reset, false),
    };
}

function parseHeadline(raw) {
    const headline = isObject(raw) ? raw : {};
    const accountId = nonEmpty(headline.account_id);
    const window = nonEmpty(headline.window);
    if (headline.mode === 'pinned' && accountId && window) return { mode: 'pinned', accountId, window };
    return { mode: 'auto' };
}

function decode(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        throw new SettingsError(`Unreadable settings from the Headroom service: ${error.message}`);
    }
}

export function parseSettings(json) {
    const raw = decode(json);
    if (!isObject(raw)) throw new SettingsError('Unexpected settings from the Headroom service');
    return {
        refreshIntervalSecs: refreshInterval(raw.refresh_interval_secs),
        notifications: parseNotifications(raw.notifications),
        headline: parseHeadline(raw.headline),
        reducedMotion: flag(raw.reduced_motion, false),
        display: parseDisplay(raw.display),
    };
}

function serializeHeadline(headline) {
    if (headline.mode !== 'pinned') return { mode: 'auto' };
    return { mode: 'pinned', account_id: headline.accountId, window: headline.window };
}

export function serializeDisplay(display) {
    return {
        theme: display.theme,
        language: display.language,
        value_mode: display.valueMode,
        reset_format: display.resetFormat,
        panel_label: display.panelLabel,
        show_spend: display.showSpend,
        show_account_spend: display.showAccountSpend,
        show_trend: display.showTrend,
        show_forecast: display.showForecast,
        hidden_windows: parseHiddenWindows(display.hiddenWindows),
    };
}

export function serializeSettings(settings) {
    const { notifications } = settings;
    return JSON.stringify({
        refresh_interval_secs: refreshInterval(settings.refreshIntervalSecs),
        notifications: {
            almost_out: notifications.almostOut,
            cutting_it_close: notifications.cuttingItClose,
            will_run_out: notifications.willRunOut,
            reset: notifications.reset,
        },
        headline: serializeHeadline(settings.headline),
        reduced_motion: settings.reducedMotion,
        display: serializeDisplay(settings.display),
    });
}

export function withDisplay(settings, patch) {
    return { ...settings, display: { ...settings.display, ...patch } };
}

export function withNotifications(settings, patch) {
    return { ...settings, notifications: { ...settings.notifications, ...patch } };
}

export function toggledValueMode(display) {
    return { valueMode: display.valueMode === 'used' ? 'left' : 'used' };
}

export function toggledResetFormat(display) {
    return { resetFormat: display.resetFormat === 'exact' ? 'countdown' : 'exact' };
}

export function isWindowHidden(display, accountId, windowId) {
    return (display.hiddenWindows[accountId] ?? []).includes(windowId);
}

export function withWindowHidden(display, accountId, windowId, hidden) {
    const current = (display.hiddenWindows[accountId] ?? []).filter(id => id !== windowId);
    const next = hidden ? [...current, windowId] : current;
    const hiddenWindows = { ...display.hiddenWindows, [accountId]: next };
    if (next.length === 0) delete hiddenWindows[accountId];
    return { hiddenWindows };
}
