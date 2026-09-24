import { seriesKey } from './providers.js';

export function bodyKind(period) {
    return period.providers.length === 0 ? 'empty' : 'ring';
}

export function bodyKey(period) {
    return JSON.stringify([bodyKind(period), period.providers.map(spend => spend.provider)]);
}

export function ringSlices(period) {
    return period.providers.map(spend => ({ value: spend.costMicros, series: seriesKey(spend.provider) }));
}

export function showsTokenLine(period) {
    return period.providers.length === 1;
}
