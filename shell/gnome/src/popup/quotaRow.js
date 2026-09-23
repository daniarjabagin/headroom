import { leftAtResetText, limitText, percentLeft, resetText, spareText } from '../format.js';
import { column, fileIcon, label, row } from '../widgets.js';
import { Meter } from './meter.js';

const TONE_CLASSES = { good: 'ok', warning: 'warn', critical: 'crit', neutral: 'none' };

function toneClass(tone) {
    return TONE_CLASSES[tone] ?? 'none';
}

function hasData(window) {
    return window.remainingPercent !== null;
}

function tickPosition(window, alwaysShowPacing) {
    const even = window.pace.evenPacePercent;
    if (even === null || !hasData(window)) return null;
    if (!alwaysShowPacing && window.tone !== 'warning' && window.tone !== 'critical') return null;
    return 1 - even / 100;
}

function paceNote(window, now, alwaysShowPacing) {
    const { severity, sparePercent, runsOutAt } = window.pace;
    if (severity === 'spent') return { flame: true, text: 'Limit reached' };
    if (severity === 'running_out') return { flame: true, text: limitText(runsOutAt, now) };
    if (severity === 'close' && sparePercent !== null) return { flame: false, text: spareText(sparePercent) };
    if (alwaysShowPacing && severity === 'healthy' && sparePercent !== null)
        return { flame: false, text: leftAtResetText(sparePercent) };
    return null;
}

function trailingText(window, now) {
    if (!hasData(window)) return 'No data';
    return resetText(window.resetsAt, now);
}

export class QuotaRow {
    constructor(ctx, window) {
        this._ctx = ctx;
        this.actor = column({ style_class: 'headroom-quota-row', x_expand: true });
        this._label = label('', 'headroom-metric-label', { x_expand: true });
        this._flame = fileIcon(ctx.dir, 'flame-symbolic.svg', 'headroom-flame');
        this._note = label('', 'headroom-reading dim');
        this._meter = new Meter();
        this._headline = label('', 'headroom-reading', { x_expand: true });
        this._trailing = label('', 'headroom-reading dim');
        const top = row({ style_class: 'headroom-row-line' });
        const notes = row({ style_class: 'headroom-pace-note' });
        notes.add_child(this._flame);
        notes.add_child(this._note);
        top.add_child(this._label);
        top.add_child(notes);
        const bottom = row({ style_class: 'headroom-row-line' });
        bottom.add_child(this._headline);
        bottom.add_child(this._trailing);
        this.actor.add_child(top);
        this.actor.add_child(this._meter);
        this.actor.add_child(bottom);
        this.update(window, false);
    }

    update(window, animate = true) {
        this._window = window;
        const pacing = this._ctx.settings.alwaysShowPacing;
        this._label.text = window.label;
        this._headline.text = percentLeft(window.remainingPercent);
        this._meter.update(
            {
                fraction: hasData(window) ? window.remainingPercent / 100 : 0,
                tone: hasData(window) ? toneClass(window.tone) : 'none',
                tick: tickPosition(window, pacing),
            },
            animate
        );
        this.tick(this._ctx.now());
    }

    tick(now) {
        const note = paceNote(this._window, now, this._ctx.settings.alwaysShowPacing);
        this._flame.visible = note?.flame ?? false;
        this._note.visible = note !== null;
        this._note.text = note?.text ?? '';
        this._trailing.text = trailingText(this._window, now);
    }
}
