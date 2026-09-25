.pragma library

const NEW_KEYS = {
    adaptive_refresh: true,
    status_pages: true,
    shortcuts: true,
    logging: true,
    onboarding: true,
    notifications: {
        threshold_percent: true,
        provider_thresholds: true,
        quiet_hours: true
    },
    display: {
        density: true,
        time_format: true,
        panel_mode: true,
        panel_indicator: true,
        panel_limits: true,
        panel_position: true,
        spend_period: true,
        spend_unit: true,
        spend_breakdown: true,
        starred_accounts: true,
        collapse_unstarred: true,
        hide_on_screen_share: true
    }
};

const NEW_VALUES = {
    display: {
        panel_label: ["none"]
    }
};

function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function supports06(state) {
    return state !== null && state !== undefined && state.supports06 === true;
}

function isNewValue(values, key, value) {
    return Array.isArray(values?.[key]) && values[key].includes(value);
}

function strippedEntry(key, value, keys, values) {
    if (keys?.[key] === true || isNewValue(values, key, value))
        return null;
    if (!isObject(value) || !isObject(keys?.[key]))
        return [key, value];
    const inner = stripped(value, keys[key], values?.[key]);
    return inner === null ? null : [key, inner];
}

function stripped(patch, keys, values) {
    const entries = Object.entries(patch).map(([key, value]) => strippedEntry(key, value, keys, values)).filter(entry => entry !== null);
    if (entries.length === 0 && Object.keys(patch).length > 0)
        return null;
    return entries.reduce((result, [key, value]) => Object.assign(result, {
                [key]: value
            }), {});
}

function compatiblePatch(patch, capable) {
    if (!isObject(patch))
        return null;
    if (capable)
        return Object.keys(patch).length > 0 ? patch : null;
    const result = stripped(patch, NEW_KEYS, NEW_VALUES);
    return result === null || Object.keys(result).length === 0 ? null : result;
}
