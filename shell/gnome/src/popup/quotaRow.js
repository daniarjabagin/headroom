import Clutter from 'gi://Clutter';
import {
    forecastText,
    isCountdownLive,
    limitText,
    percentReading,
    readingPercent,
    resetText,
    spareText,
    windowLabel,
} from '../format.js';
import { _ } from '../i18n.js';
import { button, column, fileIcon, label, row, spacer, wrappingLabel } from '../widgets.js';
import { Meter } from './meter.js';
import { NumberTween } from './tween.js';

const TONE_CLASSES = { good: 'ok', warning: 'warn', critical: 'crit', neutral: 'none' };

function toneClass(tone) {
    return TONE_CLASSES[tone] ?? 'none';
}

function hasData(window) {
    return window.remainingPercent !== null;
}

function fillFraction(window, valueMode) {
    if (!hasData(window)) return 0;
    return readingPercent(window, valueMode) / 100;
}

function tickPosition(window, display) {
    const even = window.pace.evenPacePercent;
    if (even === null || !hasData(window)) return null;
    if (!display.showForecast && window.tone !== 'warning' && window.tone !== 'critical') return null;
    return display.valueMode === 'used' ? even / 100 : 1 - even / 100;
}

function paceNote(window, now, showForecast) {
    const { severity, sparePercent, runsOutAt } = window.pace;
    if (severity === 'spent') return { flame: true, text: _('Limit reached') };
    if (severity === 'running_out')
        return { flame: true, text: showForecast ? _('Over pace') : limitText(runsOutAt, now) };
    if (severity === 'close' && sparePercent !== null && !showForecast)
        return { flame: false, text: spareText(sparePercent) };
    return null;
}

function trailingText(window, now, resetFormat) {
    if (!hasData(window)) return _('No data');
    return resetText(window.resetsAt, now, resetFormat, true);
}

function toggle(text, styleClass, onClick) {
    const actor = button(text, `headroom-toggle ${styleClass}`, onClick);
    actor.y_align = Clutter.ActorAlign.CENTER;
    return actor;
}

export class QuotaRow {
    constructor(ctx, window) {
        this._ctx = ctx;
        this._valueMode = null;
        this.actor = column({ style_class: 'headroom-quota-row', x_expand: true });
        this._label = label('', 'headroom-metric-label', { x_expand: true });
        this._flame = fileIcon(ctx.dir, 'flame-symbolic.svg', 'headroom-flame');
        this._note = label('', 'headroom-reading dim');
        this._meter = new Meter();
        this._headline = label('', 'headroom-reading');
        this._reading = new NumberTween(ctx.motion, this._headline, percent =>
            percentReading(percent, this._ctx.display.valueMode)
        );
        this._trailing = label('', 'headroom-reading dim');
        this._forecast = wrappingLabel('', 'headroom-forecast', { x_align: Clutter.ActorAlign.START });
        this.actor.add_child(this._topLine());
        this.actor.add_child(this._meter);
        this.actor.add_child(this._bottomLine());
        this.actor.add_child(this._forecast);
        this.update(window, false);
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
        const bottom = row({ style_class: 'headroom-row-line' });
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
        this._meter.update(
            {
                fraction: fillFraction(window, display.valueMode),
                tone: hasData(window) ? toneClass(window.tone) : 'none',
                tick: tickPosition(window, display),
            },
            smooth
        );
        this.tick(this._ctx.now());
    }

    _updateHeadline(window, valueMode, smooth) {
        const modeChanged = this._valueMode !== valueMode;
        this._valueMode = valueMode;
        const percent = hasData(window) ? readingPercent(window, valueMode) : null;
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
