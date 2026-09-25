import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import { forecastText, isCountdownLive, percentReading, readingPercent, windowLabel } from '../format.js';
import { fillFraction, hasData, meterTone, paceNote, tickPosition } from '../quota.js';
import { button, column, fileIcon, label, row, spacer, wrappingLabel } from '../widgets.js';
import { compactTrailing, paceTip, resetHint, trailingLabel, valueHint } from './limitTexts.js';
import { MASK } from './mask.js';
import { Meter } from './meter.js';
import { NumberTween } from './tween.js';

const SINGLE_LOOK = {
    meter: () => new Meter(),
    meterState: (window, display) => ({
        fraction: fillFraction(window, display.valueMode),
        tone: meterTone(window),
        tick: tickPosition(window, display),
    }),
    percent: (window, valueMode) => readingPercent(window, valueMode),
    reading: (percent, _window, valueMode) => percentReading(percent, valueMode),
};

const BLANK_METER = { fraction: 0, tone: 'none', tick: null };

function maskedMeter(state) {
    return { ...BLANK_METER, segments: (state.segments ?? []).map(() => BLANK_METER) };
}

function toggle(text, styleClass, onClick) {
    const actor = button(text, `headroom-toggle ${styleClass}`, onClick);
    actor.y_align = Clutter.ActorAlign.CENTER;
    return actor;
}

export class QuotaRow {
    constructor(ctx, window, look = SINGLE_LOOK) {
        this._ctx = ctx;
        this._look = look;
        this._valueMode = null;
        this._compact = ctx.display.density === 'compact';
        this.actor = column({ style_class: 'headroom-quota-row', x_expand: true });
        this._label = label('', 'headroom-metric-label', { x_expand: true });
        this._flame = fileIcon(ctx.dir, 'flame-symbolic.svg', 'headroom-flame');
        this._note = label('', 'headroom-reading dim');
        this._meter = look.meter();
        this._headline = label('', 'headroom-reading');
        this._headline.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;
        this._reading = new NumberTween(ctx.motion, this._headline, percent => this._readingText(percent));
        this._trailing = label('', 'headroom-reading dim');
        this._forecast = wrappingLabel('', 'headroom-forecast', { x_align: Clutter.ActorAlign.START });
        this._readingToggle = toggle(this._headline, 'reading', () => ctx.actions.toggleValueMode());
        this._trailingToggle = toggle(this._trailing, 'trailing', () => ctx.actions.toggleResetFormat());
        ctx.tooltips.attach(this._readingToggle, () => valueHint(ctx.display.valueMode));
        ctx.tooltips.attach(this._trailingToggle, () => resetHint(ctx.display.resetFormat));
        if (this._compact) this._buildCompact();
        else this._buildNormal();
        this.update(window, false);
    }

    get window() {
        return this._window;
    }

    _readingText(percent) {
        if (this._ctx.masked) return MASK;
        return this._look.reading(percent, this._window, this._ctx.display.valueMode);
    }

    _buildNormal() {
        const top = row({ style_class: 'headroom-row-line' });
        const notes = row({ style_class: 'headroom-pace-note' });
        notes.add_child(this._flame);
        notes.add_child(this._note);
        top.add_child(this._label);
        top.add_child(notes);
        const bottom = row({ style_class: 'headroom-row-line toggles' });
        bottom.add_child(this._readingToggle);
        bottom.add_child(spacer());
        bottom.add_child(this._trailingToggle);
        for (const actor of [top, this._meter, bottom, this._forecast]) this.actor.add_child(actor);
    }

    _buildCompact() {
        const top = row({ style_class: 'headroom-row-line compact' });
        top.add_child(this._label);
        top.add_child(this._flame);
        top.add_child(this._readingToggle);
        top.add_child(this._trailingToggle);
        this._ctx.tooltips.attach(this._meter, () =>
            this._ctx.masked ? null : paceTip(this._window, this._ctx.now(), this._ctx.display, this._ctx.hour12())
        );
        this.actor.add_child(top);
        this.actor.add_child(this._meter);
    }

    update(window, animate = true) {
        this._window = window;
        const display = this._ctx.display;
        const smooth = animate && this._ctx.motion.enabled;
        this._label.text = windowLabel(window.id, window.label);
        this._updateHeadline(window, display.valueMode, smooth);
        const meterState = this._look.meterState(window, display);
        this._meter.update(this._ctx.masked ? maskedMeter(meterState) : meterState, smooth);
        this.tick(this._ctx.now());
    }

    _updateHeadline(window, valueMode, smooth) {
        const modeChanged = this._valueMode !== valueMode;
        this._valueMode = valueMode;
        const percent = hasData(window) ? this._look.percent(window, valueMode) : null;
        this._reading.set(percent, smooth && !modeChanged);
    }

    grow(delay) {
        this._meter.grow(delay);
    }

    settle() {
        this._meter.settle();
    }

    needsSecondTicks(now) {
        return (
            hasData(this._window) &&
            this._ctx.display.resetFormat === 'countdown' &&
            isCountdownLive(this._window.resetsAt, now)
        );
    }

    tick(now) {
        const display = this._ctx.display;
        const hour12 = this._ctx.hour12();
        const note = this._ctx.masked ? null : paceNote(this._window, now, display.showForecast);
        this._flame.visible = note?.flame ?? false;
        this._note.visible = !this._compact && Boolean(note?.text);
        this._note.text = note?.text ?? '';
        const trailing = this._compact ? compactTrailing : trailingLabel;
        this._trailing.text = trailing(this._window, now, display.resetFormat, hour12);
        const forecast =
            display.showForecast && !this._compact ? forecastText(this._window, now, display, hour12) : null;
        this._forecast.visible = forecast !== null;
        this._forecast.text = this._ctx.masked ? MASK : (forecast ?? '');
    }
}
