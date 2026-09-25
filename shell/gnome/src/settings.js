import { parseDisplay } from './displaySettings.js';
import { isObject } from './fields.js';
import { parseNotifications } from './notificationSettings.js';
import { choice, flag, isAccelerator, LOG_LEVELS, nonEmpty, refreshInterval, SettingsError } from './settingsValues.js';

export {
    DENSITIES,
    LANGUAGES,
    MAX_PANEL_LIMITS,
    PANEL_BOXES,
    PANEL_INDICATORS,
    PANEL_LABELS,
    PANEL_MODES,
    parseDisplay,
    RESET_FORMATS,
    SPEND_BREAKDOWNS,
    SPEND_PERIODS,
    SPEND_UNITS,
    THEMES,
    TIME_FORMATS,
    VALUE_MODES,
} from './displaySettings.js';
export {
    MAX_THRESHOLD_PERCENT,
    MIN_THRESHOLD_PERCENT,
    PROVIDER_THRESHOLD_OFF,
    THRESHOLD_CHOICES,
} from './notificationSettings.js';
export { isReleaseAtLeast, needsOnboarding, supports06 } from './compat.js';
export * from './settingsPatches.js';

export {
    isAccelerator,
    LOG_LEVELS,
    MAX_REFRESH_SECS,
    MIN_REFRESH_SECS,
    SettingsError,
    SHORTCUT_MAX_CHARS,
} from './settingsValues.js';

function section(raw) {
    return isObject(raw) ? raw : {};
}

function parseHeadline(raw) {
    const headline = section(raw);
    const accountId = nonEmpty(headline.account_id);
    const window = nonEmpty(headline.window);
    if (headline.mode === 'pinned' && accountId && window) return { mode: 'pinned', accountId, window };
    return { mode: 'auto' };
}

function parseShortcuts(raw) {
    const open = section(raw).open;
    return { open: typeof open === 'string' && isAccelerator(open) ? open : '' };
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
        adaptiveRefresh: flag(raw.adaptive_refresh, true),
        notifications: parseNotifications(raw.notifications),
        headline: parseHeadline(raw.headline),
        reducedMotion: flag(raw.reduced_motion, false),
        display: parseDisplay(raw.display),
        updates: { check: flag(section(raw.updates).check, true) },
        statusPages: { enabled: flag(section(raw.status_pages).enabled, false) },
        shortcuts: parseShortcuts(raw.shortcuts),
        logging: { level: choice(LOG_LEVELS, section(raw.logging).level, 'info') },
        onboarding: { completed: flag(section(raw.onboarding).completed, false) },
    };
}

export function parseSettings(json) {
    return settingsFrom(decodeSettings(json));
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

export function isStarred(display, accountId) {
    return display.starredAccounts.includes(accountId);
}

export function starredAfter(display, accountId, starred) {
    const current = display.starredAccounts.filter(id => id !== accountId);
    return starred ? [...current, accountId] : current;
}
