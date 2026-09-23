.pragma library

.import "I18n.js" as I18n

const THEMES = ["system", "light", "dark"];
const VALUE_MODES = ["left", "used"];
const RESET_FORMATS = ["countdown", "exact"];
const PANEL_LABELS = ["percent", "window"];
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
    showAccountSpend: "show_account_spend",
    showTrend: "show_trend",
    showForecast: "show_forecast",
    translucent: "translucent",
    hiddenWindows: "hidden_windows"
};

const NOTIFICATION_KEYS = {
    almostOut: "almost_out",
    cuttingItClose: "cutting_it_close",
    willRunOut: "will_run_out",
    reset: "reset"
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
        showAccountSpend: flag(display.show_account_spend, true),
        showTrend: flag(display.show_trend, true),
        showForecast: flag(display.show_forecast, true),
        translucent: flag(display.translucent, false),
        hiddenWindows: parseHiddenWindows(display.hidden_windows)
    };
}

function parseNotifications(raw) {
    const notifications = isObject(raw) ? raw : {};
    return {
        almostOut: flag(notifications.almost_out, true),
        cuttingItClose: flag(notifications.cutting_it_close, true),
        willRunOut: flag(notifications.will_run_out, true),
        reset: flag(notifications.reset, false)
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
        display: parseDisplay(raw.display)
    };
}

function isSettingsError(error) {
    return error instanceof SettingsError;
}

function renamed(patch, keys) {
    return fromPairs(Object.entries(patch).filter(([key]) => key in keys).map(([key, value]) => [keys[key], value]));
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
