import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { dayTitle } from '../dates.js';
import { _ } from '../i18n.js';
import { compactTokensText, exactSpendLine, exactTokens, spendLine, usd } from '../numbers.js';
import { providerInfo } from '../providers.js';
import { column, label, row } from '../widgets.js';
import { modelTooltip } from './modelTooltip.js';

const TREND_DAYS = 30;
const TREND_HEIGHT = 18;
const TREND_STUB = 2;
const TREND_MIN_SHARE = 0.18;

const BALANCE_LABELS = {
    credits: () => _('Credits'),
    extra_usage: () => _('Extra usage'),
};

function lastDays(daily) {
    const days = daily.slice(-TREND_DAYS);
    const padding = Array.from({ length: TREND_DAYS - days.length }, () => ({
        date: '',
        totalTokens: 0,
        costMicros: 0,
        partial: false,
    }));
    return [...padding, ...days];
}

function barHeight(value, peak) {
    if (value <= 0 || peak <= 0) return TREND_STUB;
    return Math.max(Math.round(TREND_HEIGHT * TREND_MIN_SHARE), Math.round((TREND_HEIGHT * value) / peak));
}

function dayTooltip(day) {
    if (day.date === '') return null;
    const actor = column({ style_class: 'headroom-tip' });
    actor.add_child(label(dayTitle(day.date), 'headroom-tip-title', { x_align: Clutter.ActorAlign.START }));
    const partial = day.partial ? _(' · some models unpriced') : '';
    const figures =
        day.totalTokens === 0 ? _('No usage') : `${compactTokensText(day.totalTokens)} · ${usd(day.costMicros)}`;
    actor.add_child(label(`${figures}${partial}`, 'headroom-tip-figure', { x_align: Clutter.ActorAlign.START }));
    return actor;
}

export class TrendRow {
    constructor(ctx, usage) {
        this._days = lastDays(usage.daily);
        this.actor = row({ style_class: 'headroom-text-row' });
        this.actor.add_child(label(_('Usage Trend'), 'headroom-value-label', { x_expand: true }));
        const strip = row({ style_class: 'headroom-trend', y_align: Clutter.ActorAlign.CENTER });
        this._bars = this._days.map((_day, index) => {
            const slot = new St.Widget({ style_class: 'headroom-trend-slot', layout_manager: new Clutter.BinLayout() });
            const bar = new St.Widget({
                style_class: 'headroom-trend-bar',
                y_expand: true,
                y_align: Clutter.ActorAlign.END,
            });
            slot.add_child(bar);
            ctx.tooltips.attach(slot, () => dayTooltip(this._days[index]));
            strip.add_child(slot);
            return bar;
        });
        this.actor.add_child(strip);
        this.update(usage);
    }

    update(usage) {
        this._days = lastDays(usage.daily);
        const peak = Math.max(...this._days.map(day => day.totalTokens));
        this._days.forEach((day, index) => {
            this._bars[index].style = `height: ${barHeight(day.totalTokens, peak)}px;`;
        });
    }
}

function totalsTooltip(title, totals) {
    if (totals.totalTokens === 0) return null;
    return modelTooltip(title, totals) ?? exactSpendLine(totals);
}

function balanceTitle(balance) {
    return BALANCE_LABELS[balance.id]?.() ?? balance.label;
}

function balanceValue(balance) {
    if (balance.kind === 'usd' && balance.usdMicros !== null) return usd(balance.usdMicros);
    if (balance.kind === 'count' && balance.value !== null)
        return `${exactTokens(balance.value)} ${balance.unit ?? ''}`.trim();
    return _('No data');
}

export function showsSpend(ctx, account) {
    return account.usage !== null && ctx.display.showAccountSpend;
}

export class ExtraRows {
    constructor(ctx, account) {
        this._ctx = ctx;
        this._account = account;
        this.actor = column({ style_class: 'headroom-expanded', x_expand: true });
        this._values = [...this._spendEntries(), ...this._balanceEntries()].map(entry => this._valueRow(entry));
        this.update(account);
    }

    update(account) {
        this._account = account;
        const texts = [...this._spendEntries(), ...this._balanceEntries()].map(entry => entry.value());
        texts.forEach((text, index) => (this._values[index].text = text));
    }

    _spendEntries() {
        if (!showsSpend(this._ctx, this._account)) return [];
        const provider = providerInfo(this._account.provider).name;
        return [
            ['today', _('Today')],
            ['yesterday', _('Yesterday')],
            ['last30Days', _('Last 30 Days')],
        ].map(([key, title]) => ({
            title,
            value: () => spendLine(this._account.usage[key]),
            tooltip: () => totalsTooltip(`${title} · ${provider}`, this._account.usage[key]),
        }));
    }

    _balanceEntries() {
        return this._account.balances.map((_balance, index) => ({
            title: balanceTitle(this._account.balances[index]),
            value: () => balanceValue(this._account.balances[index]),
            tooltip: null,
        }));
    }

    _valueRow(entry) {
        const actor = row({ style_class: 'headroom-text-row' });
        actor.add_child(label(entry.title, 'headroom-value-label', { x_expand: true }));
        const valueLabel = label('', 'headroom-value');
        if (entry.tooltip) {
            valueLabel.add_style_class_name('headroom-hover-chip');
            this._ctx.tooltips.attach(valueLabel, entry.tooltip);
        }
        actor.add_child(valueLabel);
        this.actor.add_child(actor);
        return valueLabel;
    }
}
