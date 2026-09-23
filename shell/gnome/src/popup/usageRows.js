import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { compactTokens, exactTokens, exactUsd, spendLine, usd } from '../format.js';
import { column, label, row } from '../widgets.js';

const TREND_DAYS = 30;
const TREND_HEIGHT = 18;
const TREND_STUB = 2;
const TREND_MIN_SHARE = 0.18;

function lastDays(daily) {
    const days = daily.slice(-TREND_DAYS);
    const padding = Array.from({ length: TREND_DAYS - days.length }, () => ({ date: '', totalTokens: 0 }));
    return [...padding, ...days];
}

function barHeight(value, peak) {
    if (value <= 0 || peak <= 0) return TREND_STUB;
    return Math.max(Math.round(TREND_HEIGHT * TREND_MIN_SHARE), Math.round((TREND_HEIGHT * value) / peak));
}

function peakDescription(days) {
    const peakDay = days.reduce((best, day) => (day.totalTokens > best.totalTokens ? day : best), days[0]);
    if (peakDay.totalTokens === 0) return 'No usage in the last 30 days';
    return `Peak ${compactTokens(peakDay.totalTokens)} tokens on ${peakDay.date}`;
}

export function trendRow(ctx, usage) {
    const days = lastDays(usage.daily);
    const peak = Math.max(...days.map(day => day.totalTokens));
    const actor = row({ style_class: 'headroom-text-row' });
    actor.add_child(label('Usage Trend', 'headroom-value-label', { x_expand: true }));
    const strip = row({ style_class: 'headroom-trend', y_align: Clutter.ActorAlign.CENTER });
    for (const day of days) {
        const bar = new St.Widget({ style_class: 'headroom-trend-bar', y_align: Clutter.ActorAlign.END });
        bar.style = `height: ${barHeight(day.totalTokens, peak)}px;`;
        strip.add_child(bar);
    }
    ctx.tooltips.attach(strip, () => peakDescription(days));
    actor.add_child(strip);
    return actor;
}

function valueRow(ctx, title, value, tooltip) {
    const actor = row({ style_class: 'headroom-text-row' });
    actor.add_child(label(title, 'headroom-value-label', { x_expand: true }));
    const valueLabel = label(value, 'headroom-value');
    if (tooltip) ctx.tooltips.attach(valueLabel, () => tooltip);
    actor.add_child(valueLabel);
    return actor;
}

function spendTooltip(totals) {
    if (totals.totalTokens === 0) return null;
    const partial = totals.partial ? ' · some models unpriced' : '';
    return `${exactUsd(totals.costMicros)} · ${exactTokens(totals.totalTokens)} tokens${partial}`;
}

function spendRows(ctx, usage) {
    return [
        ['Today', usage.today],
        ['Yesterday', usage.yesterday],
        ['Last 30 Days', usage.last30Days],
    ].map(([title, totals]) => valueRow(ctx, title, spendLine(totals), spendTooltip(totals)));
}

function balanceValue(balance) {
    if (balance.kind === 'usd' && balance.usdMicros !== null) return usd(balance.usdMicros);
    if (balance.kind === 'count' && balance.value !== null)
        return `${exactTokens(balance.value)} ${balance.unit ?? ''}`.trim();
    return 'No data';
}

function balanceRows(ctx, balances) {
    return balances.map(balance => valueRow(ctx, balance.label, balanceValue(balance), null));
}

export function expandedRows(ctx, account) {
    const rows = column({ style_class: 'headroom-expanded', x_expand: true });
    const children = [...(account.usage ? spendRows(ctx, account.usage) : []), ...balanceRows(ctx, account.balances)];
    for (const child of children) rows.add_child(child);
    return rows;
}
