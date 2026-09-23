import Clutter from 'gi://Clutter';
import St from 'gi://St';

const DRAG_THRESHOLD = 6;
const LIFTED_OPACITY = 90;
const SETTLE_MS = 150;

function stageY(event) {
    return event.get_coords()[1];
}

function bounds(actor) {
    const [, y] = actor.get_transformed_position();
    const [, height] = actor.get_transformed_size();
    return { top: y, bottom: y + height, middle: y + height / 2 };
}

export class Reorderer {
    constructor({ layer, onDrop, onSettled }) {
        this._layer = layer;
        this._onDrop = onDrop;
        this._onSettled = onSettled;
        this._sections = [];
        this._drag = null;
        this._indicator = new St.Widget({ style_class: 'headroom-drop-indicator', visible: false });
        this._layer.add_child(this._indicator);
    }

    setSections(sections) {
        this.cancel();
        this._sections = sections;
        for (const section of sections)
            section.header.connect('button-press-event', (_actor, event) => this._press(section, event));
    }

    get enabled() {
        return this._sections.length > 1;
    }

    get dragging() {
        return this._drag !== null;
    }

    cancel() {
        if (!this._drag) return;
        this._finish(false);
    }

    destroy() {
        this.cancel();
        this._sections = [];
    }

    _press(section, event) {
        if (!this.enabled || event.get_button() !== Clutter.BUTTON_PRIMARY || this._drag)
            return Clutter.EVENT_PROPAGATE;
        this._drag = { section, startY: stageY(event), lifted: false, clone: null, target: -1 };
        this._layer.reactive = true;
        this._drag.grab = global.stage.grab(this._layer);
        this._drag.eventId = this._layer.connect('captured-event', (_actor, captured) => this._onEvent(captured));
        return Clutter.EVENT_STOP;
    }

    _onEvent(event) {
        const type = event.type();
        if (type === Clutter.EventType.MOTION) this._move(stageY(event));
        else if (type === Clutter.EventType.BUTTON_RELEASE) this._finish(true);
        else if (type === Clutter.EventType.KEY_PRESS && event.get_key_symbol() === Clutter.KEY_Escape)
            this._finish(false);
        else return Clutter.EVENT_PROPAGATE;
        return Clutter.EVENT_STOP;
    }

    _move(y) {
        const drag = this._drag;
        const offset = y - drag.startY;
        if (!drag.lifted && Math.abs(offset) < DRAG_THRESHOLD) return;
        if (!drag.lifted) this._lift();
        drag.clone.translation_y = offset;
        drag.target = this._targetIndex(y);
        this._showIndicator(drag.target);
    }

    _lift() {
        const drag = this._drag;
        const source = drag.section.actor;
        const [x, y] = source.get_transformed_position();
        const [, layerX, layerY] = this._layer.transform_stage_point(x, y);
        const [width, height] = source.get_size();
        const clone = new Clutter.Clone({ source, width, height });
        drag.clone = new St.Bin({ style_class: 'headroom-drag-clone', child: clone });
        drag.clone.set_position(Math.round(layerX), Math.round(layerY));
        this._layer.add_child(drag.clone);
        this._layer.set_child_above_sibling(this._indicator, null);
        source.opacity = LIFTED_OPACITY;
        drag.lifted = true;
    }

    _others() {
        return this._sections.filter(section => section !== this._drag.section);
    }

    _targetIndex(y) {
        return this._others().filter(section => bounds(section.actor).middle < y).length;
    }

    _showIndicator(target) {
        const others = this._others();
        if (others.length === 0 || target === this._sections.indexOf(this._drag.section)) {
            this._indicator.hide();
            return;
        }
        const spacing = this._spacing(others);
        const y =
            target === 0 ? bounds(others[0].actor).top - spacing : bounds(others[target - 1].actor).bottom + spacing;
        const [x] = others[0].actor.get_transformed_position();
        const [width] = others[0].actor.get_transformed_size();
        const [, layerX, layerY] = this._layer.transform_stage_point(x, y);
        const [, height] = this._indicator.get_preferred_height(-1);
        this._indicator.set_position(Math.round(layerX), Math.round(layerY - height / 2));
        this._indicator.set_size(Math.round(width), height);
        this._indicator.show();
    }

    _spacing(sections) {
        const first = sections[0].actor;
        const next = first.get_next_sibling();
        return next ? (bounds(next).top - bounds(first).bottom) / 2 : 0;
    }

    _finish(commit) {
        const drag = this._drag;
        this._drag = null;
        this._layer.disconnect(drag.eventId);
        drag.grab.dismiss();
        this._layer.reactive = false;
        this._indicator.hide();
        drag.section.actor.opacity = 255;
        if (drag.clone) this._settle(drag.clone);
        const from = this._sections.indexOf(drag.section);
        if (commit && drag.lifted && drag.target >= 0 && drag.target !== from) this._onDrop(from, drag.target);
        this._onSettled();
    }

    _settle(clone) {
        clone.ease({
            opacity: 0,
            duration: SETTLE_MS,
            mode: Clutter.AnimationMode.EASE_OUT_QUAD,
            onStopped: () => clone.destroy(),
        });
    }
}
