import { clockTime } from '../dates.js';
import { _, fill, n_ } from '../i18n.js';
import { PROVIDER_THRESHOLD_OFF, THRESHOLD_CHOICES } from '../settings.js';

export const DEFAULT_THRESHOLD = 'default';

const HALF_HOUR_MINUTES = 30;
const DAY_MINUTES = 24 * 60;

function percentLabel(percent) {
    return fill(_('{percent}%'), { percent });
}

function withCurrent(values, current) {
    const all = values.includes(current) ? values : [...values, current];
    return all.sort((a, b) => a - b);
}

export function thresholdOptions(current) {
    return withCurrent(THRESHOLD_CHOICES, current).map(value => ({ value: String(value), label: percentLabel(value) }));
}

export function providerThresholdOptions(defaultPercent, current) {
    const custom =
        current === null || current === PROVIDER_THRESHOLD_OFF
            ? THRESHOLD_CHOICES
            : withCurrent(THRESHOLD_CHOICES, current);
    return [
        { value: DEFAULT_THRESHOLD, label: fill(_('Default ({percent}%)'), { percent: defaultPercent }) },
        { value: String(PROVIDER_THRESHOLD_OFF), label: _('Off') },
        ...custom.map(value => ({ value: String(value), label: percentLabel(value) })),
    ];
}

export function providerThreshold(thresholds, provider) {
    return provider in thresholds ? thresholds[provider] : null;
}

export function thresholdChoice(threshold) {
    return threshold === null ? DEFAULT_THRESHOLD : String(threshold);
}

export function thresholdFromChoice(choice) {
    return choice === DEFAULT_THRESHOLD ? null : Number(choice);
}

export function thresholdProviders(accounts, thresholds, registry) {
    const found = new Map();
    for (const account of accounts) if (!found.has(account.provider)) found.set(account.provider, account.providerName);
    for (const provider of Object.keys(thresholds))
        if (!found.has(provider))
            found.set(provider, registry.find(entry => entry.id === provider)?.displayName ?? provider);
    return [...found].map(([id, name]) => ({ id, name }));
}

export function differSubtitle(providers, thresholds) {
    const count = providers.filter(provider => provider.id in thresholds).length;
    if (count === 0) return _('Every provider uses the default');
    return fill(n_('{count} provider differs from the default', '{count} providers differ from the default', count), {
        count,
    });
}

function clockValue(minutes) {
    const hours = String(Math.floor(minutes / 60)).padStart(2, '0');
    return `${hours}:${String(minutes % 60).padStart(2, '0')}`;
}

export function clockLabel(value, hour12) {
    const [hours, minutes] = value.split(':').map(Number);
    return clockTime(new Date(2000, 0, 1, hours, minutes), hour12);
}

export function clockOptions(current, hour12) {
    const values = [];
    for (let minutes = 0; minutes < DAY_MINUTES; minutes += HALF_HOUR_MINUTES) values.push(clockValue(minutes));
    if (!values.includes(current)) values.push(current);
    return values.sort().map(value => ({ value, label: clockLabel(value, hour12) }));
}

export function almostOutSubtitle(percent) {
    return fill(_('A limit drops under {percent}% left'), { percent });
}
