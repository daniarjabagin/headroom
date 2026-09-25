import { _ } from '../i18n.js';
import { button, column, label, row, spacer, textButton, themeIcon } from '../widgets.js';
import { createBody } from './spendBody.js';
import { SpendBreakdown } from './spendBreakdown.js';
import {
    legendTitle,
    periodChoices,
    periodData,
    periodTitle,
    shownPeriod,
    spendBodyKey,
    unitHint,
    UNITS,
    unitTitle,
} from './spendView.js';

const MENU_GAP = 4;

function infoText(unit, period) {
    const base =
        unit === 'tokens' ? _('Tokens counted from local logs.') : _('Estimated from local logs and public pricing.');
    const partial = period.partial && unit !== 'tokens' ? ` ${_('Some models have no public price yet.')}` : '';
    return `${base}${partial}`;
}

function titlePull(onClick) {
    const content = row({ style_class: 'headroom-title-pull-box' });
    const title = label('', 'headroom-title');
    content.add_child(title);
    content.add_child(themeIcon('pan-down-symbolic', 'headroom-title-pull-icon'));
    const actor = button(content, 'headroom-title-pull', () => onClick());
    return { actor, title };
}

export class SpendSection {
    constructor(ctx, spend, trailing) {
        this._ctx = ctx;
        this._spend = spend;
        this._body = null;
        this._bodyKey = null;
        this._segmentsKey = null;
        this.actor = column({ style_class: 'headroom-section', x_expand: true });
        this.actor.add_child(this._header(trailing));
        this._card = column({ style_class: 'headroom-card headroom-spend-card', x_expand: true, reactive: true });
        this._segments = row({ style_class: 'headroom-segmented', x_expand: true });
        this._bodySlot = column({ x_expand: true });
        this._breakdown = new SpendBreakdown(ctx, breakdown => ctx.selectSpend({ breakdown }));
        for (const actor of [this._segments, this._bodySlot, this._breakdown.actor]) this._card.add_child(actor);
        this.actor.add_child(this._card);
        this._show(false);
    }

    update(spend) {
        this._spend = spend;
        this._show(true);
    }

    grow() {
        this._body?.update(this._periodData(), { sweep: this._ctx.motion.enabled, morph: false });
    }

    _header(trailing) {
        const actor = row({ style_class: 'headroom-section-header spend' });
        if (this._ctx.canChooseUnit()) {
            const pull = titlePull(() => this._openUnits());
            this._pull = pull.actor;
            this._title = pull.title;
            actor.add_style_class_name('with-pull');
            actor.add_child(pull.actor);
        } else {
            this._title = label('', 'headroom-title');
            actor.add_child(this._title);
        }
        const info = themeIcon('help-about-symbolic', 'headroom-info-icon');
        this._ctx.tooltips.attach(info, () => infoText(this._settings().unit, this._periodData()));
        actor.add_child(info);
        actor.add_child(spacer());
        actor.add_child(trailing);
        return actor;
    }

    _settings() {
        const settings = this._ctx.spendSettings();
        return { ...settings, period: shownPeriod(this._spend, settings.period) };
    }

    _periodData() {
        return periodData(this._spend, this._settings().period);
    }

    _openUnits() {
        const [x, y] = this._pull.get_transformed_position();
        const [, height] = this._pull.get_transformed_size();
        const current = this._settings().unit;
        const items = UNITS.map(unit => ({
            id: unit,
            label: unitTitle(unit),
            hint: unitHint(unit),
            checked: unit === current,
        }));
        this._ctx.menu.open(items, [x, y + height + MENU_GAP], item => this._ctx.selectSpend({ unit: item.id }));
    }

    _syncSegments(period) {
        const choices = periodChoices(this._spend);
        const key = JSON.stringify(choices);
        if (key !== this._segmentsKey) {
            this._segmentsKey = key;
            this._segments.destroy_all_children();
            this._segments.style_class = `headroom-segmented${choices.length > 3 ? ' tight' : ''}`;
            for (const choice of choices) {
                const segment = textButton(periodTitle(choice), 'headroom-segment', () =>
                    this._ctx.selectSpend({ period: choice })
                );
                segment.x_expand = true;
                segment.name = choice;
                this._segments.add_child(segment);
            }
        }
        for (const segment of this._segments.get_children()) {
            if (segment.name === period) segment.add_style_pseudo_class('checked');
            else segment.remove_style_pseudo_class('checked');
        }
    }

    _show(animate) {
        const { period, unit, breakdown } = this._settings();
        this._title.text = unitTitle(unit);
        this._syncSegments(period);
        const data = periodData(this._spend, period);
        const key = spendBodyKey(data, unit);
        const morph = animate && this._ctx.motion.enabled && key === this._bodyKey;
        if (key !== this._bodyKey) {
            this._body?.actor.destroy();
            this._body = createBody(this._ctx, data, unit, spend => legendTitle(spend, this._settings().period));
            this._bodyKey = key;
            this._bodySlot.add_child(this._body.actor);
        }
        this._body.update(data, { sweep: false, morph });
        this._breakdown.update(data, breakdown, unit);
    }
}
