import * as display from './displaySettings.js';
import { isObject } from './fields.js';
import { isClockTime, isProviderThreshold, isThreshold } from './notificationSettings.js';
import { distinctIds, isAccelerator, LOG_LEVELS, nonEmpty, refreshInterval, SettingsError } from './settingsValues.js';

const isBool = value => typeof value === 'boolean';
const among = allowed => value => allowed.includes(value);

const DISPLAY_FIELDS = {
    theme: ['theme', among(display.THEMES)],
    language: ['language', among(display.LANGUAGES)],
    valueMode: ['value_mode', among(display.VALUE_MODES)],
    resetFormat: ['reset_format', among(display.RESET_FORMATS)],
    panelLabel: ['panel_label', among(display.PANEL_LABELS)],
    showSpend: ['show_spend', isBool],
    showAccountSpend: ['show_account_spend', isBool],
    showTrend: ['show_trend', isBool],
    showForecast: ['show_forecast', isBool],
    translucent: ['translucent', isBool],
    combineAccounts: ['combine_accounts', isBool],
    density: ['density', among(display.DENSITIES)],
    timeFormat: ['time_format', among(display.TIME_FORMATS)],
    panelMode: ['panel_mode', among(display.PANEL_MODES)],
    panelIndicator: ['panel_indicator', among(display.PANEL_INDICATORS)],
    spendPeriod: ['spend_period', among(display.SPEND_PERIODS)],
    spendUnit: ['spend_unit', among(display.SPEND_UNITS)],
    spendBreakdown: ['spend_breakdown', among(display.SPEND_BREAKDOWNS)],
    showBreakdown: ['show_breakdown', isBool],
    collapseUnstarred: ['collapse_unstarred', isBool],
    hideOnScreenShare: ['hide_on_screen_share', isBool],
};

const NOTIFICATION_FIELDS = {
    almostOut: ['almost_out', isBool],
    cuttingItClose: ['cutting_it_close', isBool],
    willRunOut: ['will_run_out', isBool],
    reset: ['reset', isBool],
    thresholdPercent: ['threshold_percent', isThreshold],
};

const QUIET_HOURS_FIELDS = {
    enabled: ['enabled', isBool],
    from: ['from', isClockTime],
    to: ['to', isClockTime],
    allowCritical: ['allow_critical', isBool],
};

const SINCE_06 = {
    top: ['adaptive_refresh', 'status_pages', 'shortcuts', 'logging', 'onboarding'],
    notifications: ['threshold_percent', 'provider_thresholds', 'quiet_hours'],
    display: [
        'density',
        'time_format',
        'panel_mode',
        'panel_indicator',
        'panel_limits',
        'panel_position',
        'spend_period',
        'spend_unit',
        'spend_breakdown',
        'starred_accounts',
        'collapse_unstarred',
        'hide_on_screen_share',
    ],
};

function renamed(fields, changes) {
    return Object.fromEntries(
        Object.entries(changes).map(([key, value]) => {
            if (!(key in fields)) throw new SettingsError(`Unknown setting ${key}`);
            const [name, isValid] = fields[key];
            if (!isValid(value)) throw new SettingsError(`Invalid value for setting ${key}`);
            return [name, value];
        })
    );
}

export function displayPatch(changes) {
    return { display: renamed(DISPLAY_FIELDS, changes) };
}

export function notificationsPatch(changes) {
    return { notifications: renamed(NOTIFICATION_FIELDS, changes) };
}

export function quietHoursPatch(changes) {
    return { notifications: { quiet_hours: renamed(QUIET_HOURS_FIELDS, changes) } };
}

export function providerThresholdPatch(provider, threshold) {
    if (!nonEmpty(provider)) throw new SettingsError('Provider id must not be empty');
    if (threshold !== null && !isProviderThreshold(threshold))
        throw new SettingsError(`Invalid threshold for ${provider}`);
    return { notifications: { provider_thresholds: { [provider]: threshold } } };
}

export function refreshIntervalPatch(secs) {
    return { refresh_interval_secs: refreshInterval(secs) };
}

export function adaptiveRefreshPatch(enabled) {
    return { adaptive_refresh: enabled === true };
}

export function updatesPatch(check) {
    return { updates: { check: check === true } };
}

export function statusPagesPatch(enabled) {
    return { status_pages: { enabled: enabled === true } };
}

export function shortcutPatch(accelerator) {
    if (!isAccelerator(accelerator)) throw new SettingsError(`Invalid shortcut ${accelerator}`);
    return { shortcuts: { open: accelerator } };
}

export function loggingPatch(level) {
    if (!LOG_LEVELS.includes(level)) throw new SettingsError(`Invalid log level ${level}`);
    return { logging: { level } };
}

export function onboardingPatch(completed) {
    return { onboarding: { completed: completed === true } };
}

export function headlinePatch(headline) {
    if (headline.mode !== 'pinned') return { headline: { mode: 'auto', account_id: null, window: null } };
    return { headline: { mode: 'pinned', account_id: headline.accountId, window: headline.window } };
}

export function hiddenWindowsPatch(accountId, windows) {
    return { display: { hidden_windows: { [accountId]: windows.length > 0 ? windows : null } } };
}

function limitEntry(limit) {
    const accountId = nonEmpty(limit?.accountId);
    const window = nonEmpty(limit?.window);
    if (!accountId || !window) throw new SettingsError('A panel limit needs an account and a window');
    return { accountId, window };
}

export function panelLimitsPatch(limits) {
    const distinct = display.distinctLimits(limits.map(limitEntry));
    if (distinct.length > display.MAX_PANEL_LIMITS)
        throw new SettingsError(`At most ${display.MAX_PANEL_LIMITS} panel limits`);
    return {
        display: { panel_limits: distinct.map(limit => ({ account_id: limit.accountId, window: limit.window })) },
    };
}

export function panelPositionPatch(position) {
    if (!display.PANEL_BOXES.includes(position.box) || !display.isPanelIndex(position.index))
        throw new SettingsError('Invalid panel position');
    return { display: { panel_position: { box: position.box, index: position.index } } };
}

export function starredAccountsPatch(accountIds) {
    return { display: { starred_accounts: distinctIds(accountIds) } };
}

function namesAny(section, keys) {
    return isObject(section) && keys.some(key => key in section);
}

export function needsSettings06(patch) {
    if (!isObject(patch)) return false;
    if (SINCE_06.top.some(key => key in patch)) return true;
    if (namesAny(patch.notifications, SINCE_06.notifications)) return true;
    if (namesAny(patch.display, SINCE_06.display)) return true;
    return patch.display?.panel_label === 'none';
}
