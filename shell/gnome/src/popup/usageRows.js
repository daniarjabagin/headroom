import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { compactTokens, exactTokens, exactUsd, spendLine, usd } from '../format.js';
import { _, fill } from '../i18n.js';
import { providerInfo } from '../providers.js';
import { column, label, row } from '../widgets.js';
import { modelTooltip } from './modelTooltip.js';

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
    if (peakDay.totalTokens === 0) return _('No usage in the last 30 days');
    return fill(_('Peak {tokens} tokens on {date}'), {
        tokens: compactTokens(peakDay.totalTokens),
        date: peakDay.date,
    });
}

export function trendRow(ctx, usage) {
    const days = lastDays(usage.daily);
    const peak = Math.max(...days.map(day => day.totalTokens));
    const actor = row({ style_class: 'headroom-text-row' });
    actor.add_child(label(_('Usage Trend'), 'headroom-value-label', { x_expand: true }));
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

function valueRow(ctx, title, value, tooltipFor) {
    const actor = row({ style_class: 'headroom-text-row' });
    actor.add_child(label(title, 'headroom-value-label', { x_expand: true }));
    const valueLabel = label(value, 'headroom-value');
    if (tooltipFor) {
        valueLabel.add_style_class_name('headroom-hover-chip');
        ctx.tooltips.attach(valueLabel, tooltipFor);
    }
    actor.add_child(valueLabel);
    return actor;
}

function spendTooltip(totals) {
    const partial = totals.partial ? _(' · some models unpriced') : '';
    return `${exactUsd(totals.costMicros)} · ${exactTokens(totals.totalTokens)} ${_('tokens')}${partial}`;
}

function totalsTooltip(title, totals) {
    if (totals.totalTokens === 0) return null;
    return () => modelTooltip(title, totals) ?? spendTooltip(totals);
}

function spendRows(ctx, account) {
    const provider = providerInfo(account.provider).name;
    return [
        [_('Today'), account.usage.today],
        [_('Yesterday'), account.usage.yesterday],
        [_('Last 30 Days'), account.usage.last30Days],
    ].map(([title, totals]) =>
        valueRow(ctx, title, spendLine(totals), totalsTooltip(`${title} · ${provider}`, totals))
    );
}

function balanceValue(balance) {
    if (balance.kind === 'usd' && balance.usdMicros !== null) return usd(balance.usdMicros);
    if (balance.kind === 'count' && balance.value !== null)
        return `${exactTokens(balance.value)} ${balance.unit ?? ''}`.trim();
    return _('No data');
}

function balanceRows(ctx, balances) {
    return balances.map(balance => valueRow(ctx, balance.label, balanceValue(balance), null));
}

export function showsSpend(ctx, account) {
    return account.usage !== null && ctx.display.showAccountSpend;
}

export function expandedRows(ctx, account) {
    const rows = column({ style_class: 'headroom-expanded', x_expand: true });
    const children = [
        ...(showsSpend(ctx, account) ? spendRows(ctx, account) : []),
        ...balanceRows(ctx, account.balances),
    ];
    for (const child of children) rows.add_child(child);
    return rows;
}
