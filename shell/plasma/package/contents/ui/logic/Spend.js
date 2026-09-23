.pragma library

.import "I18n.js" as I18n
.import "Providers.js" as Providers

const PERIODS = [
    {
        value: "today",
        msgid: I18n.N("Today")
    },
    {
        value: "yesterday",
        msgid: I18n.N("Yesterday")
    },
    {
        value: "last30Days",
        msgid: I18n.N("30 Days")
    }
];

const MIN_SLICE = 0.025;
const START_DEGREES = -90;
const DOT_SWEEP = 0.1;

function periodOptions(lang) {
    return PERIODS.map(period => ({
                value: period.value,
                label: I18n.tr(lang, period.msgid)
            }));
}

function periodTitle(lang, key) {
    const period = PERIODS.find(candidate => candidate.value === key) ?? PERIODS[0];
    return I18n.tr(lang, period.msgid);
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

function fullRing(spend) {
    return {
        start: START_DEGREES,
        sweep: 360,
        color: Providers.providerInfo(spend.provider).ringColor
    };
}

function slices(providers, gapDegrees, capDegrees) {
    if (providers.length === 1)
        return [fullRing(providers[0])];
    const inset = gapDegrees / 2 + capDegrees;
    const fractions = visibleFractions(providers.map(spend => spend.costMicros), (inset * 2 + DOT_SWEEP) / 360);
    let start = START_DEGREES;
    return providers.map((spend, index) => {
        const sweep = fractions[index] * 360;
        const slice = {
            start: start + inset,
            sweep: Math.max(DOT_SWEEP, sweep - inset * 2),
            color: Providers.providerInfo(spend.provider).ringColor
        };
        start += sweep;
        return slice;
    });
}

function revealed(slice, progress) {
    const reach = START_DEGREES + 360 * progress;
    return Math.max(0, Math.min(slice.sweep, reach - slice.start));
}

function bodyKind(period) {
    if (period.providers.length === 0)
        return "empty";
    return period.providers.length === 1 ? "stats" : "ring";
}

function infoText(lang, period) {
    const partial = period.partial ? ` ${I18n.tr(lang, "Some models have no public price yet.")}` : "";
    return `${I18n.tr(lang, "Estimated from local logs and public pricing.")}${partial}`;
}

function breakdownTitle(lang, periodKey, provider) {
    return `${periodTitle(lang, periodKey)} · ${Providers.providerInfo(provider).name}`;
}
