import Clutter from 'gi://Clutter';
import St from 'gi://St';

const EDGE = 6;

function fillParent(actor, parent) {
    actor.add_constraint(new Clutter.BindConstraint({ source: parent, coordinate: Clutter.BindCoordinate.SIZE }));
}

export class Overlay {
    constructor() {
        this.actor = new St.Widget({
            style_class: 'headroom-overlay',
            layout_manager: new Clutter.FixedLayout(),
            x_expand: true,
            y_expand: true,
        });
        this._catcher = new St.Widget({ reactive: true, visible: false });
        fillParent(this._catcher, this.actor);
        this._catcher.connect('button-press-event', () => {
            this._onDismiss?.();
            return Clutter.EVENT_STOP;
        });
        this.actor.add_child(this._catcher);
        this._onDismiss = null;
    }

    capture(onDismiss) {
        this._onDismiss = onDismiss;
        this._catcher.show();
        this.actor.set_child_above_sibling(this._catcher, null);
    }

    release() {
        this._onDismiss = null;
        this._catcher.hide();
    }

    add(actor) {
        this.actor.add_child(actor);
        this.actor.set_child_above_sibling(actor, null);
    }

    localPoint(stageX, stageY) {
        const [ok, x, y] = this.actor.transform_stage_point(stageX, stageY);
        return ok ? [x, y] : [EDGE, EDGE];
    }

    localBox(actor) {
        const [x, y] = actor.get_transformed_position();
        const [width, height] = actor.get_transformed_size();
        const [left, top] = this.localPoint(x, y);
        const [right, bottom] = this.localPoint(x + width, y + height);
        return { x: left, y: top, width: right - left, height: bottom - top };
    }

    place(actor, x, y) {
        const [, width] = actor.get_preferred_width(-1);
        const [, height] = actor.get_preferred_height(width);
        const maxX = Math.max(EDGE, this.actor.width - width - EDGE);
        const maxY = Math.max(EDGE, this.actor.height - height - EDGE);
        actor.set_position(
            Math.round(Math.min(Math.max(EDGE, x), maxX)),
            Math.round(Math.min(Math.max(EDGE, y), maxY))
        );
        return { width, height };
    }
}
