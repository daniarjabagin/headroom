import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { column, label } from '../widgets.js';

const HOLE_RATIO = 0.618;
const MIN_SLICE = 0.025;
const GAP_RATIO = 1.5 / 104;
const START_ANGLE = -Math.PI / 2;

function parseHex(hex) {
    const value = parseInt(hex.slice(1), 16);
    return [(value >> 16) & 255, (value >> 8) & 255, value & 255].map(channel => channel / 255);
}

function visibleFractions(values) {
    const total = values.reduce((sum, value) => sum + value, 0);
    if (total <= 0) return values.map(() => 0);
    const raised = values.map(value => Math.max(MIN_SLICE, value / total));
    const raisedTotal = raised.reduce((sum, value) => sum + value, 0);
    return raised.map(value => value / raisedTotal);
}

function drawSlices(cr, slices, size) {
    const outer = size / 2;
    const thickness = outer * (1 - HOLE_RATIO);
    const radius = outer - thickness / 2;
    const gap = slices.length > 1 ? (GAP_RATIO * size) / radius : 0;
    const fractions = visibleFractions(slices.map(slice => slice.value));
    cr.setLineWidth(thickness);
    let angle = START_ANGLE;
    slices.forEach((slice, index) => {
        const sweep = fractions[index] * 2 * Math.PI;
        const [red, green, blue] = parseHex(slice.color);
        cr.setSourceRGBA(red, green, blue, 1);
        cr.arc(outer, outer, radius, angle + gap / 2, angle + sweep - gap / 2);
        cr.stroke();
        angle += sweep;
    });
}

export class Donut {
    constructor() {
        this._slices = [];
        this.actor = new St.Widget({ style_class: 'headroom-donut', layout_manager: new Clutter.BinLayout() });
        this._area = new St.DrawingArea({ style_class: 'headroom-donut-area', x_expand: true, y_expand: true });
        this._area.connect('repaint', area => this._repaint(area));
        const center = column({
            style_class: 'headroom-donut-center',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._value = label('', 'headroom-donut-value', { x_align: Clutter.ActorAlign.CENTER });
        this._unit = label('dollars', 'headroom-donut-unit', { x_align: Clutter.ActorAlign.CENTER });
        center.add_child(this._value);
        center.add_child(this._unit);
        this.actor.add_child(this._area);
        this.actor.add_child(center);
    }

    update(slices, valueText) {
        this._slices = [...slices].sort((a, b) => b.value - a.value);
        this._value.text = valueText;
        this._area.queue_repaint();
    }

    _repaint(area) {
        const cr = area.get_context();
        const [width, height] = area.get_surface_size();
        drawSlices(cr, this._slices, Math.min(width, height));
        cr.$dispose();
    }
}
