import { isObject, list, number, oneOf, text, TONES } from './fields.js';

const PROVIDER_ID = /^[a-z0-9_-]+$/;
const MAX_ITEMS = 3;

export function parseHeadline(raw) {
    if (!isObject(raw)) return null;
    const remainingPercent = number(raw.remaining_percent);
    if (remainingPercent === null) return null;
    return {
        accountId: text(raw.account_id),
        windowId: text(raw.window),
        provider: text(raw.provider),
        providerName: text(raw.provider_name) ?? text(raw.provider),
        accountLabel: text(raw.account_label),
        windowLabel: text(raw.window_label),
        usedPercent: number(raw.used_percent),
        remainingPercent,
        tone: oneOf(TONES, raw.tone, 'neutral'),
        combined: raw.combined === true,
        accountCount: number(raw.account_count),
    };
}

function logoKey(value, provider) {
    const key = text(value) ?? provider;
    return typeof key === 'string' && PROVIDER_ID.test(key) ? key : null;
}

function headlineValue(headline, valueMode) {
    if (valueMode === 'used') return headline.usedPercent ?? 100 - headline.remainingPercent;
    return headline.remainingPercent;
}

function parsePanelItem(raw) {
    const headline = parseHeadline(raw);
    if (headline === null) return null;
    return {
        ...headline,
        valuePercent: number(raw.value_percent) ?? headline.remainingPercent,
        evenPacePercent: number(raw.even_pace_percent),
        logo: logoKey(raw.logo, headline.provider),
    };
}

function headlineItem(headline, display) {
    return {
        ...headline,
        valuePercent: headlineValue(headline, display.valueMode),
        evenPacePercent: null,
        logo: logoKey(null, headline.provider),
    };
}

export function reportsPanelItems(raw) {
    return Array.isArray(raw.panel_items);
}

export function parsePanelItems(raw, headline, display) {
    if (!reportsPanelItems(raw)) return headline ? [headlineItem(headline, display)] : [];
    return list(raw.panel_items).map(parsePanelItem).filter(Boolean).slice(0, MAX_ITEMS);
}

export function parsePanelTone(raw, headline) {
    if (!('panel_tone' in raw)) return headline?.tone ?? null;
    return oneOf(TONES, raw.panel_tone, null);
}
