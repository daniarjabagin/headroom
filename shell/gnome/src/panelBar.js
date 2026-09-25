import GObject from 'gi://GObject';
import { fillWidth, tickLeft } from './panelGeometry.js';
import { FractionLeaf, lookupDouble, lookupLength, setSource, toneColor, trackAlpha } from './panelRing.js';

const BAR_HEIGHT = 5;
const TICK_WIDTH = 2;
const TICK_ALPHA = 0.8;

function pill(cr, x, y, width, height, radius) {
    const r = Math.min(radius, width / 2, height / 2);
    cr.newSubPath();
    cr.arc(x + width - r, y + r, r, -Math.PI / 2, 0);
    cr.arc(x + width - r, y + height - r, r, 0, Math.PI / 2);
    cr.arc(x + r, y + height - r, r, Math.PI / 2, Math.PI);
    cr.arc(x + r, y + r, r, Math.PI, (3 * Math.PI) / 2);
    cr.closePath();
    cr.fill();
}

export const PanelBar = GObject.registerClass(
    class HeadroomPanelBar extends FractionLeaf {
        _init(motion) {
            super._init(motion, 'headroom-panel-bar');
            this._tick = null;
        }

        update({ fraction, tone, tick }) {
            if (tick !== this._tick) {
                this._tick = tick;
                this.queue_repaint();
            }
            super.update({ fraction, tone });
        }

        vfunc_repaint() {
            const cr = this.get_context();
            const node = this.get_theme_node();
            const [width, height] = this.get_surface_size();
            const barHeight = Math.min(height, lookupLength(node, '-headroom-bar-height', BAR_HEIGHT));
            const top = (height - barHeight) / 2;
            const foreground = node.get_foreground_color();
            setSource(cr, foreground, trackAlpha(node));
            pill(cr, 0, top, width, barHeight, barHeight / 2);
            const filled = fillWidth(this._shownFraction, width, barHeight);
            if (filled > 0) {
                setSource(cr, toneColor(node, this._tone));
                pill(cr, 0, top, filled, barHeight, barHeight / 2);
            }
            if (this._tick !== null) this._paintTick(cr, node, width, height);
            cr.$dispose();
        }

        _paintTick(cr, node, width, height) {
            const tickWidth = lookupLength(node, '-headroom-tick-width', TICK_WIDTH);
            setSource(cr, node.get_foreground_color(), lookupDouble(node, '-headroom-tick-alpha', TICK_ALPHA));
            pill(cr, tickLeft(this._tick, width, tickWidth), 0, tickWidth, height, tickWidth / 2);
        }
    }
);
