.pragma library

.import "Providers.js" as Providers

const PERIODS = [
    {
        key: "today",
        title: "Today"
    },
    {
        key: "yesterday",
        title: "Yesterday"
    },
    {
        key: "last30Days",
        title: "30 Days"
    }
];

const MIN_SLICE = 0.025;
const START_DEGREES = -90;

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

function bodyKind(period) {
    if (period.providers.length === 0)
        return "empty";
    return period.providers.length === 1 ? "stats" : "ring";
}

function infoText(period) {
    const partial = period.partial ? " Some models have no public price yet." : "";
    return `Estimated from local logs and public pricing.${partial}`;
}
