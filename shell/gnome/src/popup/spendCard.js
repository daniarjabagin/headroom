import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { exactTokens, exactUsd, ringUsd, usd } from '../format.js';
import { _ } from '../i18n.js';
import { providerInfo } from '../providers.js';
import { column, label, row, spacer, textButton, themeIcon } from '../widgets.js';
import { Donut } from './donut.js';
import { modelTooltip } from './modelTooltip.js';

const PERIODS = [
    ['today', () => _('Today')],
    ['yesterday', () => _('Yesterday')],
    ['last30Days', () => _('30 Days')],
];

function periodTitle(key) {
    return PERIODS.find(([candidate]) => candidate === key)[1]();
}

function spendTooltip(spend) {
    const partial = spend.partial ? _(' · some models unpriced') : '';
    return `${exactUsd(spend.costMicros)} · ${exactTokens(spend.totalTokens)} ${_('tokens')}${partial}`;
}

function attachBreakdown(ctx, actor, spend) {
    const title = `${periodTitle(ctx.period)} · ${providerInfo(spend.provider).name}`;
    ctx.tooltips.attach(actor, () => modelTooltip(title, spend) ?? spendTooltip(spend));
}

function legendRow(ctx, spend) {
    const info = providerInfo(spend.provider);
    const actor = row({ style_class: 'headroom-legend-row headroom-hover-chip' });
    const dot = new St.Widget({ style_class: 'headroom-legend-dot', y_align: Clutter.ActorAlign.CENTER });
    dot.style = `background-color: ${info.ringColor};`;
    actor.add_child(dot);
    actor.add_child(label(info.name, 'headroom-legend-name', { x_expand: true }));
    actor.add_child(label(usd(spend.costMicros), 'headroom-legend-value'));
    attachBreakdown(ctx, actor, spend);
    return actor;
}

function ringBody(ctx, period) {
    const actor = row({ style_class: 'headroom-spend-body' });
    const donut = new Donut();
    donut.update(
        period.providers.map(spend => ({ value: spend.costMicros, color: providerInfo(spend.provider).ringColor })),
        ringUsd(period.costMicros)
    );
    const legend = column({ style_class: 'headroom-legend', x_expand: true, y_align: Clutter.ActorAlign.CENTER });
    for (const spend of period.providers) legend.add_child(legendRow(ctx, spend));
    actor.add_child(donut.actor);
    actor.add_child(legend);
    return actor;
}

function statColumn(value, caption) {
    const actor = column({ style_class: 'headroom-stat', x_expand: true });
    actor.add_child(label(value, 'headroom-stat-value'));
    actor.add_child(label(caption, 'headroom-stat-caption'));
    return actor;
}

function statsBody(ctx, period) {
    const actor = row({ style_class: 'headroom-spend-stats headroom-hover-chip' });
    const provider = period.providers[0];
    const caption = `${_('dollars')} · ${providerInfo(provider.provider).name}`;
    actor.add_child(statColumn(exactUsd(period.costMicros), caption));
    actor.add_child(statColumn(exactTokens(period.totalTokens), _('tokens')));
    attachBreakdown(ctx, actor, provider);
    return actor;
}

function emptyBody() {
    return label(_('No usage in this period'), 'headroom-empty-period', { x_align: Clutter.ActorAlign.CENTER });
}

function periodBody(ctx, period) {
    if (period.providers.length === 0) return emptyBody();
    if (period.providers.length === 1) return statsBody(ctx, period);
    return ringBody(ctx, period);
}

function header() {
    const actor = row({ style_class: 'headroom-section-header spend' });
    actor.add_child(label(_('Total Spend'), 'headroom-title'));
    const info = themeIcon('help-about-symbolic', 'headroom-info-icon');
    actor.add_child(info);
    actor.add_child(spacer());
    return [actor, info];
}

export class SpendSection {
    constructor(ctx, spend, period) {
        this._ctx = ctx;
        this._spend = spend;
        this._period = period;
        this._body = null;
        this.actor = column({ style_class: 'headroom-section', x_expand: true });
        const [headerRow, info] = header();
        ctx.tooltips.attach(info, () => this._infoText());
        this._card = column({ style_class: 'headroom-card headroom-spend-card', x_expand: true });
        this._segments = new Map();
        this._card.add_child(this._segmentedControl());
        this.actor.add_child(headerRow);
        this.actor.add_child(this._card);
        this._showPeriod();
    }

    update(spend) {
        this._spend = spend;
        this._showPeriod();
    }

    _infoText() {
        const partial = this._spend[this._period].partial ? ` ${_('Some models have no public price yet.')}` : '';
        return `${_('Estimated from local logs and public pricing.')}${partial}`;
    }

    _segmentedControl() {
        const actor = row({ style_class: 'headroom-segmented', x_expand: true });
        for (const [key, title] of PERIODS) {
            const segment = textButton(title(), 'headroom-segment', () => this._select(key));
            segment.x_expand = true;
            this._segments.set(key, segment);
            actor.add_child(segment);
        }
        return actor;
    }

    _select(period) {
        this._period = period;
        this._ctx.selectPeriod(period);
        this._showPeriod();
    }

    _showPeriod() {
        for (const [key, segment] of this._segments) {
            if (key === this._period) segment.add_style_pseudo_class('checked');
            else segment.remove_style_pseudo_class('checked');
        }
        this._body?.destroy();
        this._body = periodBody(this._ctx, this._spend[this._period]);
        this._card.add_child(this._body);
    }
}
