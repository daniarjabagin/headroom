.pragma library

.import "I18n.js" as I18n
.import "SettingsValues.js" as Values

const THEMES = ["system", "light", "dark"];
const VALUE_MODES = ["left", "used"];
const RESET_FORMATS = ["countdown", "exact"];
const PANEL_LABELS = ["percent", "window", "none"];
const LANGUAGES = ["system", "en", "ru"];
const MIN_REFRESH_SECS = 60;
const MAX_REFRESH_SECS = 3600;
const DEFAULT_REFRESH_SECS = 300;

const DISPLAY_KEYS = {
    theme: "theme",
    language: "language",
    valueMode: "value_mode",
    resetFormat: "reset_format",
    panelLabel: "panel_label",
    showSpend: "show_spend",
    showBreakdown: "show_breakdown",
    showAccountSpend: "show_account_spend",
    showTrend: "show_trend",
    showForecast: "show_forecast",
    translucent: "translucent",
    combineAccounts: "combine_accounts",
    hiddenWindows: "hidden_windows",
    density: "density",
    timeFormat: "time_format",
    panelMode: "panel_mode",
    panelIndicator: "panel_indicator",
    panelLimits: "panel_limits",
    panelPosition: "panel_position",
    spendPeriod: "spend_period",
    spendUnit: "spend_unit",
    spendBreakdown: "spend_breakdown",
    starredAccounts: "starred_accounts",
    collapseUnstarred: "collapse_unstarred",
    hideOnScreenShare: "hide_on_screen_share"
};

const NOTIFICATION_KEYS = {
    almostOut: "almost_out",
    cuttingItClose: "cutting_it_close",
    willRunOut: "will_run_out",
    reset: "reset",
    thresholdPercent: "threshold_percent",
    quietHours: "quiet_hours"
};

const ENCODERS = {
    panelLimits: Values.encodedPanelLimits,
    panelPosition: Values.panelPosition,
    starredAccounts: Values.distinctIds,
    thresholdPercent: Values.thresholdPercent,
    quietHours: Values.encodedQuietHours
};

class SettingsError extends I18n.LocalizedError {}

function fromPairs(pairs) {
    return pairs.reduce((result, [key, value]) => Object.assign(result, {
                [key]: value
            }), {});
}

function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function section(value) {
    return isObject(value) ? value : {};
}

function flag(value, fallback) {
    return typeof value === "boolean" ? value : fallback;
}

function choice(allowed, value, fallback) {
    return allowed.includes(value) ? value : fallback;
}

function nonEmpty(value) {
    return typeof value === "string" && value.length > 0 ? value : null;
}

function refreshInterval(value) {
    if (!Number.isInteger(value))
        return DEFAULT_REFRESH_SECS;
    return Math.min(MAX_REFRESH_SECS, Math.max(MIN_REFRESH_SECS, value));
}

function parseHiddenWindows(raw) {
    if (!isObject(raw))
        return {};
    const entries = Object.entries(raw).map(([accountId, windows]) => [accountId, Array.isArray(windows) ? [...new Set(windows.filter(nonEmpty))] : []]).filter(([, windows]) => windows.length > 0);
    return fromPairs(entries);
}

function parseDisplay(raw) {
    const display = isObject(raw) ? raw : {};
    return {
        theme: choice(THEMES, display.theme, "system"),
        language: choice(LANGUAGES, display.language, "system"),
        valueMode: choice(VALUE_MODES, display.value_mode, "left"),
        resetFormat: choice(RESET_FORMATS, display.reset_format, "countdown"),
        panelLabel: choice(PANEL_LABELS, display.panel_label, "percent"),
        showSpend: flag(display.show_spend, true),
        showBreakdown: flag(display.show_breakdown, true),
        showAccountSpend: flag(display.show_account_spend, true),
        showTrend: flag(display.show_trend, true),
        showForecast: flag(display.show_forecast, true),
        translucent: flag(display.translucent, false),
        combineAccounts: flag(display.combine_accounts, false),
        hiddenWindows: parseHiddenWindows(display.hidden_windows),
        density: choice(Values.DENSITIES, display.density, "normal"),
        timeFormat: choice(Values.TIME_FORMATS, display.time_format, "auto"),
        panelMode: choice(Values.PANEL_MODES, display.panel_mode, "headline"),
        panelIndicator: choice(Values.PANEL_INDICATORS, display.panel_indicator, "ring"),
        panelLimits: Values.panelLimits(display.panel_limits),
        panelPosition: Values.panelPosition(display.panel_position),
        spendPeriod: choice(Values.SPEND_PERIODS, display.spend_period, "30d"),
        spendUnit: choice(Values.SPEND_UNITS, display.spend_unit, "cost"),
        spendBreakdown: choice(Values.SPEND_BREAKDOWNS, display.spend_breakdown, "models"),
        starredAccounts: Values.distinctIds(display.starred_accounts),
        collapseUnstarred: flag(display.collapse_unstarred, false),
        hideOnScreenShare: flag(display.hide_on_screen_share, true)
    };
}

