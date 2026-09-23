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

function visibleFractions(values) {
    const total = values.reduce((sum, value) => sum + value, 0);
    if (total <= 0)
        return values.map(() => 0);
    const raised = values.map(value => Math.max(MIN_SLICE, value / total));
    const raisedTotal = raised.reduce((sum, value) => sum + value, 0);
    return raised.map(value => value / raisedTotal);
}

function slices(providers, gapDegrees) {
    const fractions = visibleFractions(providers.map(spend => spend.costMicros));
    const gap = providers.length > 1 ? gapDegrees : 0;
    let start = START_DEGREES;
    return providers.map((spend, index) => {
        const sweep = fractions[index] * 360;
        const slice = {
            start: start + gap / 2,
            sweep: Math.max(0, sweep - gap),
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
