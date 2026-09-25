import Clutter from 'gi://Clutter';
import Graphene from 'gi://Graphene';
import St from 'gi://St';
import { animate, FAST_MS } from '../motion.js';
import { column, label, row, spacer } from '../widgets.js';
import { SEPARATOR } from './cardMenu.js';
import { popupIcon } from './icons.js';

const START_SCALE = 0.97;
const NAVIGATION = { [Clutter.KEY_Up]: St.DirectionType.UP, [Clutter.KEY_Down]: St.DirectionType.DOWN };

function itemTexts(item) {
    if (!item.hint) return label(item.label, 'headroom-menu-label');
    const texts = column({ style_class: 'headroom-menu-texts' });
    texts.add_child(label(item.label, 'headroom-menu-label', { x_align: Clutter.ActorAlign.START }));
    texts.add_child(label(item.hint, 'headroom-menu-hint', { x_align: Clutter.ActorAlign.START }));
    return texts;
}

function leading(dir, item) {
    if (item.checked !== undefined) {
        const check = popupIcon(dir, 'object-select', 'headroom-menu-check');
        check.opacity = item.checked ? 255 : 0;
        return check;
    }
    return item.icon ? popupIcon(dir, item.icon, 'headroom-menu-icon') : null;
}

function itemContent(dir, item) {
    const content = row({ style_class: 'headroom-menu-item-box', x_expand: true });
    const lead = leading(dir, item);
    if (lead) content.add_child(lead);
    content.add_child(itemTexts(item));
    content.add_child(spacer());
    if (item.accel) content.add_child(label(item.accel, 'headroom-menu-accel'));
    return content;
}

export class FloatingMenu {
    constructor(ctx, overlay) {
        this._ctx = ctx;
        this._overlay = overlay;
        this._box = null;
    }

    get isOpen() {
        return this._box !== null;
    }

    open(items, [stageX, stageY], onActivate) {
        this.close(false);
        this._box = column({ style_class: 'headroom-floating-menu', reactive: true, can_focus: true });
        this._box.pivot_point = new Graphene.Point({ x: 0, y: 0 });
        for (const item of items) this._box.add_child(this._itemActor(item, onActivate));
        this._box.connect('key-press-event', (_actor, event) => this._onKey(event));
        this._overlay.capture(() => this.close());
        this._overlay.add(this._box);
        const [x, y] = this._overlay.localPoint(stageX, stageY);
        this._overlay.place(this._box, x, y);
        this._reveal();
    }

    close(animated = true) {
        const box = this._box;
        if (!box) return;
        this._box = null;
        this._overlay.release();
        if (!animated) {
            box.destroy();
            return;
        }
        animate(this._ctx.motion, box, { opacity: 0 }, { duration: FAST_MS, onComplete: () => box.destroy() });
    }

    _reveal() {
        const box = this._box;
        box.grab_key_focus();
        if (!this._ctx.motion.enabled) return;
        box.opacity = 0;
        box.scale_x = START_SCALE;
        box.scale_y = START_SCALE;
        animate(this._ctx.motion, box, { opacity: 255, scale_x: 1, scale_y: 1 }, { duration: FAST_MS + 30 });
    }

    _itemActor(item, onActivate) {
        if (item === SEPARATOR) return new St.Widget({ style_class: 'headroom-menu-separator', x_expand: true });
        const actor = new St.Button({
            child: itemContent(this._ctx.dir, item),
            style_class: 'headroom-menu-item',
            reactive: true,
            can_focus: true,
            track_hover: true,
            x_expand: true,
        });
        actor.connect('clicked', () => {
            this.close();
            onActivate(item);
        });
        return actor;
    }

    _onKey(event) {
        const symbol = event.get_key_symbol();
        if (symbol === Clutter.KEY_Escape) {
            this.close();
            return Clutter.EVENT_STOP;
        }
        const direction = NAVIGATION[symbol];
        if (direction === undefined) return Clutter.EVENT_PROPAGATE;
        const from = global.stage.key_focus === this._box ? null : global.stage.key_focus;
        this._box.navigate_focus(from, direction, true);
        return Clutter.EVENT_STOP;
    }
}
