import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const SHOW_DELAY_MS = 400;
const WARM_MS = 300;
const FADE_MS = 120;
const GAP = 6;
const SIDE_GAP = 10;
const SIDE_LIFT = 10;

function contentActor(content) {
    if (typeof content !== 'string') return content;
    const text = new St.Label({ text: content, style_class: 'headroom-tooltip-text' });
    text.clutter_text.line_wrap = true;
    return text;
}

export class Tooltips {
    constructor(motion) {
        this._motion = motion;
        this._bin = new St.Bin({ style_class: 'headroom-tooltip', visible: false, opacity: 0 });
        Main.layoutManager.uiGroup.add_child(this._bin);
        this._timeoutId = 0;
        this._target = null;
        this._hiddenAt = 0;
        this._sideAnchor = null;
        this._current = null;
    }

    setSideAnchor(actor) {
        this._sideAnchor = actor;
    }

    setTheme(themeClass) {
        this._bin.style_class = `headroom-tooltip ${themeClass}`.trim();
    }

    attach(actor, contentFor, { side = false } = {}) {
        actor.track_hover = true;
        actor.reactive = true;
        actor.connect('notify::hover', () =>
            actor.hover ? this._schedule(actor, contentFor, side) : this._hide(actor)
        );
        actor.connect('destroy', () => this._hide(actor));
    }

    hide() {
        this._hide(this._target);
    }

    refresh(actor) {
        if (actor !== this._target || !this._bin?.visible || !this._current) return;
        const content = this._current.contentFor();
        if (!content) {
            this._hide(actor);
            return;
        }
        this._fill(content);
        this._position(actor, this._current.side);
    }

    destroy() {
        this._clearTimeout();
        this._bin.destroy();
        this._bin = null;
    }

    _schedule(actor, contentFor, side) {
        this._clearTimeout();
        this._target = actor;
        this._current = { contentFor, side };
        if (Date.now() - this._hiddenAt < WARM_MS) {
            this._show(actor, contentFor(), side, false);
            return;
        }
        this._timeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, SHOW_DELAY_MS, () => {
            this._timeoutId = 0;
            this._show(actor, contentFor(), side);
            return GLib.SOURCE_REMOVE;
        });
    }

    _show(actor, content, side, fade = true) {
        if (!content || !actor.mapped) return;
        this._fill(content);
        this._bin.show();
        Main.layoutManager.uiGroup.set_child_above_sibling(this._bin, null);
        this._position(actor, side);
        this._bin.remove_all_transitions();
        if (fade && this._motion.enabled)
            this._bin.ease({ opacity: 255, duration: FADE_MS, mode: Clutter.AnimationMode.EASE_OUT_QUAD });
        else this._bin.opacity = 255;
    }

    _fill(content) {
        this._bin.child?.destroy();
        this._bin.set_child(contentActor(content));
    }

    _position(actor, side) {
        if (side && this._sideAnchor?.mapped) this._placeBeside(actor);
        else this._place(actor);
    }

    _place(actor) {
        const [x, y] = actor.get_transformed_position();
        const [width, height] = actor.get_transformed_size();
        const [, tipWidth] = this._bin.get_preferred_width(-1);
        const [, tipHeight] = this._bin.get_preferred_height(tipWidth);
        const monitor = Main.layoutManager.findMonitorForActor(actor);
        const left = Math.round(x + width / 2 - tipWidth / 2);
        const tipX = Math.min(Math.max(monitor.x, left), monitor.x + monitor.width - tipWidth);
        const below = Math.round(y + height + GAP);
        const fitsBelow = below + tipHeight <= monitor.y + monitor.height;
        this._bin.set_position(tipX, fitsBelow ? below : Math.round(y - GAP - tipHeight));
    }

    _placeBeside(actor) {
        const [anchorX] = this._sideAnchor.get_transformed_position();
        const [anchorWidth] = this._sideAnchor.get_transformed_size();
        const [, y] = actor.get_transformed_position();
        const [, tipWidth] = this._bin.get_preferred_width(-1);
        const [, tipHeight] = this._bin.get_preferred_height(tipWidth);
        const monitor = Main.layoutManager.findMonitorForActor(actor);
        const left = Math.round(anchorX - SIDE_GAP - tipWidth);
        const tipX = left >= monitor.x ? left : Math.round(anchorX + anchorWidth + SIDE_GAP);
        const top = Math.min(Math.max(monitor.y, Math.round(y - SIDE_LIFT)), monitor.y + monitor.height - tipHeight);
        this._bin.set_position(tipX, top);
    }

    _hide(actor) {
        if (actor !== this._target) return;
        this._clearTimeout();
        this._target = null;
        this._current = null;
        if (!this._bin) return;
        if (this._bin.visible) this._hiddenAt = Date.now();
        this._bin.remove_all_transitions();
        this._bin.opacity = 0;
        this._bin.hide();
    }

    _clearTimeout() {
        if (this._timeoutId === 0) return;
        GLib.source_remove(this._timeoutId);
        this._timeoutId = 0;
    }
}