function parseNotifications(raw) {
    const notifications = isObject(raw) ? raw : {};
    return {
        almostOut: flag(notifications.almost_out, true),
        cuttingItClose: flag(notifications.cutting_it_close, true),
        willRunOut: flag(notifications.will_run_out, true),
        reset: flag(notifications.reset, false),
        thresholdPercent: Values.thresholdPercent(notifications.threshold_percent),
        providerThresholds: Values.providerThresholds(notifications.provider_thresholds),
        quietHours: Values.quietHours(notifications.quiet_hours)
    };
}

function parseUpdates(raw) {
    const updates = isObject(raw) ? raw : {};
    return {
        check: flag(updates.check, true)
    };
}

function parseHeadline(raw) {
    const headline = isObject(raw) ? raw : {};
    const accountId = nonEmpty(headline.account_id);
    const window = nonEmpty(headline.window);
    if (headline.mode === "pinned" && accountId && window)
        return {
            mode: "pinned",
            accountId,
            window
        };
    return {
        mode: "auto"
    };
}

function decode(json) {
    let raw;
    try {
        raw = JSON.parse(json);
    } catch (error) {
        throw new SettingsError(I18n.N("Unreadable settings from the Headroom service: {reason}"), {
            reason: error.message
        });
    }
    if (!isObject(raw))
        throw new SettingsError(I18n.N("Unexpected settings from the Headroom service"));
    return raw;
}

function fromRaw(raw) {
    return {
        refreshIntervalSecs: refreshInterval(raw.refresh_interval_secs),
        notifications: parseNotifications(raw.notifications),
        headline: parseHeadline(raw.headline),
        reducedMotion: flag(raw.reduced_motion, false),
        display: parseDisplay(raw.display),
        updates: parseUpdates(raw.updates),
        adaptiveRefresh: flag(raw.adaptive_refresh, true),
        statusPages: {
            enabled: flag(section(raw.status_pages).enabled, false)
        },
        shortcuts: {
            open: Values.shortcut(section(raw.shortcuts).open)
        },
        logging: {
            level: choice(Values.LOG_LEVELS, section(raw.logging).level, "info")
        },
        onboarding: {
            completed: flag(section(raw.onboarding).completed, false)
        }
    };
}

function isSettingsError(error) {
    return error instanceof SettingsError;
}

function encoded(key, value) {
    return value !== null && key in ENCODERS ? ENCODERS[key](value) : value;
}

function renamed(patch, keys) {
    return fromPairs(Object.entries(patch).filter(([key]) => key in keys).map(([key, value]) => [keys[key], encoded(key, value)]));
}

function mergePatch(target, patch) {
    if (!isObject(patch))
        return patch;
    const result = Object.assign({}, isObject(target) ? target : {});
    for (const [key, value] of Object.entries(patch)) {
        if (value === null)
            delete result[key];
        else
            result[key] = mergePatch(result[key], value);
    }
    return result;
}

function displayPatch(patch) {
    return {
        display: renamed(patch, DISPLAY_KEYS)
    };
}

function notificationsPatch(patch) {
    return {
        notifications: renamed(patch, NOTIFICATION_KEYS)
    };
}

function refreshIntervalPatch(seconds) {
    return {
        refresh_interval_secs: refreshInterval(seconds)
    };
}

function updatesPatch(check) {
    return {
        updates: {
            check: check === true
        }
    };
}

function headlinePatch(headline) {
    const pinned = headline.mode === "pinned";
    return {
        headline: {
            mode: pinned ? "pinned" : "auto",
            account_id: pinned ? headline.accountId : null,
            window: pinned ? headline.window : null
        }
    };
}

function toggledValueMode(display) {
    return {
        valueMode: display.valueMode === "used" ? "left" : "used"
    };
}

function toggledResetFormat(display) {
    return {
        resetFormat: display.resetFormat === "exact" ? "countdown" : "exact"
    };
}

function isWindowHidden(display, accountId, windowId) {
    return (display.hiddenWindows[accountId] ?? []).includes(windowId);
}

function windowHiddenPatch(display, accountId, windowId, hidden) {
    const current = (display.hiddenWindows[accountId] ?? []).filter(id => id !== windowId);
    const next = hidden ? current.concat([windowId]) : current;
    return {
        hiddenWindows: {
            [accountId]: next.length > 0 ? next : null
        }
    };
}

function adaptiveRefreshPatch(enabled) {
    return {
        adaptive_refresh: enabled === true
    };
}

function statusPagesPatch(enabled) {
    return {
        status_pages: {
            enabled: enabled === true
        }
    };
}

function shortcutPatch(accelerator) {
    return {
        shortcuts: {
            open: Values.shortcut(accelerator)
        }
    };
}

function loggingPatch(level) {
    return {
        logging: {
            level: choice(Values.LOG_LEVELS, level, "info")
        }
    };
}

function onboardingPatch(completed) {
    return {
        onboarding: {
            completed: completed === true
        }
    };
}

function providerThresholdPatch(provider, threshold) {
    const value = Number.isInteger(threshold) ? Math.min(Values.MAX_THRESHOLD, Math.max(0, threshold)) : null;
    return {
        notifications: {
            provider_thresholds: {
                [provider]: value
            }
        }
    };
}

function isStarred(display, accountId) {
    return display.starredAccounts.includes(accountId);
}

function starredPatch(display, accountId, starred) {
    const others = display.starredAccounts.filter(id => id !== accountId);
    return {
        starredAccounts: starred ? others.concat([accountId]) : others
    };
}
