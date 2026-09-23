import Cairo from 'cairo';
import GObject from 'gi://GObject';
import St from 'gi://St';

const TRACK_ALPHA = 0.28;
const LINE_RATIO = 0.2;
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
    class HeadroomPanelRing extends St.DrawingArea {
        _init() {
            super._init({ style_class: 'headroom-panel-ring' });
            this._fraction = 0;
            this._tone = 'neutral';
        }

        update(fraction, tone) {
            this._fraction = Math.min(1, Math.max(0, fraction));
            this._tone = tone;
            this.queue_repaint();
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
            if (this._fraction > 0) {
                cr.setLineCap(Cairo.LineCap.ROUND);
                setSource(cr, toneColor(node, this._tone));
                cr.arc(center, center, radius, -Math.PI / 2, -Math.PI / 2 + this._fraction * 2 * Math.PI);
                cr.stroke();
            }
            cr.$dispose();
        }
    }
);
