import GLib from 'gi://GLib';
import { animate } from '../motion.js';
import { label, row } from '../widgets.js';
import { popupIcon } from './icons.js';

const VISIBLE_MS = 2400;
const RISE_PX = 6;
const FOOTER_GAP = 14;

export class Toast {
    constructor(ctx, overlay, anchorOf) {
        this._ctx = ctx;
        this._overlay = overlay;
        this._anchorOf = anchorOf;
        this._actor = null;
        this._timeoutId = 0;
    }

    show(title, detail, icon = 'check-circle') {
        this.hide();
        const actor = row({ style_class: 'headroom-toast' });
        actor.add_child(popupIcon(this._ctx.dir, icon, `headroom-toast-icon ${icon}`));
        actor.add_child(label(title, 'headroom-toast-title'));
        if (detail) actor.add_child(label(detail, 'headroom-toast-detail'));
        this._overlay.add(actor);
        const { width, height } = this._overlay.place(actor, 0, 0);
        const anchor = this._overlay.localBox(this._anchorOf());
        const x = anchor.x + (anchor.width - width) / 2;
        actor.set_position(Math.round(x), Math.round(anchor.y - FOOTER_GAP - height));
        this._actor = actor;
        this._enter(actor);
        this._timeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, VISIBLE_MS, () => {
            this._timeoutId = 0;
            this._leave();
            return GLib.SOURCE_REMOVE;
        });
    }

    hide() {
        this._clearTimeout();
        this._actor?.destroy();
        this._actor = null;
    }

    _enter(actor) {
        if (!this._ctx.motion.enabled) return;
        actor.opacity = 0;
        actor.translation_y = RISE_PX;
        animate(this._ctx.motion, actor, { opacity: 255, translation_y: 0 });
    }

    _leave() {
        const actor = this._actor;
        this._actor = null;
        if (!actor) return;
        animate(this._ctx.motion, actor, { opacity: 0, translation_y: RISE_PX }, { onComplete: () => actor.destroy() });
    }

    _clearTimeout() {
        if (this._timeoutId === 0) return;
        GLib.source_remove(this._timeoutId);
        this._timeoutId = 0;
    }
}
