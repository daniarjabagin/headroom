import { _, currentLanguage, fill, n_ } from './i18n.js';

const MICROS_PER_CENT = 10000;
const CURRENCY_SYMBOLS = { USD: '$', CNY: '¥', EUR: '€' };
const COMPACT_FROM = 1000;

const UNITS = [
    [1e9, 'B'],
    [1e6, 'M'],
    [1e3, 'K'],
];

const RU_UNITS = { B: '\u00a0млрд', M: '\u00a0млн', K: '\u00a0тыс.' };
const RU_GROUP = '\u00a0';

const tokenDigits = scaled => (scaled >= 100 ? 0 : 1);
const moneyDigits = scaled => (scaled >= 100 ? 0 : scaled >= 10 ? 1 : 2);

function abbreviate(value, digitsFor) {
    for (const [size, suffix] of UNITS) {
        if (value >= size) {
            const scaled = value / size;
            const text = scaled.toFixed(digitsFor(scaled)).replace(/\.0+$/, '');
            return { text, suffix };
        }
    }
    return { text: String(value), suffix: '' };
}

function groupDigits(value, separator) {
    return String(value).replace(/\B(?=(\d{3})+(?!\d))/g, separator);
}

export function compactTokens(count) {
    const { text, suffix } = abbreviate(count, tokenDigits);
    if (currentLanguage() !== 'ru') return `${text}${suffix}`;
    return `${text.replace('.', ',')}${RU_UNITS[suffix] ?? ''}`;
}

export function exactTokens(count) {
    return groupDigits(count, currentLanguage() === 'ru' ? RU_GROUP : ',');
}

function unitWordCount(count) {
    return count < COMPACT_FROM ? count : 0;
}

export function compactTokensText(count) {
    return fill(n_('{tokens} token', '{tokens} tokens', unitWordCount(count)), { tokens: compactTokens(count) });
}

export function exactTokensText(count) {
    return fill(n_('{tokens} token', '{tokens} tokens', count), { tokens: exactTokens(count) });
}

function centsOf(micros) {
    const cents = Math.round(Math.abs(micros) / MICROS_PER_CENT);
    return micros < 0 ? -cents : cents;
}

export function money(currency, micros) {
    const cents = centsOf(micros);
    const whole = Math.abs(cents);
    const digits = `${groupDigits(Math.trunc(whole / 100), ',')}.${String(whole % 100).padStart(2, '0')}`;
    const sign = cents < 0 ? '-' : '';
    const symbol = CURRENCY_SYMBOLS[currency];
    return symbol ? `${sign}${symbol}${digits}` : `${sign}${digits} ${currency}`;
}

export function exactUsd(micros) {
    return money('USD', micros);
}

export function usd(micros) {
    const cents = centsOf(micros);
    if (cents < 100000) return exactUsd(micros);
    const { text, suffix } = abbreviate(cents / 100, moneyDigits);
    return `$${text}${suffix}`;
}

export function ringUsd(micros) {
    const cents = centsOf(micros);
    if (cents < 10000) return usd(micros);
    if (cents < 1000000) return `$${Math.round(cents / 100)}`;
    return usd(micros);
}

export function spendLine(totals) {
    if (totals.costMicros === 0 && totals.totalTokens === 0) return _('No data');
    return `${usd(totals.costMicros)} · ${compactTokensText(totals.totalTokens)}`;
}

export function exactSpendLine(totals) {
    const partial = totals.partial ? _(' · some models unpriced') : '';
    return `${exactUsd(totals.costMicros)} · ${exactTokensText(totals.totalTokens)}${partial}`;
}
