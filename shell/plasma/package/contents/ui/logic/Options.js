.pragma library

.import "Format.js" as Format
.import "I18n.js" as I18n
.import "Providers.js" as Providers
.import "State.js" as State

const AUTO = "auto";
const PIN_SEPARATOR = "\n";
const REFRESH_PRESETS = [60, 120, 300, 600, 900, 1800, 3600];

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

function panelLabelOptions(lang) {
    return choices(lang, [["percent", I18n.N("Percent")], ["window", I18n.N("Provider + limit")]]);
}

function refreshLabel(lang, seconds) {
    const minutes = seconds / 60;
    if (!Number.isInteger(minutes))
        return I18n.trn(lang, "{count} second", "{count} seconds", seconds);
    return I18n.trn(lang, "Every minute", "Every {count} minutes", minutes);
}

function refreshOptions(lang, current) {
    const values = REFRESH_PRESETS.includes(current) ? REFRESH_PRESETS.slice() : REFRESH_PRESETS.concat([current]);
    return values.sort((a, b) => a - b).map(value => ({
                value,
                label: refreshLabel(lang, value)
            }));
}

function pinKey(accountId, windowId) {
    return `${accountId}${PIN_SEPARATOR}${windowId}`;
}

function limitOptions(lang, state, headline) {
    const accounts = state === null ? [] : State.visibleAccounts(state);
    const options = [
        {
            value: AUTO,
            label: I18n.tr(lang, "Auto — most critical")
        }
    ];
    for (const account of accounts) {
        const title = Providers.accountTitle(account, State.showsName(account, accounts));
        for (const window of account.windows)
            options.push({
                value: pinKey(account.id, window.id),
                label: `${title} — ${Format.windowLabel(lang, window)}`
            });
    }
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
    const [accountId, window] = value.split(PIN_SEPARATOR);
    return {
        mode: "pinned",
        accountId,
        window
    };
}

function indexOfValue(options, value) {
    return Math.max(0, options.findIndex(option => option.value === value));
}
