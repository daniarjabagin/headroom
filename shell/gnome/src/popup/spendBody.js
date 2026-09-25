import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import { _ } from '../i18n.js';
import { exactSpendLine } from '../numbers.js';
import { seriesKey } from '../providers.js';
import { column, label, row } from '../widgets.js';
import { Donut } from './donut.js';
import { MASK } from './mask.js';
import { modelTooltip } from './modelTooltip.js';
import { centerCaption, centerText, centerValue, legendText, legendValue, unitSlices } from './spendView.js';
import { NumberTween } from './tween.js';

function tweenedLabel(ctx, styleClass, render) {
    const actor = label('', styleClass);
    actor.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;
    return { actor, tween: new NumberTween(ctx.motion, actor, render) };
}

function tweenable(value) {
    return value !== null;
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
    constructor(ctx, spend, unit) {
        this._ctx = ctx;
        this._unit = unit;
        this.actor = column({ style_class: 'headroom-legend-entry headroom-hover-chip' });
        const value = tweenedLabel(ctx, 'headroom-legend-value', shown => this._text(shown));
        this._value = value.tween;
        this.actor.add_child(legendLine(spend, value.actor));
    }

    _text(shown) {
        if (this._ctx.masked) return MASK;
        return legendText(shown === null ? null : Math.round(shown), this._unit);
    }

    update(spend, morph) {
        const value = legendValue(spend, this._unit);
        this._value.set(value, morph && tweenable(value));
    }
}

function attachBreakdown(ctx, actor, titleOf, spendOf) {
    const content = () => {
        const spend = spendOf();
        const options = { series: seriesKey(spend.provider), masked: ctx.masked };
        return modelTooltip(titleOf(spend), spend, options) ?? (ctx.masked ? null : exactSpendLine(spend));
    };
    ctx.tooltips.attach(actor, content, { side: true });
}

class RingBody {
    constructor(ctx, period, unit, titleFor) {
        this._ctx = ctx;
        this._unit = unit;
        this._period = period;
        this.actor = row({ style_class: 'headroom-spend-body' });
        this._donut = new Donut();
        this._center = new NumberTween(ctx.motion, this._donut.value, value => this._centerText(value));
        this._donut.setCaption(centerCaption(unit));
        const legend = column({ style_class: 'headroom-legend', x_expand: true, y_align: Clutter.ActorAlign.CENTER });
        this._entries = period.providers.map((spend, index) => {
            const entry = new LegendEntry(ctx, spend, unit);
            attachBreakdown(ctx, entry.actor, titleFor, () => this._period.providers[index]);
            legend.add_child(entry.actor);
            return entry;
        });
        this.actor.add_child(this._donut.actor);
        this.actor.add_child(legend);
    }

    _centerText(value) {
        if (this._ctx.masked) return MASK;
        return centerText(value === null ? null : Math.round(value), this._unit);
    }

    update(period, { sweep, morph }) {
        this._period = period;
        this._donut.update(unitSlices(period, this._unit), { sweep, morph });
        const value = centerValue(period, this._unit);
        this._center.set(value, morph && tweenable(value));
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

export function createBody(ctx, period, unit, titleFor) {
    return period.providers.length === 0 ? new EmptyBody() : new RingBody(ctx, period, unit, titleFor);
}
