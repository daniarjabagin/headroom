import { _ } from '../i18n.js';
import { column, label, row, spacer, textButton, themeIcon } from '../widgets.js';
import { bodyKey, createBody } from './spendBody.js';

const PERIODS = [
    ['today', () => _('Today')],
    ['yesterday', () => _('Yesterday')],
    ['last30Days', () => _('30 Days')],
];

function periodTitle(key) {
    return PERIODS.find(([candidate]) => candidate === key)[1]();
}

function header(trailing) {
    const actor = row({ style_class: 'headroom-section-header spend' });
    actor.add_child(label(_('Total Spend'), 'headroom-title'));
    const info = themeIcon('help-about-symbolic', 'headroom-info-icon');
    actor.add_child(info);
    actor.add_child(spacer());
    actor.add_child(trailing);
    return [actor, info];
}

export class SpendSection {
    constructor(ctx, spend, period, trailing) {
        this._ctx = ctx;
        this._spend = spend;
        this._period = period;
        this._body = null;
        this._bodyKey = null;
        this.actor = column({ style_class: 'headroom-section', x_expand: true });
        const [headerRow, info] = header(trailing);
        ctx.tooltips.attach(info, () => this._infoText());
        this._card = column({
            style_class: 'headroom-card headroom-spend-card',
            x_expand: true,
            reactive: true,
            track_hover: true,
        });
        this._segments = new Map();
        this._card.add_child(this._segmentedControl());
        this.actor.add_child(headerRow);
        this.actor.add_child(this._card);
        this._showPeriod(false);
    }

    update(spend) {
        this._spend = spend;
        this._showPeriod(true);
    }

    grow() {
        this._body?.update(this._spend[this._period], { sweep: this._ctx.motion.enabled, morph: false });
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
        this._showPeriod(true);
    }

    _titleFor(spend) {
        return `${periodTitle(this._period)} · ${spend.providerName}`;
    }

    _showPeriod(animate) {
        for (const [key, segment] of this._segments) {
            if (key === this._period) segment.add_style_pseudo_class('checked');
            else segment.remove_style_pseudo_class('checked');
        }
        const period = this._spend[this._period];
        const key = bodyKey(period);
        const morph = animate && this._ctx.motion.enabled && key === this._bodyKey;
        if (key !== this._bodyKey) {
            this._body?.actor.destroy();
            this._body = createBody(this._ctx, period, spend => this._titleFor(spend));
            this._bodyKey = key;
            this._card.add_child(this._body.actor);
        }
        this._body.update(period, { sweep: false, morph });
    }
}
