import { combinedMeterState, combinedPercent, groupDetail } from '../../combined.js';
import {
    capacityReading,
    isPooled,
    panelPercent,
    percentReading,
    readingPercent,
    resetPhrase,
    windowLabel,
} from '../../format.js';
import { _, fill } from '../../i18n.js';
import { fillFraction, hasData, meterTone, tickPosition } from '../../quota.js';
import { trailingLabel } from '../limitTexts.js';

const HEADLINE_WINDOW = 'weekly';

const MONTHS = () => [
    _('Jan'),
    _('Feb'),
    _('Mar'),
    _('Apr'),
    _('May'),
    _('Jun'),
    _('Jul'),
    _('Aug'),
    _('Sep'),
    _('Oct'),
    _('Nov'),
    _('Dec'),
];

function cardWindows(card) {
    const windows = card.kind === 'combined' ? card.group.windows : card.account.windows;
    return windows.filter(window => !window.hidden && hasData(window));
}

function cardIdentity(card) {
    if (card.kind === 'combined')
        return {
            provider: card.group.provider,
            providerName: card.group.providerName,
            detail: groupDetail(card.group),
        };
    const account = card.account;
    return { provider: account.provider, providerName: account.providerName, detail: account.plan };
}

function windowPercent(card, window, valueMode) {
    return card.kind === 'combined' ? combinedPercent(window, valueMode) : readingPercent(window, valueMode);
}

function windowReading(card, window, valueMode) {
    const percent = windowPercent(card, window, valueMode);
    if (card.kind === 'combined' && isPooled(window))
        return capacityReading(percent, window.capacityPercent, valueMode);
    return percentReading(percent, valueMode);
}

function windowSegments(card, window, display) {
    if (card.kind === 'combined') return combinedMeterState(window, card.members, display).segments;
    return [
        {
            fraction: fillFraction(window, display.valueMode),
            tone: meterTone(window),
            tick: tickPosition(window, display),
        },
    ];
}

function shareRow(card, window, ctx) {
    return {
        label: windowLabel(window.id, window.label),
        segments: windowSegments(card, window, ctx.display),
        reading: windowReading(card, window, ctx.display.valueMode),
        reset: trailingLabel(window, ctx.now, 'countdown', ctx.hour12),
    };
}

function headlineSub(card, window, ctx) {
    const name = windowLabel(window.id, window.label).toLowerCase();
    const reset = window.resetsAt === null ? null : resetPhrase(window.resetsAt, ctx.now, 'countdown', false);
    const pooled = card.kind === 'combined' && isPooled(window);
    const scope = pooled
        ? fill(_('of {capacity}% {window}'), { capacity: Math.round(window.capacityPercent), window: name })
        : name;
    return reset ? `${scope} · ${reset}` : scope;
}

function headline(card, window, ctx) {
    const valueMode = ctx.display.valueMode;
    return {
        percent: panelPercent(windowPercent(card, window, valueMode)),
        word: valueMode === 'used' ? _('used') : _('left'),
        sub: headlineSub(card, window, ctx),
        segments: windowSegments(card, window, ctx.display),
    };
}

export function shareDate(providerName, now) {
    const date = `${now.getDate()} ${MONTHS()[now.getMonth()]} ${now.getFullYear()}`;
    return fill(_('{provider} limits · {date}'), { provider: providerName, date }).toUpperCase();
}

export function shareModel(card, ctx) {
    const windows = cardWindows(card);
    if (windows.length === 0) return null;
    const lead = windows.find(window => window.id === HEADLINE_WINDOW) ?? windows[0];
    const identity = cardIdentity(card);
    return {
        ...identity,
        date: shareDate(identity.providerName, ctx.now),
        headline: headline(card, lead, ctx),
        rows: windows.map(window => shareRow(card, window, ctx)),
    };
}

export function shareText(model) {
    const title = model.detail ? `${model.providerName} · ${model.detail}` : model.providerName;
    const rows = model.rows.map(row => `${row.label}: ${row.reading} · ${row.reset}`);
    return [title, ...rows, _('— via Headroom')].join('\n');
}
