import { panelCount, panelPercent, shortWindowLabel } from './format.js';

const LOUD_TONES = ['warning', 'critical'];
const LIGHT_PANEL_LUMINANCE = 0.5;

export function itemKey(item) {
    return `${item.accountId ?? ''}\u0000${item.windowId ?? ''}`;
}

export function sameKeys(a, b) {
    return a.length === b.length && a.every((key, index) => key === b[index]);
}

export function loudTone(tone) {
    return LOUD_TONES.includes(tone) ? tone : null;
}

function markOnly(tone) {
    return { mark: { tone: loudTone(tone) }, items: [], style: 'mark' };
}

function fraction(percent) {
    return Math.min(1, Math.max(0, percent / 100));
}

function tickFraction(item, valueMode) {
    if (item.evenPacePercent === null) return null;
    const even = fraction(item.evenPacePercent);
    return valueMode === 'used' ? even : 1 - even;
}

function indicatorKind(display, legacy) {
    if (display.panelIndicator === 'none') return null;
    if (legacy && display.panelLabel === 'window') return null;
    return display.panelIndicator;
}

function isStale(state, item) {
    if (state.offline) return true;
    return state.accounts.find(account => account.id === item.accountId)?.status === 'stale';
}

function itemLayout(state, item, indicator, legacy) {
    const display = state.display;
    const withLogo = display.panelMode === 'several' || display.panelLabel === 'window';
    const counted = withLogo && item.combined && item.accountCount !== null;
    return {
        key: itemKey(item),
        logo: withLogo ? (item.logo ?? item.provider ?? 'unknown') : null,
        letter: withLogo ? shortWindowLabel(item.windowId, item.windowLabel) : null,
        count: counted ? panelCount(item.accountCount) : null,
        indicator,
        fraction: fraction(item.valuePercent),
        tick: indicator === 'bar' && !legacy ? tickFraction(item, display.valueMode) : null,
        value: display.panelLabel === 'none' ? null : panelPercent(item.valuePercent),
        tone: item.tone,
        stale: isStale(state, item),
    };
}

export function panelLayout(state, { masked = false, legacy = false } = {}) {
    if (!state || masked) return markOnly(null);
    const display = state.display;
    if (display.panelMode === 'icon') return markOnly(state.panelTone);
    if (state.panelItems.length === 0) return markOnly(null);
    const indicator = indicatorKind(display, legacy);
    if (indicator === null && display.panelLabel === 'none') return markOnly(null);
    return {
        mark: null,
        items: state.panelItems.map(item => itemLayout(state, item, indicator, legacy)),
        style: `${display.panelMode}:${indicator}:${display.panelLabel}`,
    };
}

function channel(value) {
    const unit = value / 255;
    return unit <= 0.04045 ? unit / 12.92 : ((unit + 0.055) / 1.055) ** 2.4;
}

export function isLightPanel(foreground) {
    const luminance =
        0.2126 * channel(foreground.red) + 0.7152 * channel(foreground.green) + 0.0722 * channel(foreground.blue);
    return luminance < LIGHT_PANEL_LUMINANCE;
}
