.pragma library

.import "Parse.js" as Parse

const TONES = ["good", "warning", "critical", "neutral"];
const MAX_ITEMS = 3;

function parseHeadline(raw) {
    if (!Parse.isObject(raw))
        return null;
    const remainingPercent = Parse.number(raw.remaining_percent);
    if (remainingPercent === null)
        return null;
    const provider = Parse.text(raw.provider);
    return {
        accountId: Parse.text(raw.account_id),
        windowId: Parse.text(raw.window),
        provider,
        providerName: Parse.text(raw.provider_name) ?? provider,
        accountLabel: Parse.text(raw.account_label),
        windowLabel: Parse.text(raw.window_label),
        usedPercent: Parse.number(raw.used_percent),
        remainingPercent,
        tone: Parse.oneOf(TONES, raw.tone, "neutral"),
        combined: raw.combined === true,
        accountCount: Parse.number(raw.account_count)
    };
}

function headlinePercent(headline, valueMode) {
    if (valueMode === "used" && headline.usedPercent !== null)
        return headline.usedPercent;
    return headline.remainingPercent;
}

function parseItem(raw, valueMode) {
    const headline = parseHeadline(raw);
    if (headline === null)
        return null;
    return Object.assign(headline, {
        valuePercent: Parse.number(raw.value_percent) ?? headlinePercent(headline, valueMode),
        evenPacePercent: Parse.number(raw.even_pace_percent),
        logo: Parse.text(raw.logo) ?? headline.provider
    });
}

function parseItems(raw, valueMode) {
    return Parse.list(raw).map(item => parseItem(item, valueMode)).filter(item => item !== null).slice(0, MAX_ITEMS);
}

function headlineEvenPace(headline, accounts) {
    const account = accounts.find(candidate => candidate.id === headline.accountId);
    const window = account?.windows.find(candidate => candidate.id === headline.windowId);
    return window?.pace.evenPacePercent ?? null;
}

function headlineItem(headline, accounts, valueMode) {
    return Object.assign({}, headline, {
        valuePercent: headlinePercent(headline, valueMode),
        evenPacePercent: headlineEvenPace(headline, accounts),
        logo: headline.provider
    });
}

function items(raw, headline, accounts, valueMode) {
    if (Array.isArray(raw))
        return parseItems(raw, valueMode);
    return headline === null ? [] : [headlineItem(headline, accounts, valueMode)];
}

function tone(raw, served, headline) {
    if (served)
        return Parse.oneOf(TONES, raw, null);
    return headline?.tone ?? null;
}

function served(raw) {
    return Array.isArray(raw.panel_items);
}
