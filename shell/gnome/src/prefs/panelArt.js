import Gtk from 'gi://Gtk';
import { clockTime } from '../dates.js';
import { panelPercent, readingPercent } from '../format.js';
import { _ } from '../i18n.js';

const RING_SIZE = 18;
const RING_WIDTH = 3;
const TONE_CLASSES = { good: 'accent', warning: 'warning', critical: 'error', neutral: 'dim-label' };

export const PANEL_ART_CSS = `
.headroom-panel-art { border-radius: 12px; background: @accent_bg_color; }
.headroom-panel-art .bar { padding: 6px 12px; background: @window_bg_color; border-radius: 12px 12px 0 0; }
.headroom-panel-art .indicator { padding: 2px 8px; border-radius: 99px;
    box-shadow: inset 0 0 0 2px @accent_color; }
.headroom-panel-art .callout { margin: 14px 24px 36px 24px; padding: 10px 14px; border-radius: 12px;
    background: @window_bg_color; }
.headroom-app-tile { padding: 22px; border-radius: 24px; background: alpha(currentColor, 0.06);
    box-shadow: 0 1px 3px alpha(black, 0.18); }
`;

function drawRing(area, cr, width, height, fraction) {
    const color = area.get_color();
    const radius = (Math.min(width, height) - RING_WIDTH) / 2;
    cr.setLineWidth(RING_WIDTH);
    cr.setSourceRGBA(color.red, color.green, color.blue, 0.25);
    cr.arc(width / 2, height / 2, radius, 0, 2 * Math.PI);
    cr.stroke();
    cr.setSourceRGBA(color.red, color.green, color.blue, color.alpha);
    cr.arc(width / 2, height / 2, radius, -Math.PI / 2, -Math.PI / 2 + 2 * Math.PI * fraction);
    cr.stroke();
}

export class PanelArt {
    constructor() {
        this._fraction = 0;
        this._ring = new Gtk.DrawingArea({
            content_width: RING_SIZE,
            content_height: RING_SIZE,
            valign: Gtk.Align.CENTER,
        });
        this._ring.set_draw_func((area, cr, width, height) => drawRing(area, cr, width, height, this._fraction));
        this._value = new Gtk.Label({ css_classes: ['heading', 'numeric'] });
        this._clock = new Gtk.Label({ css_classes: ['heading', 'numeric'], hexpand: true, xalign: 1 });
        const indicator = new Gtk.Box({ spacing: 6, css_classes: ['indicator'], margin_start: 12 });
        indicator.append(this._ring);
        indicator.append(this._value);
        const bar = new Gtk.Box({ css_classes: ['bar'] });
        bar.append(this._clock);
        bar.append(indicator);
        bar.append(new Gtk.Box({ hexpand: true }));
        const callout = new Gtk.Label({
            label: `<b>Headroom</b>\n${_('Click to see every limit, spend and reset time.')}`,
            use_markup: true,
            wrap: true,
            xalign: 0,
            halign: Gtk.Align.END,
            css_classes: ['callout'],
        });
        this.widget = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, css_classes: ['headroom-panel-art'] });
        this.widget.append(bar);
        this.widget.append(callout);
    }

    update(state, settings, hour12) {
        const headline = state.headline;
        const percent = headline ? readingPercent(headline, settings.display.valueMode) : null;
        this._value.label = percent === null ? '—' : panelPercent(percent);
        this._fraction = percent === null ? 0 : Math.min(100, Math.max(0, percent)) / 100;
        this._ring.css_classes = [TONE_CLASSES[headline?.tone ?? 'neutral']];
        this._ring.queue_draw();
        this._clock.label = clockTime(new Date(), hour12);
    }
}
