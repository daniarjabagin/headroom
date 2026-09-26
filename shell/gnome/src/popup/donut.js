import Cairo from 'cairo';
import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import St from 'gi://St';
import { donutGeometry, donutSegments, sectorPath, visibleFractions } from '../donutGeometry.js';
import { EASE, STANDARD_MS } from '../motion.js';
import { sameValues } from '../sameValues.js';
import { column, label } from '../widgets.js';
import { CenterFit } from './donutCenter.js';

const SWEEP_MS = 250;

function seriesColor(node, series) {
    const [found, color] = node.lookup_color(`-headroom-series-${series}`, false);
    return found ? color : node.get_foreground_color();
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

function drawSlices(cr, node, series, fractions, reveal, size) {
    const geometry = donutGeometry(size);
    cr.setAntialias(Cairo.Antialias.BEST);
    for (const segment of donutSegments(fractions, reveal)) {
        const arcs = sectorPath(geometry, segment);
        if (!arcs) continue;
        const color = seriesColor(node, series[segment.index]);
        cr.setSourceRGBA(color.red / 255, color.green / 255, color.blue / 255, color.alpha / 255);
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
            this._series = [];
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

        setSlices(series, fractions, { sweep, morph }) {
            if (!sweep && sameValues(series, this._series) && sameValues(fractions, this._to)) return;
            this.remove_transition('progress');
            this._from = morph ? this._current() : fractions;
            this._to = fractions;
            this._series = series;
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
            const size = Math.min(width, height);
            drawSlices(cr, this.get_theme_node(), this._series, this._current(), reveal, size);
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
        this._caption = label('', 'headroom-donut-caption', { x_align: Clutter.ActorAlign.CENTER, visible: false });
        center.add_child(this.value);
        center.add_child(this._caption);
        this.actor.add_child(this._area);
        this.actor.add_child(center);
        this._fit = new CenterFit(this.actor, this.value, this._caption);
    }

    setCaption(text) {
        this._caption.text = text ?? '';
        this._caption.visible = Boolean(text);
        this._fit.fit();
    }

    update(slices, { sweep = false, morph = false } = {}) {
        const fractions = visibleFractions(slices.map(slice => slice.value));
        this._area.setSlices(
            slices.map(slice => slice.series),
            fractions,
            { sweep, morph }
        );
    }
}
