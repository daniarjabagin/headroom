import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import { _ } from '../i18n.js';
import { exactSpendLine, exactTokens, exactUsd, ringUsd, usd } from '../numbers.js';
import { providerInfo } from '../providers.js';
import { column, label, row } from '../widgets.js';
import { Donut } from './donut.js';
import { modelTooltip } from './modelTooltip.js';
import { NumberTween } from './tween.js';

function kindOf(period) {
    if (period.providers.length === 0) return 'empty';
    return period.providers.length === 1 ? 'stats' : 'ring';
}

export function bodyKey(period) {
    return JSON.stringify([kindOf(period), period.providers.map(spend => spend.provider)]);
}

function attachBreakdown(ctx, actor, titleOf, spendOf) {
    ctx.tooltips.attach(actor, () => modelTooltip(titleOf(), spendOf()) ?? exactSpendLine(spendOf()));
}

function tweenedLabel(ctx, styleClass, render) {
    const actor = label('', styleClass);
    return { actor, tween: new NumberTween(ctx.motion, actor, value => render(Math.round(value))) };
}

class RingBody {
    constructor(ctx, period, titleFor) {
        this._period = period;
        this.actor = row({ style_class: 'headroom-spend-body' });
        this._donut = new Donut();
        this._center = new NumberTween(ctx.motion, this._donut.value, value => ringUsd(Math.round(value)));
        const legend = column({ style_class: 'headroom-legend', x_expand: true, y_align: Clutter.ActorAlign.CENTER });
        this._values = period.providers.map((spend, index) => {
            const legendRow = row({ style_class: 'headroom-legend-row headroom-hover-chip' });
            const dot = new St.Widget({ style_class: 'headroom-legend-dot', y_align: Clutter.ActorAlign.CENTER });
            dot.style = `background-color: ${providerInfo(spend.provider).ringColor};`;
            legendRow.add_child(dot);
            legendRow.add_child(label(providerInfo(spend.provider).name, 'headroom-legend-name', { x_expand: true }));
            const value = tweenedLabel(ctx, 'headroom-legend-value', usd);
            value.actor.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;
            legendRow.add_child(value.actor);
            const spendOf = () => this._period.providers[index];
            attachBreakdown(ctx, legendRow, () => titleFor(spendOf().provider), spendOf);
            legend.add_child(legendRow);
            return value.tween;
        });
        this.actor.add_child(this._donut.actor);
        this.actor.add_child(legend);
    }

    update(period, { sweep, morph }) {
        this._period = period;
        const slices = period.providers.map(spend => ({
            value: spend.costMicros,
            color: providerInfo(spend.provider).ringColor,
        }));
        this._donut.update(slices, { sweep, morph });
        this._center.set(period.costMicros, morph);
        period.providers.forEach((spend, index) => this._values[index].set(spend.costMicros, morph));
    }
}

function statColumn(value, caption) {
    const actor = column({ style_class: 'headroom-stat', x_expand: true });
    actor.add_child(value);
    actor.add_child(label(caption, 'headroom-stat-caption'));
    return actor;
}

class StatsBody {
    constructor(ctx, period, titleFor) {
        this._period = period;
        this.actor = row({ style_class: 'headroom-spend-stats headroom-hover-chip' });
        const provider = period.providers[0].provider;
        const cost = tweenedLabel(ctx, 'headroom-stat-value', exactUsd);
        const tokens = tweenedLabel(ctx, 'headroom-stat-value', exactTokens);
        this._cost = cost.tween;
        this._tokens = tokens.tween;
        this.actor.add_child(statColumn(cost.actor, `${_('dollars')} · ${providerInfo(provider).name}`));
        this.actor.add_child(statColumn(tokens.actor, _('tokens')));
        const spendOf = () => this._period.providers[0];
        attachBreakdown(ctx, this.actor, () => titleFor(provider), spendOf);
    }

    update(period, { morph }) {
        this._period = period;
        this._cost.set(period.costMicros, morph);
        this._tokens.set(period.totalTokens, morph);
    }
}

class EmptyBody {
    constructor() {
        this.actor = label(_('No usage in this period'), 'headroom-empty-period', {
            x_align: Clutter.ActorAlign.CENTER,
        });
    }

    update() {}
}

export function createBody(ctx, period, titleFor) {
    const kind = kindOf(period);
    if (kind === 'empty') return new EmptyBody();
    if (kind === 'stats') return new StatsBody(ctx, period, titleFor);
    return new RingBody(ctx, period, titleFor);
}
