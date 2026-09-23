import GLib from 'gi://GLib';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const SHOW_DELAY_MS = 400;
const GAP = 6;

export class Tooltips {
    constructor() {
        this._label = new St.Label({ style_class: 'headroom-tooltip', visible: false });
        this._label.clutter_text.line_wrap = true;
        Main.layoutManager.uiGroup.add_child(this._label);
        this._timeoutId = 0;
        this._target = null;
    }

    attach(actor, textFor) {
        actor.track_hover = true;
        actor.reactive = true;
        actor.connect('notify::hover', () => (actor.hover ? this._schedule(actor, textFor) : this._hide(actor)));
        actor.connect('destroy', () => this._hide(actor));
    }

    hide() {
        this._hide(this._target);
    }

    destroy() {
        this._clearTimeout();
        this._label.destroy();
        this._label = null;
    }

    _schedule(actor, textFor) {
        this._clearTimeout();
        this._target = actor;
        this._timeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, SHOW_DELAY_MS, () => {
            this._timeoutId = 0;
            this._show(actor, textFor());
            return GLib.SOURCE_REMOVE;
        });
    }

    _show(actor, text) {
        if (!text || !actor.mapped) return;
        this._label.text = text;
        this._label.show();
        Main.layoutManager.uiGroup.set_child_above_sibling(this._label, null);
        const [x, y] = actor.get_transformed_position();
        const [width, height] = actor.get_transformed_size();
        const monitor = Main.layoutManager.findMonitorForActor(actor);
        const labelX = Math.round(x + width / 2 - this._label.width / 2);
        const maxX = monitor.x + monitor.width - this._label.width;
        this._label.set_position(Math.min(Math.max(monitor.x, labelX), maxX), Math.round(y + height + GAP));
    }

    _hide(actor) {
        if (actor !== this._target) return;
        this._clearTimeout();
        this._target = null;
        this._label?.hide();
    }

    _clearTimeout() {
        if (this._timeoutId === 0) return;
        GLib.source_remove(this._timeoutId);
        this._timeoutId = 0;
    }
}
