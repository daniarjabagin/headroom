import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { dragStarted, dropTarget, withinReach } from './panelGeometry.js';

const HOLD_MS = 400;
const PRIMARY_BUTTON = 1;
const GHOST_OPACITY = 210;
const SOURCE_OPACITY = 90;
const MARKER_INSET = 4;

function extent(actor) {
    const [x] = actor.get_transformed_position();
    const [width] = actor.get_transformed_size();
    return { x1: x, x2: x + width };
}

function eventPoint(event) {
    const [x, y] = event.get_coords();
    return { x, y };
}

export class PanelDrag {
    constructor({ button, placement, onDrop, onClick }) {
        this._button = button;
        this._placement = placement;
        this._onDrop = onDrop;
        this._onClick = onClick;
        this.enabled = false;
        this._press = null;
        this._drag = null;
        this._holdId = 0;
    }

    get dragging() {
        return this._drag !== null;
    }

    handleEvent(event) {
        switch (event.type()) {
            case Clutter.EventType.BUTTON_PRESS:
                return this._onPress(event);
            case Clutter.EventType.MOTION:
                return this._onMotion(event);
            case Clutter.EventType.BUTTON_RELEASE:
                return this._onRelease(event);
            case Clutter.EventType.KEY_PRESS:
                return this._onKey(event);
            case Clutter.EventType.TOUCH_BEGIN:
                this._onClick();
                return true;
            default:
                return false;
        }
    }

    cancel() {
        this._clearHold();
        this._press = null;
        this._endDrag();
    }

    destroy() {
        this.cancel();
    }

    _onPress(event) {
        if (event.get_button() !== PRIMARY_BUTTON) {
            this._onClick();
            return true;
        }
        this.cancel();
        this._press = { start: eventPoint(event), last: eventPoint(event) };
        this._holdId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, HOLD_MS, () => {
            this._holdId = 0;
            this._beginDrag();
            return GLib.SOURCE_REMOVE;
        });
        return true;
    }

    _onMotion(event) {
        if (this._drag) {
            this._follow(eventPoint(event));
            return true;
        }
        if (!this._press) return false;
        this._press.last = eventPoint(event);
        if (dragStarted(this._press.start, this._press.last, St.Settings.get().drag_threshold)) this._beginDrag();
        return true;
    }

    _onRelease(event) {
        if (event.get_button() !== PRIMARY_BUTTON) return this._press !== null || this._drag !== null;
        if (this._drag) {
            this._follow(eventPoint(event));
            this._drop();
            return true;
        }
        if (!this._press) return false;
        this.cancel();
        this._onClick();
        return true;
    }

    _onKey(event) {
        if (!this._drag || event.get_key_symbol() !== Clutter.KEY_Escape) return false;
        this.cancel();
        return true;
    }

    _clearHold() {
        if (this._holdId === 0) return;
        GLib.source_remove(this._holdId);
        this._holdId = 0;
    }

    _beginDrag() {
        this._clearHold();
        const press = this._press;
        if (!press || this._drag || !this.enabled || !this._placement.movable) return;
        const container = this._button.container;
        const [x, y] = container.get_transformed_position();
        this._drag = {
            grab: global.stage.grab(this._button),
            focus: global.stage.get_key_focus(),
            offsetX: press.start.x - x,
            top: y,
            ghost: new Clutter.Clone({ source: container, opacity: GHOST_OPACITY, reactive: false }),
            marker: new St.Widget({ style_class: 'headroom-drop-marker', reactive: false, visible: false }),
            target: null,
        };
        Main.uiGroup.add_child(this._drag.ghost);
        Main.uiGroup.add_child(this._drag.marker);
        container.opacity = SOURCE_OPACITY;
        global.stage.set_key_focus(this._button);
        this._follow(press.last);
    }

    _follow(point) {
        const drag = this._drag;
        drag.ghost.set_position(Math.round(point.x - drag.offsetX), Math.round(drag.top));
        drag.target = this._targetAt(point);
        drag.marker.visible = drag.target !== null;
        if (drag.target) this._placeMarker(drag.marker, drag.target.markerX);
    }

    _placeMarker(marker, x) {
        const [, top] = Main.panel.get_transformed_position();
        const [, height] = Main.panel.get_transformed_size();
        marker.set_height(Math.max(1, height - 2 * MARKER_INSET));
        marker.set_position(Math.round(x - marker.width / 2), Math.round(top + MARKER_INSET));
    }

    _targetAt(point) {
        const [, top] = Main.panel.get_transformed_position();
        const [, height] = Main.panel.get_transformed_size();
        if (!withinReach(point.y, top, top + height)) return null;
        const boxes = this._placement.boxes().map(({ name, actor }) => ({
            name,
            ...extent(actor),
            children: this._placement
                .siblings(actor)
                .map(child => (child.visible && child.mapped ? extent(child) : null)),
        }));
        return dropTarget(boxes, point.x);
    }

    _drop() {
        const target = this._drag.target;
        this.cancel();
        if (!target) return;
        const position = { box: target.box, index: target.index };
        if (this._placement.isCurrent(position) || !this._placement.move(position)) return;
        this._placement.remember(position);
        this._onDrop(position);
    }

    _endDrag() {
        const drag = this._drag;
        if (!drag) return;
        this._drag = null;
        drag.grab.dismiss();
        drag.ghost.destroy();
        drag.marker.destroy();
        this._button.container.opacity = 255;
        if (global.stage.get_key_focus() === this._button) global.stage.set_key_focus(drag.focus);
    }
}
