.pragma library

.import "Format.js" as Format
.import "I18n.js" as I18n
.import "Providers.js" as Providers
.import "State.js" as State

const AUTO = "auto";
const PIN_SEPARATOR = "\n";
const REFRESH_PRESETS = [60, 120, 300, 600, 900, 1800, 3600];
const THRESHOLD_PRESETS = [5, 10, 20, 30];
const PROVIDER_DEFAULT = "default";
const PROVIDER_OFF = 0;

const SECTIONS = [
    {
        key: "showSpend",
        title: I18n.N("Total spend"),
        subtitle: I18n.N("Spend ring for all tools at the top")
    },
    {
        key: "showAccountSpend",
        title: I18n.N("Per-account spend"),
        subtitle: I18n.N("Today, yesterday and 30 days under each account")
    },
    {
        key: "showTrend",
        title: I18n.N("Usage trend"),
        subtitle: I18n.N("Daily token bars for the last 30 days")
    },
    {
        key: "showForecast",
        title: I18n.N("Pace forecast"),
        subtitle: I18n.N("Where each limit lands at the current pace")
    }
];

const MILESTONES = [
    {
        key: "almostOut",
        title: I18n.N("Almost out"),
        subtitle: I18n.N("A limit drops under 10% left")
    },
    {
        key: "cuttingItClose",
        title: I18n.N("Cutting it close"),
        subtitle: I18n.N("The pace says a limit will barely last until reset")
    },
    {
        key: "willRunOut",
        title: I18n.N("Will run out"),
        subtitle: I18n.N("The pace says a limit runs out before it resets")
    },
    {
        key: "reset",
        title: I18n.N("Limit reset"),
        subtitle: I18n.N("A limit that was running low resets")
    }
];

function choices(lang, entries) {
    return entries.map(([value, msgid]) => ({
                value,
                label: I18n.tr(lang, msgid)
            }));
}

function themeOptions(lang) {
    return choices(lang, [["system", I18n.N("System")], ["light", I18n.N("Light")], ["dark", I18n.N("Dark")]]);
}

function languageOptions(lang) {
    return [
        {
            value: "system",
            label: I18n.tr(lang, "System")
        },
        {
            value: "en",
            label: "English"
        },
        {
            value: "ru",
            label: "Русский"
        }
    ];
}

function valueModeOptions(lang) {
    return choices(lang, [["left", I18n.N("Left")], ["used", I18n.N("Used")]]);
}

function resetFormatOptions(lang) {
    return choices(lang, [["countdown", I18n.N("Countdown")], ["exact", I18n.N("Exact time")]]);
}

function panelLabelOptions(lang, capable) {
    const entries = [["percent", I18n.N("Percent")], ["window", I18n.N("Provider + limit")]];
    return choices(lang, capable === true ? entries.concat([["none", I18n.N("None")]]) : entries);
}

function densityOptions(lang) {
    return choices(lang, [["normal", I18n.N("Normal")], ["compact", I18n.N("Compact")]]);
}

function timeFormatOptions(lang) {
    return choices(lang, [["auto", I18n.N("Automatic")], ["12h", I18n.N("12-hour")], ["24h", I18n.N("24-hour")]]);
}

function panelModeOptions(lang) {
    return choices(lang, [["headline", I18n.N("One limit")], ["several", I18n.N("Several limits")], ["icon", I18n.N("Icon only")]]);
}

function panelIndicatorOptions(lang) {
    return choices(lang, [["ring", I18n.N("Ring")], ["bar", I18n.N("Bar")], ["none", I18n.N("None")]]);
}

function spendUnitOptions(lang) {
    return choices(lang, [["cost", I18n.N("Cost")], ["tokens", I18n.N("Tokens")], ["cost_per_mtok", I18n.N("Cost per MTok")]]);
}

function spendBreakdownOptions(lang) {
    return choices(lang, [["models", I18n.N("Models")], ["projects", I18n.N("Projects")]]);
}

