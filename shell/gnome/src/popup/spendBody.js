import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import { _ } from '../i18n.js';
import { exactSpendLine, exactTokensText, ringUsd, usd } from '../numbers.js';
import { seriesKey } from '../providers.js';
import { bodyKind, ringSlices, showsTokenLine } from '../spendShape.js';
import { column, label, row } from '../widgets.js';
import { Donut } from './donut.js';
import { modelTooltip } from './modelTooltip.js';
import { NumberTween } from './tween.js';

function attachBreakdown(ctx, actor, titleOf, spendOf) {
    ctx.tooltips.attach(actor, () => modelTooltip(titleOf(), spendOf()) ?? exactSpendLine(spendOf()));
}

function tweenedLabel(ctx, styleClass, render) {
    const actor = label('', styleClass);
    actor.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;
    return { actor, tween: new NumberTween(ctx.motion, actor, value => render(Math.round(value))) };
}

function legendLine(spend, valueActor) {
    const line = row({ style_class: 'headroom-legend-row' });
    line.add_child(
        new St.Widget({
            style_class: `headroom-legend-dot headroom-series-${seriesKey(spend.provider)}`,
            y_align: Clutter.ActorAlign.CENTER,
        })
    );
    line.add_child(label(spend.providerName, 'headroom-legend-name', { x_expand: true }));
    line.add_child(valueActor);
    return line;
}

class LegendEntry {
    constructor(ctx, spend, withTokens) {
        this.actor = column({ style_class: 'headroom-legend-entry headroom-hover-chip' });
        const cost = tweenedLabel(ctx, 'headroom-legend-value', usd);
        this._cost = cost.tween;
        this.actor.add_child(legendLine(spend, cost.actor));
        this._tokens = null;
        if (!withTokens) return;
        const tokens = tweenedLabel(ctx, 'headroom-legend-tokens', exactTokensText);
        this._tokens = tokens.tween;
        this.actor.add_child(tokens.actor);
    }

    update(spend, morph) {
        this._cost.set(spend.costMicros, morph);
        this._tokens?.set(spend.totalTokens, morph);
    }
}

class RingBody {
    constructor(ctx, period, titleFor) {
        this._period = period;
        this.actor = row({ style_class: 'headroom-spend-body' });
        this._donut = new Donut();
        this._center = new NumberTween(ctx.motion, this._donut.value, value => ringUsd(Math.round(value)));
        const legend = column({ style_class: 'headroom-legend', x_expand: true, y_align: Clutter.ActorAlign.CENTER });
        const withTokens = showsTokenLine(period);
        this._entries = period.providers.map((spend, index) => {
            const entry = new LegendEntry(ctx, spend, withTokens);
            const spendOf = () => this._period.providers[index];
            attachBreakdown(ctx, entry.actor, () => titleFor(spendOf()), spendOf);
            legend.add_child(entry.actor);
            return entry;
        });
        this.actor.add_child(this._donut.actor);
        this.actor.add_child(legend);
    }

    update(period, { sweep, morph }) {
        this._period = period;
        this._donut.update(ringSlices(period), { sweep, morph });
        this._center.set(period.costMicros, morph);
        period.providers.forEach((spend, index) => this._entries[index].update(spend, morph));
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
    return bodyKind(period) === 'empty' ? new EmptyBody() : new RingBody(ctx, period, titleFor);
}
