import Cairo from 'cairo';
import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import St from 'gi://St';
import { donutGeometry, donutSegments, sectorPath, visibleFractions } from '../donutGeometry.js';
import { EASE, STANDARD_MS } from '../motion.js';
import { column, label } from '../widgets.js';

const SWEEP_MS = 250;

function parseHex(hex) {
    const value = parseInt(hex.slice(1), 16);
    return [(value >> 16) & 255, (value >> 8) & 255, value & 255].map(channel => channel / 255);
}

function blend(from, to, progress) {
    return to.map((value, index) => (from[index] ?? 0) + (value - (from[index] ?? 0)) * progress);
}

function tracePath(cr, arcs) {
    for (const arc of arcs) {
        if (arc.newSubPath) cr.newSubPath();
        const [x, y] = arc.center;
        if (arc.negative) cr.arcNegative(x, y, arc.radius, arc.from, arc.to);
        else cr.arc(x, y, arc.radius, arc.from, arc.to);
    }
    cr.closePath();
}

function drawSlices(cr, colors, fractions, reveal, size) {
    const geometry = donutGeometry(size);
    cr.setAntialias(Cairo.Antialias.BEST);
    for (const segment of donutSegments(fractions, reveal)) {
        const arcs = sectorPath(geometry, segment);
        if (!arcs) continue;
        const [red, green, blue] = parseHex(colors[segment.index]);
        cr.setSourceRGBA(red, green, blue, 1);
        tracePath(cr, arcs);
        cr.fill();
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
            this._reveal = { from: 1, to: 1 };
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
            this._reveal = { from: sweep ? 0 : 1, to: 1 };
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
            const reveal = this._reveal.from + (this._reveal.to - this._reveal.from) * this._progress;
            drawSlices(cr, this._colors, this._current(), reveal, Math.min(width, height));
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
