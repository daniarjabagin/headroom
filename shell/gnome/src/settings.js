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
        translucent: flag(display.translucent, false),
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

function parseUpdates(raw) {
    const updates = isObject(raw) ? raw : {};
    return { check: flag(updates.check, true) };
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

export function decodeSettings(json) {
    const raw = decode(json);
    if (!isObject(raw)) throw new SettingsError('Unexpected settings from the Headroom service');
    return raw;
}

export function settingsFrom(raw) {
    return {
        refreshIntervalSecs: refreshInterval(raw.refresh_interval_secs),
        notifications: parseNotifications(raw.notifications),
        headline: parseHeadline(raw.headline),
        reducedMotion: flag(raw.reduced_motion, false),
        display: parseDisplay(raw.display),
        updates: parseUpdates(raw.updates),
    };
}

export function parseSettings(json) {
    return settingsFrom(decodeSettings(json));
}

const DISPLAY_FIELDS = {
    theme: 'theme',
    language: 'language',
    valueMode: 'value_mode',
    resetFormat: 'reset_format',
    panelLabel: 'panel_label',
    showSpend: 'show_spend',
    showAccountSpend: 'show_account_spend',
    showTrend: 'show_trend',
    showForecast: 'show_forecast',
    translucent: 'translucent',
};

const NOTIFICATION_FIELDS = {
    almostOut: 'almost_out',
    cuttingItClose: 'cutting_it_close',
    willRunOut: 'will_run_out',
    reset: 'reset',
};

function renamed(fields, changes) {
    return Object.fromEntries(
        Object.entries(changes).map(([key, value]) => {
            if (!(key in fields)) throw new SettingsError(`Unknown setting ${key}`);
            return [fields[key], value];
        })
    );
}

export function displayPatch(changes) {
    return { display: renamed(DISPLAY_FIELDS, changes) };
}

export function notificationsPatch(changes) {
    return { notifications: renamed(NOTIFICATION_FIELDS, changes) };
}

export function refreshIntervalPatch(secs) {
    return { refresh_interval_secs: refreshInterval(secs) };
}

export function updatesPatch(check) {
    return { updates: { check: check === true } };
}

export function headlinePatch(headline) {
    if (headline.mode !== 'pinned') return { headline: { mode: 'auto', account_id: null, window: null } };
    return { headline: { mode: 'pinned', account_id: headline.accountId, window: headline.window } };
}

export function hiddenWindowsPatch(accountId, windows) {
    return { display: { hidden_windows: { [accountId]: windows.length > 0 ? windows : null } } };
}

export function mergePatch(target, patch) {
    if (!isObject(patch)) return patch;
    const merged = isObject(target) ? { ...target } : {};
    for (const [key, value] of Object.entries(patch)) {
        if (value === null) delete merged[key];
        else merged[key] = mergePatch(merged[key], value);
    }
    return merged;
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

export function hiddenWindowsAfter(display, accountId, windowId, hidden) {
    const current = (display.hiddenWindows[accountId] ?? []).filter(id => id !== windowId);
    return hidden ? [...current, windowId] : current;
}