function logLevelOptions(lang) {
    return choices(lang, [["error", I18n.N("Errors only")], ["warn", I18n.N("Warnings")], ["info", I18n.N("Info")], ["debug", I18n.N("Debug")]]);
}

function percentLabel(percent) {
    return `${percent}%`;
}

function withCurrent(presets, current) {
    const values = presets.includes(current) ? presets.slice() : presets.concat([current]);
    return values.sort((a, b) => a - b);
}

function thresholdOptions(lang, current) {
    return withCurrent(THRESHOLD_PRESETS, current).map(value => ({
                value,
                label: percentLabel(value)
            }));
}

function providerThresholdOptions(lang, globalThreshold, current) {
    const fallback = {
        value: PROVIDER_DEFAULT,
        label: I18n.tr(lang, "Default ({percent}%)", {
            percent: globalThreshold
        })
    };
    const off = {
        value: PROVIDER_OFF,
        label: I18n.tr(lang, "Off")
    };
    const presets = current === undefined || current === PROVIDER_OFF ? THRESHOLD_PRESETS : withCurrent(THRESHOLD_PRESETS, current);
    return [fallback, off].concat(presets.map(value => ({
                    value,
                    label: percentLabel(value)
                })));
}

function providerThresholdKey(thresholds, provider) {
    return provider in thresholds ? thresholds[provider] : PROVIDER_DEFAULT;
}

function providerThresholdFor(value) {
    return value === PROVIDER_DEFAULT ? null : value;
}

function refreshLabel(lang, seconds) {
    const minutes = seconds / 60;
    if (!Number.isInteger(minutes))
        return I18n.trn(lang, "{count} second", "{count} seconds", seconds);
    return I18n.trn(lang, "Every minute", "Every {count} minutes", minutes);
}

function refreshOptions(lang, current) {
    return withCurrent(REFRESH_PRESETS, current).map(value => ({
                value,
                label: refreshLabel(lang, value)
            }));
}

function pinKey(accountId, windowId) {
    return `${accountId}${PIN_SEPARATOR}${windowId}`;
}

function windowOptions(lang, state) {
    const accounts = state === null ? [] : State.visibleAccounts(state);
    const options = [];
    for (const account of accounts) {
        const title = Providers.accountTitle(account, State.showsName(account, accounts));
        for (const window of account.windows)
            options.push({
                value: pinKey(account.id, window.id),
                label: `${title} — ${Format.windowLabel(lang, window)}`
            });
    }
    return options;
}

function panelLimitOptions(lang, state, limit) {
    const options = windowOptions(lang, state);
    const current = limit === null ? null : pinKey(limit.accountId, limit.window);
    if (current !== null && !options.some(option => option.value === current))
        options.push({
            value: current,
            label: I18n.tr(lang, "Pinned limit (not available now)")
        });
    return options;
}

function panelLimitKey(limit) {
    return pinKey(limit.accountId, limit.window);
}

function panelLimitFor(value) {
    const [accountId, window] = value.split(PIN_SEPARATOR);
    return {
        accountId,
        window
    };
}

function limitOptions(lang, state, headline) {
    const options = [
        {
            value: AUTO,
            label: I18n.tr(lang, "Auto — most critical")
        }
    ].concat(windowOptions(lang, state));
    const pinned = headlineKey(headline);
    if (pinned !== AUTO && !options.some(option => option.value === pinned))
        options.push({
            value: pinned,
            label: I18n.tr(lang, "Pinned limit (not available now)")
        });
    return options;
}

function headlineKey(headline) {
    return headline.mode === "pinned" ? pinKey(headline.accountId, headline.window) : AUTO;
}

function headlineFor(value) {
    if (value === AUTO)
        return {
            mode: "auto"
        };
    return Object.assign({
        mode: "pinned"
    }, panelLimitFor(value));
}

function indexOfValue(options, value) {
    return Math.max(0, options.findIndex(option => option.value === value));
}
