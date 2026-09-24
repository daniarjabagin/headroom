import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import { forecastText, isCountdownLive, percentReading, readingPercent, windowLabel } from '../format.js';
import { fillFraction, hasData, meterTone, paceNote, tickPosition, trailingText } from '../quota.js';
import { button, column, fileIcon, label, row, spacer, wrappingLabel } from '../widgets.js';
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
        this.actor = column({ style_class: 'headroom-quota-row', x_expand: true });
        this._label = label('', 'headroom-metric-label', { x_expand: true });
        this._flame = fileIcon(ctx.dir, 'flame-symbolic.svg', 'headroom-flame');
        this._note = label('', 'headroom-reading dim');
        this._meter = look.meter();
        this._headline = label('', 'headroom-reading');
        this._headline.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;
        this._reading = new NumberTween(ctx.motion, this._headline, percent =>
            look.reading(percent, this._window, this._ctx.display.valueMode)
        );
        this._trailing = label('', 'headroom-reading dim');
        this._forecast = wrappingLabel('', 'headroom-forecast', { x_align: Clutter.ActorAlign.START });
        this.actor.add_child(this._topLine());
        this.actor.add_child(this._meter);
        this.actor.add_child(this._bottomLine());
        this.actor.add_child(this._forecast);
        this.update(window, false);
    }

    get window() {
        return this._window;
    }

    _topLine() {
        const top = row({ style_class: 'headroom-row-line' });
        const notes = row({ style_class: 'headroom-pace-note' });
        notes.add_child(this._flame);
        notes.add_child(this._note);
        top.add_child(this._label);
        top.add_child(notes);
        return top;
    }

    _bottomLine() {
        const bottom = row({ style_class: 'headroom-row-line toggles' });
        bottom.add_child(toggle(this._headline, 'reading', () => this._ctx.actions.toggleValueMode()));
        bottom.add_child(spacer());
        bottom.add_child(toggle(this._trailing, 'trailing', () => this._ctx.actions.toggleResetFormat()));
        return bottom;
    }

    update(window, animate = true) {
        this._window = window;
        const display = this._ctx.display;
        const smooth = animate && this._ctx.motion.enabled;
        this._label.text = windowLabel(window.id, window.label);
        this._updateHeadline(window, display.valueMode, smooth);
        this._meter.update(this._look.meterState(window, display), smooth);
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
        const note = paceNote(this._window, now, display.showForecast);
        this._flame.visible = note?.flame ?? false;
        this._note.visible = Boolean(note?.text);
        this._note.text = note?.text ?? '';
        this._trailing.text = trailingText(this._window, now, display.resetFormat);
        const forecast = display.showForecast ? forecastText(this._window, now, display) : null;
        this._forecast.visible = forecast !== null;
        this._forecast.text = forecast ?? '';
    }
}
