import Cairo from 'cairo';
import GObject from 'gi://GObject';
import St from 'gi://St';
import { EASE, STANDARD_MS } from './motion.js';

const TRACK_ALPHA = 0.28;
const LINE_RATIO = 0.2;
const TONE_PROPERTIES = {
    warning: '-headroom-warning-color',
    critical: '-headroom-critical-color',
};

export function toneColor(node, tone) {
    const property = TONE_PROPERTIES[tone];
    if (property) {
        const [found, color] = node.lookup_color(property, false);
        if (found) return color;
    }
    return node.get_foreground_color();
}

export function setSource(cr, color, alpha = 1) {
    cr.setSourceRGBA(color.red / 255, color.green / 255, color.blue / 255, (color.alpha / 255) * alpha);
}

export function lookupDouble(node, property, fallback) {
    const [found, value] = node.lookup_double(property, false);
    return found ? value : fallback;
}

export function lookupLength(node, property, fallback) {
    const [found, value] = node.lookup_length(property, false);
    return found ? value : fallback;
}

export function trackAlpha(node) {
    return lookupDouble(node, '-headroom-track-alpha', TRACK_ALPHA);
}

export const FractionLeaf = GObject.registerClass(
    {
        Properties: {
            'shown-fraction': GObject.ParamSpec.double(
                'shown-fraction',
                'Shown fraction',
                'Currently drawn fraction',
                GObject.ParamFlags.READWRITE,
                0,
                1,
                0
            ),
        },
    },
    class HeadroomFractionLeaf extends St.DrawingArea {
        _init(motion, styleClass) {
            super._init({ style_class: styleClass });
            this._motion = motion;
            this._shownFraction = 0;
            this._tone = 'neutral';
        }

        get shown_fraction() {
            return this._shownFraction;
        }

        set shown_fraction(value) {
            this._shownFraction = value;
            this.notify('shown-fraction');
            this.queue_repaint();
        }

        update({ fraction, tone }) {
            if (tone !== this._tone) {
                this._tone = tone;
                this.queue_repaint();
            }
            this._showFraction(Math.min(1, Math.max(0, fraction)));
        }

        _showFraction(target) {
            const easing = this.get_transition('shown-fraction') !== null;
            if (target === this._shownFraction && !easing) return;
            this.remove_transition('shown-fraction');
            if (this._motion.enabled && this.mapped)
                this.ease_property('shown-fraction', target, { duration: STANDARD_MS, mode: EASE });
            else this.shown_fraction = target;
        }
    }
);

export const PanelRing = GObject.registerClass(
    class HeadroomPanelRing extends FractionLeaf {
        _init(motion) {
            super._init(motion, 'headroom-panel-ring');
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
            setSource(cr, node.get_foreground_color(), trackAlpha(node));
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
