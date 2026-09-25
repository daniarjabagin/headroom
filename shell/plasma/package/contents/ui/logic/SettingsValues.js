.pragma library

const DENSITIES = ["normal", "compact"];
const TIME_FORMATS = ["auto", "12h", "24h"];
const PANEL_MODES = ["headline", "several", "icon"];
const PANEL_INDICATORS = ["ring", "bar", "none"];
const PANEL_BOXES = ["left", "center", "right"];
const SPEND_PERIODS = ["today", "yesterday", "7d", "30d"];
const SPEND_UNITS = ["cost", "tokens", "cost_per_mtok"];
const SPEND_BREAKDOWNS = ["models", "projects"];
const LOG_LEVELS = ["error", "warn", "info", "debug"];
const MAX_PANEL_LIMITS = 3;
const MIN_THRESHOLD = 1;
const MAX_THRESHOLD = 50;
const DEFAULT_THRESHOLD = 10;
const MAX_SHORTCUT_LENGTH = 64;
const CLOCK = /^([01]\d|2[0-3]):[0-5]\d$/;
const ACCELERATOR = /^(<[A-Za-z]+>)*[A-Za-z0-9_]+$/;
const DEFAULT_POSITION = {
    box: "right",
    index: 0
};
const DEFAULT_QUIET_HOURS = {
    enabled: false,
    from: "22:00",
    to: "08:00",
    allowCritical: true
};

function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function nonEmpty(value) {
    return typeof value === "string" && value.length > 0 ? value : null;
}

function clamp(value, low, high) {
    return Math.min(high, Math.max(low, value));
}

function distinctIds(raw) {
    return Array.isArray(raw) ? [...new Set(raw.filter(nonEmpty))] : [];
}

function limitKey(limit) {
    return `${limit.accountId}\n${limit.window}`;
}

function panelLimit(raw) {
    if (!isObject(raw))
        return null;
    const accountId = nonEmpty(raw.account_id);
    const window = nonEmpty(raw.window);
    return accountId && window ? {
        accountId,
        window
    } : null;
}

function panelLimits(raw) {
    const limits = Array.isArray(raw) ? raw.map(panelLimit).filter(limit => limit !== null) : [];
    const keys = limits.map(limitKey);
    return limits.filter((limit, index) => keys.indexOf(limitKey(limit)) === index).slice(0, MAX_PANEL_LIMITS);
}

function encodedPanelLimits(limits) {
    const raw = Array.isArray(limits) ? limits.filter(isObject).map(limit => ({
                    account_id: limit.accountId,
                    window: limit.window
                })) : [];
    return panelLimits(raw).map(limit => ({
                account_id: limit.accountId,
                window: limit.window
            }));
}

function panelPosition(raw) {
    if (!isObject(raw) || !PANEL_BOXES.includes(raw.box) || !Number.isInteger(raw.index) || raw.index < 0)
        return Object.assign({}, DEFAULT_POSITION);
    return {
        box: raw.box,
        index: raw.index
    };
}

function thresholdPercent(value) {
    return Number.isInteger(value) ? clamp(value, MIN_THRESHOLD, MAX_THRESHOLD) : DEFAULT_THRESHOLD;
}

function providerThreshold(value) {
    return Number.isInteger(value) && value >= 0 && value <= MAX_THRESHOLD;
}

function providerThresholds(raw) {
    if (!isObject(raw))
        return {};
    return Object.entries(raw).filter(([provider, value]) => provider.length > 0 && providerThreshold(value)).reduce((result, [provider, value]) => Object.assign(result, {
                [provider]: value
            }), {});
}

function clock(value, fallback) {
    return typeof value === "string" && CLOCK.test(value) ? value : fallback;
}

function quietHours(raw) {
    const hours = isObject(raw) ? raw : {};
    return {
        enabled: typeof hours.enabled === "boolean" ? hours.enabled : DEFAULT_QUIET_HOURS.enabled,
        from: clock(hours.from, DEFAULT_QUIET_HOURS.from),
        to: clock(hours.to, DEFAULT_QUIET_HOURS.to),
        allowCritical: typeof hours.allow_critical === "boolean" ? hours.allow_critical : DEFAULT_QUIET_HOURS.allowCritical
    };
}

function encodedQuietHours(hours) {
    const encoded = {};
    if (typeof hours.enabled === "boolean")
        encoded.enabled = hours.enabled;
    if (clock(hours.from, null) !== null)
        encoded.from = hours.from;
    if (clock(hours.to, null) !== null)
        encoded.to = hours.to;
    if (typeof hours.allowCritical === "boolean")
        encoded.allow_critical = hours.allowCritical;
    return encoded;
}

function canEnableQuietHours(hours) {
    return hours.from !== hours.to;
}

function isClock(value) {
    return clock(value, null) !== null;
}

function shortcut(value) {
    if (typeof value !== "string" || value.length > MAX_SHORTCUT_LENGTH)
        return "";
    return value === "" || ACCELERATOR.test(value) ? value : "";
}

function isShortcut(value) {
    return value === "" || shortcut(value) === value;
}
