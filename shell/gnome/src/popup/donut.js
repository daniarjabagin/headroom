import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import St from 'gi://St';
import { _ } from '../i18n.js';
import { EASE, STANDARD_MS } from '../motion.js';
import { column, label } from '../widgets.js';

const HOLE_RATIO = 0.618;
const MIN_SLICE = 0.025;
const GAP_RATIO = 1.5 / 104;
const START_ANGLE = -Math.PI / 2;
const SWEEP_MS = 250;

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

function blend(from, to, progress) {
    return to.map((value, index) => (from[index] ?? 0) + (value - (from[index] ?? 0)) * progress);
}

function drawSlices(cr, slices, size) {
    const outer = size / 2;
    const thickness = outer * (1 - HOLE_RATIO);
    const radius = outer - thickness / 2;
    const gap = slices.length > 1 ? (GAP_RATIO * size) / radius : 0;
    cr.setLineWidth(thickness);
    let angle = START_ANGLE;
    for (const slice of slices) {
        const sweep = slice.fraction * 2 * Math.PI;
        if (sweep > gap) {
            const [red, green, blue] = parseHex(slice.color);
            cr.setSourceRGBA(red, green, blue, 1);
            cr.arc(outer, outer, radius, angle + gap / 2, angle + sweep - gap / 2);
            cr.stroke();
        }
        angle += sweep;
    }
}

const DonutArea = GObject.registerClass(
    {
        Properties: {
            progress: GObject.ParamSpec.double(
                'progress',
                'Progress',
                'Animation progress',
                GObject.ParamFlags.READWRITE,
                0,
                1,
                1
            ),
        },
    },
    class HeadroomDonutArea extends St.DrawingArea {
        _init() {
            super._init({ style_class: 'headroom-donut-area', x_expand: true, y_expand: true });
            this._progress = 1;
            this._colors = [];
            this._from = [];
            this._to = [];
            this._scale = { from: 1, to: 1 };
        }

        get progress() {
            return this._progress;
        }

        set progress(value) {
            this._progress = value;
            this.notify('progress');
            this.queue_repaint();
        }

        setSlices(colors, fractions, { sweep, morph }) {
            this.remove_transition('progress');
            this._from = morph ? this._current() : fractions;
            this._to = fractions;
            this._colors = colors;
            this._scale = { from: sweep ? 0 : 1, to: 1 };
            const animated = (sweep || morph) && this.mapped;
            this.progress = animated ? 0 : 1;
            if (animated) this.ease_property('progress', 1, { duration: sweep ? SWEEP_MS : STANDARD_MS, mode: EASE });
        }

        _current() {
            return blend(this._from, this._to, this._progress);
        }

        vfunc_repaint() {
            const cr = this.get_context();
            const [width, height] = this.get_surface_size();
            const scale = this._scale.from + (this._scale.to - this._scale.from) * this._progress;
            const fractions = this._current().map(fraction => fraction * scale);
            const slices = fractions.map((fraction, index) => ({ fraction, color: this._colors[index] }));
            drawSlices(cr, slices, Math.min(width, height));
            cr.$dispose();
        }
    }
);

export class Donut {
    constructor() {
        this.actor = new St.Widget({ style_class: 'headroom-donut', layout_manager: new Clutter.BinLayout() });
        this._area = new DonutArea();
        const center = column({
            style_class: 'headroom-donut-center',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
        });
        this.value = label('', 'headroom-donut-value', { x_align: Clutter.ActorAlign.CENTER });
        center.add_child(this.value);
        center.add_child(label(_('dollars'), 'headroom-donut-unit', { x_align: Clutter.ActorAlign.CENTER }));
        this.actor.add_child(this._area);
        this.actor.add_child(center);
    }

    update(slices, { sweep = false, morph = false } = {}) {
        const fractions = visibleFractions(slices.map(slice => slice.value));
        this._area.setSlices(
            slices.map(slice => slice.color),
            fractions,
            { sweep, morph }
        );
    }
}
