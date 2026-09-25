import { isObject } from './fields.js';
import { choice, distinctIds, flag, nonEmpty } from './settingsValues.js';

export const THEMES = ['system', 'light', 'dark'];
export const VALUE_MODES = ['left', 'used'];
export const RESET_FORMATS = ['countdown', 'exact'];
export const PANEL_LABELS = ['percent', 'window', 'none'];
export const LANGUAGES = ['system', 'en', 'ru'];
export const DENSITIES = ['normal', 'compact'];
export const TIME_FORMATS = ['auto', '12h', '24h'];
export const PANEL_MODES = ['headline', 'several', 'icon'];
export const PANEL_INDICATORS = ['ring', 'bar', 'none'];
export const PANEL_BOXES = ['left', 'center', 'right'];
export const SPEND_PERIODS = ['today', 'yesterday', '7d', '30d'];
export const SPEND_UNITS = ['cost', 'tokens', 'cost_per_mtok'];
export const SPEND_BREAKDOWNS = ['models', 'projects'];
export const MAX_PANEL_LIMITS = 3;

const DEFAULT_PANEL_POSITION = Object.freeze({ box: 'right', index: 0 });

function parseHiddenWindows(raw) {
    if (!isObject(raw)) return {};
    const entries = Object.entries(raw)
        .map(([accountId, windows]) => [accountId, distinctIds(windows)])
        .filter(([, windows]) => windows.length > 0);
    return Object.fromEntries(entries);
}

function parsePanelLimit(raw) {
    if (!isObject(raw)) return null;
    const accountId = nonEmpty(raw.account_id);
    const window = nonEmpty(raw.window);
    return accountId && window ? { accountId, window } : null;
}

export function distinctLimits(limits) {
    const keys = limits.map(limit => `${limit.accountId}\u0000${limit.window}`);
    return limits.filter((_limit, index) => keys.indexOf(keys[index]) === index);
}

function parsePanelLimits(raw) {
    const limits = (Array.isArray(raw) ? raw : []).map(parsePanelLimit).filter(Boolean);
    return distinctLimits(limits).slice(0, MAX_PANEL_LIMITS);
}

export function isPanelIndex(value) {
    return Number.isInteger(value) && value >= 0;
}

function parsePanelPosition(raw) {
    if (!isObject(raw) || !PANEL_BOXES.includes(raw.box) || !isPanelIndex(raw.index))
        return { ...DEFAULT_PANEL_POSITION };
    return { box: raw.box, index: raw.index };
}

function parseAppearance(display) {
    return {
        theme: choice(THEMES, display.theme, 'system'),
        language: choice(LANGUAGES, display.language, 'system'),
        valueMode: choice(VALUE_MODES, display.value_mode, 'left'),
        resetFormat: choice(RESET_FORMATS, display.reset_format, 'countdown'),
        density: choice(DENSITIES, display.density, 'normal'),
        timeFormat: choice(TIME_FORMATS, display.time_format, 'auto'),
        translucent: flag(display.translucent, false),
        hideOnScreenShare: flag(display.hide_on_screen_share, true),
    };
}

function parsePanel(display) {
    return {
        panelLabel: choice(PANEL_LABELS, display.panel_label, 'percent'),
        panelMode: choice(PANEL_MODES, display.panel_mode, 'headline'),
        panelIndicator: choice(PANEL_INDICATORS, display.panel_indicator, 'ring'),
        panelLimits: parsePanelLimits(display.panel_limits),
        panelPosition: parsePanelPosition(display.panel_position),
    };
}

function parseSections(display) {
    return {
        showSpend: flag(display.show_spend, true),
        showAccountSpend: flag(display.show_account_spend, true),
        showTrend: flag(display.show_trend, true),
        showForecast: flag(display.show_forecast, true),
        spendPeriod: choice(SPEND_PERIODS, display.spend_period, '30d'),
        spendUnit: choice(SPEND_UNITS, display.spend_unit, 'cost'),
        spendBreakdown: choice(SPEND_BREAKDOWNS, display.spend_breakdown, 'models'),
    };
}

function parseCards(display) {
    return {
        combineAccounts: flag(display.combine_accounts, false),
        hiddenWindows: parseHiddenWindows(display.hidden_windows),
        starredAccounts: distinctIds(display.starred_accounts),
        collapseUnstarred: flag(display.collapse_unstarred, false),
    };
}

export function parseDisplay(raw) {
    const display = isObject(raw) ? raw : {};
    return { ...parseAppearance(display), ...parsePanel(display), ...parseSections(display), ...parseCards(display) };
}
