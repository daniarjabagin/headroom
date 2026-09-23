import Cairo from 'cairo';
import GObject from 'gi://GObject';
import St from 'gi://St';
import { EASE, pulse, STANDARD_MS, stopPulse } from './motion.js';

const TRACK_ALPHA = 0.28;
const LINE_RATIO = 0.2;
const PULSE_OPACITY = 140;
const PULSE_PERIOD_MS = 2000;
const TONE_PROPERTIES = {
    warning: '-headroom-warning-color',
    critical: '-headroom-critical-color',
};

function toneColor(node, tone) {
    const property = TONE_PROPERTIES[tone];
    if (property) {
        const [found, color] = node.lookup_color(property, false);
        if (found) return color;
    }
    return node.get_foreground_color();
}

function setSource(cr, color, alpha = 1) {
    cr.setSourceRGBA(color.red / 255, color.green / 255, color.blue / 255, (color.alpha / 255) * alpha);
}

export const PanelRing = GObject.registerClass(
    {
        Properties: {
            'shown-fraction': GObject.ParamSpec.double(
                'shown-fraction',
                'Shown fraction',
                'Currently drawn ring fraction',
                GObject.ParamFlags.READWRITE,
                0,
                1,
                0
            ),
        },
    },
    class HeadroomPanelRing extends St.DrawingArea {
        _init(motion) {
            super._init({ style_class: 'headroom-panel-ring' });
            this._motion = motion;
            this._shownFraction = 0;
            this._tone = 'neutral';
            this._pulsing = false;
            this.connect('notify::mapped', () => this._syncPulse());
        }

        get shown_fraction() {
            return this._shownFraction;
        }

        set shown_fraction(value) {
            this._shownFraction = value;
            this.notify('shown-fraction');
            this.queue_repaint();
        }

        update(fraction, tone) {
            const target = Math.min(1, Math.max(0, fraction));
            const toneChanged = tone !== this._tone;
            this._tone = tone;
            if (toneChanged) this.queue_repaint();
            this.remove_transition('shown-fraction');
            if (this._motion.enabled && this.mapped && target !== this._shownFraction)
                this.ease_property('shown-fraction', target, { duration: STANDARD_MS, mode: EASE });
            else this.shown_fraction = target;
            this._syncPulse();
        }

        _syncPulse() {
            const shouldPulse = this._tone === 'critical' && this.mapped && this._motion.enabled;
            if (shouldPulse === this._pulsing) return;
            this._pulsing = shouldPulse;
            if (shouldPulse) pulse(this._motion, this, PULSE_OPACITY, PULSE_PERIOD_MS);
            else stopPulse(this);
        }

        vfunc_repaint() {
            const cr = this.get_context();
            const node = this.get_theme_node();
            const [width, height] = this.get_surface_size();
            const size = Math.min(width, height);
            const lineWidth = Math.max(2, size * LINE_RATIO);
            const radius = (size - lineWidth) / 2;
            const center = size / 2;
            cr.setLineWidth(lineWidth);
            setSource(cr, node.get_foreground_color(), TRACK_ALPHA);
            cr.arc(center, center, radius, 0, 2 * Math.PI);
            cr.stroke();
            if (this._shownFraction > 0) {
                cr.setLineCap(Cairo.LineCap.ROUND);
                setSource(cr, toneColor(node, this._tone));
                cr.arc(center, center, radius, -Math.PI / 2, -Math.PI / 2 + this._shownFraction * 2 * Math.PI);
                cr.stroke();
            }
            cr.$dispose();
        }
    }
);
