import { _, fill } from './i18n.js';
import { compactTokens, usd } from './numbers.js';

const DASH = '—';
const ELLIPSIS = '…';

const PERIOD_FIELDS = { today: 'today', yesterday: 'yesterday', '7d': 'last7Days', '30d': 'last30Days' };

export function spendPeriods(spend) {
    return Object.keys(PERIOD_FIELDS).filter(period => spend?.[PERIOD_FIELDS[period]]);
}

export function spendPeriod(spend, period) {
    return spend?.[PERIOD_FIELDS[period]] ?? spend?.last30Days ?? null;
}

export function hasProjects(period) {
    return Array.isArray(period?.projects);
}

export function costPerMtokText(micros) {
    if (micros === null || micros === undefined) return DASH;
    return fill(_('{cost}/MTok'), { cost: usd(micros) });
}

export function spendValue(entry, unit) {
    if (unit === 'tokens') return entry.totalTokens;
    if (unit === 'cost_per_mtok') return entry.costPerMtokMicros ?? null;
    return entry.costMicros;
}

export function spendValueText(entry, unit) {
    if (unit === 'tokens') return compactTokens(entry.totalTokens);
    if (unit === 'cost_per_mtok') return costPerMtokText(entry.costPerMtokMicros ?? null);
    return usd(entry.costMicros);
}

export function ellipsizedMiddle(value, maxChars) {
    const chars = Array.from(value);
    if (chars.length <= maxChars) return value;
    if (maxChars < 2) return ELLIPSIS;
    const tail = Math.ceil((maxChars - 1) / 2);
    const head = maxChars - 1 - tail;
    return `${chars.slice(0, head).join('')}${ELLIPSIS}${chars.slice(chars.length - tail).join('')}`;
}

export function projectLabel(project, maxChars) {
    if (project === null) return _('No project');
    return ellipsizedMiddle(project, maxChars);
}
