.pragma library

.import "I18n.js" as I18n
.import "Providers.js" as Providers

const PERIODS = [
    {
        value: "today",
        key: "today",
        msgid: I18n.N("Today")
    },
    {
        value: "yesterday",
        key: "yesterday",
        msgid: I18n.N("Yesterday")
    },
    {
        value: "7d",
        key: "last7Days",
        msgid: I18n.N("7 Days")
    },
    {
        value: "30d",
        key: "last30Days",
        msgid: I18n.N("30 Days")
    }
];

const MIN_SLICE = 0.025;
const START_DEGREES = -90;

function periodFor(value) {
    return PERIODS.find(candidate => candidate.value === value) ?? PERIODS[0];
}

function hasPeriod(spend, value) {
    return (spend?.[periodFor(value).key] ?? null) !== null;
}

function periodOptions(lang, spend) {
    return PERIODS.filter(period => period.value !== "7d" || hasPeriod(spend, period.value)).map(period => ({
                value: period.value,
                label: I18n.tr(lang, period.msgid)
            }));
}

function periodTotals(spend, value) {
    return hasPeriod(spend, value) ? spend[periodFor(value).key] : spend.last30Days;
}

function periodTitle(lang, value) {
    return I18n.tr(lang, periodFor(value).msgid);
}

function hasProjects(period) {
    return period.projects !== null;
}

function sharedFractions(values, minimum, raised) {
    const free = values.reduce((sum, value, index) => raised[index] ? sum : sum + value, 0);
    const share = 1 - minimum * raised.filter(Boolean).length;
    return values.map((value, index) => raised[index] ? minimum : value / free * share);
}

function raisedFractions(values, minimum) {
    let raised = values.map(() => false);
    for (let round = 0; round < values.length; round++) {
        const fractions = sharedFractions(values, minimum, raised);
        const grown = fractions.map((fraction, index) => raised[index] || fraction < minimum);
        if (grown.every((flag, index) => flag === raised[index]))
            return fractions;
        raised = grown;
    }
    return sharedFractions(values, minimum, raised);
}

function visibleFractions(values, minimum) {
    if (values.length === 0)
        return [];
    const total = values.reduce((sum, value) => sum + value, 0);
    if (total <= 0)
        return values.map(() => 1 / values.length);
    return raisedFractions(values, Math.min(Math.max(MIN_SLICE, minimum), 1 / values.length));
}

function fullRing(spend, dark) {
    return {
        start: START_DEGREES,
        sweep: 360,
        color: Providers.ringColor(spend.provider, dark)
    };
}

function weightOf(spend, byTokens) {
    return byTokens === true ? spend.totalTokens : spend.costMicros;
}

function slices(providers, minDegrees, dark, byTokens) {
    if (providers.length === 1)
        return [fullRing(providers[0], dark)];
    const fractions = visibleFractions(providers.map(spend => weightOf(spend, byTokens)), minDegrees / 360);
    let start = START_DEGREES;
    return providers.map((spend, index) => {
        const slice = {
            start,
            sweep: fractions[index] * 360,
            color: Providers.ringColor(spend.provider, dark)
        };
        start += slice.sweep;
        return slice;
    });
}

function revealed(slice, progress) {
    const reach = START_DEGREES + 360 * progress;
    return Math.max(0, Math.min(slice.sweep, reach - slice.start));
}

function bodyKind(period) {
    return period.providers.length === 0 ? "empty" : "ring";
}

function infoText(lang, period) {
    const partial = period.partial ? ` ${I18n.tr(lang, "Some models have no public price yet.")}` : "";
    return `${I18n.tr(lang, "Estimated from local logs and public pricing.")}${partial}`;
}

function breakdownTitle(lang, periodKey, spend) {
    return `${periodTitle(lang, periodKey)} · ${spend.providerName}`;
}

function popoverTitle(lang, periodKey, spend) {
    return `${spend.providerName} · ${periodTitle(lang, periodKey)}`;
}
