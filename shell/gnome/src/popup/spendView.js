import { _ } from '../i18n.js';
import { compactTokens, ringUsd, usd } from '../numbers.js';
import { seriesKey } from '../providers.js';
import { spendPeriod, spendPeriods, spendValueText } from '../spendUnits.js';

const DASH = '—';
const LEGACY_PERIODS = ['today', 'yesterday', '30d'];

const PERIOD_TITLES = {
    today: () => _('Today'),
    yesterday: () => _('Yesterday'),
    '7d': () => _('7 Days'),
    '30d': () => _('30 Days'),
};

const PERIOD_SPANS = {
    today: () => _('Today'),
    yesterday: () => _('Yesterday'),
    '7d': () => _('Last 7 days'),
    '30d': () => _('Last 30 days'),
};

export const UNITS = ['cost', 'tokens', 'cost_per_mtok'];

const UNIT_TITLES = {
    cost: () => _('Total Spend'),
    tokens: () => _('Total Tokens'),
    cost_per_mtok: () => _('Cost per MTok'),
};

const UNIT_HINTS = {
    cost: () => _('Dollars, from local logs and public prices'),
    tokens: () => _('Input, output and cache tokens'),
    cost_per_mtok: () => _('Spend divided by million tokens'),
};

export function periodChoices(spend) {
    const available = spendPeriods(spend);
    return available.length > 0 ? available : LEGACY_PERIODS;
}

export function shownPeriod(spend, period) {
    return periodChoices(spend).includes(period) ? period : '30d';
}

export function periodTitle(period) {
    return (PERIOD_TITLES[period] ?? PERIOD_TITLES['30d'])();
}

export function periodSpan(period) {
    return (PERIOD_SPANS[period] ?? PERIOD_SPANS['30d'])();
}

export function periodData(spend, period) {
    return spendPeriod(spend, shownPeriod(spend, period));
}

export function unitTitle(unit) {
    return (UNIT_TITLES[unit] ?? UNIT_TITLES.cost)();
}

export function unitHint(unit) {
    return (UNIT_HINTS[unit] ?? UNIT_HINTS.cost)();
}

function ringValue(entry, unit) {
    return unit === 'cost' ? entry.costMicros : entry.totalTokens;
}

export function unitSlices(period, unit) {
    return period.providers.map(spend => ({ value: ringValue(spend, unit), series: seriesKey(spend.provider) }));
}

export function centerValue(period, unit) {
    if (unit === 'tokens') return period.totalTokens;
    if (unit === 'cost_per_mtok') return period.costPerMtokMicros;
    return period.costMicros;
}

export function centerText(value, unit) {
    if (unit === 'tokens') return compactTokens(value);
    if (unit === 'cost_per_mtok') return value === null ? DASH : usd(value);
    return ringUsd(value);
}

export function centerCaption(unit) {
    if (unit === 'tokens') return _('tokens');
    if (unit === 'cost_per_mtok') return _('blended');
    return null;
}

export function legendValue(entry, unit) {
    if (unit === 'cost_per_mtok') return entry.costPerMtokMicros;
    return unit === 'tokens' ? entry.totalTokens : entry.costMicros;
}

export function legendText(value, unit) {
    if (unit === 'cost_per_mtok') return value === null ? DASH : usd(value);
    return spendValueText({ costMicros: value, totalTokens: value }, unit);
}

export function spendBodyKey(period, unit) {
    return JSON.stringify([period.providers.length === 0, unit, period.providers.map(spend => spend.provider)]);
}
