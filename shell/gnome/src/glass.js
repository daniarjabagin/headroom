import GObject from 'gi://GObject';
import Shell from 'gi://Shell';
import St from 'gi://St';

const BLUR_RADIUS = 36;
const CORNER_INSET = 4;
const GLASS_CLASS = 'headroom-translucent';
const GEOMETRY_SIGNALS = ['notify::allocation', 'notify::translation-x', 'notify::translation-y'];

export class Glass {
    constructor(menu) {
        this._menu = menu;
        this._backdrop = null;
        this._bindings = [];
    }

    setEnabled(enabled) {
        if (enabled === (this._backdrop !== null)) return;
        if (enabled) this._attach();
        else this.destroy();
    }

    destroy() {
        this._menu.actor.remove_style_class_name(GLASS_CLASS);
        if (!this._backdrop) return;
        for (const binding of this._bindings) binding.unbind();
        this._bindings = [];
        this._menu.actor.disconnectObject(this);
        this._menu.box.disconnectObject(this);
        this._backdrop.destroy();
        this._backdrop = null;
    }

    _attach() {
        const { actor, box } = this._menu;
        actor.add_style_class_name(GLASS_CLASS);
        this._backdrop = new St.Widget({ style_class: 'headroom-glass-backdrop', reactive: false });
        this._backdrop.add_effect(
            new Shell.BlurEffect({ mode: Shell.BlurMode.BACKGROUND, radius: BLUR_RADIUS, brightness: 1 })
        );
        actor.get_parent().insert_child_below(this._backdrop, actor);
        this._bindings = ['visible', 'opacity'].map(name =>
            actor.bind_property(name, this._backdrop, name, GObject.BindingFlags.SYNC_CREATE)
        );
        for (const signal of GEOMETRY_SIGNALS) {
            actor.connectObject(signal, () => this._sync(), this);
            box.connectObject(signal, () => this._sync(), this);
        }
        this._sync();
    }

    _sync() {
        const { box } = this._menu;
        const parent = this._backdrop.get_parent();
        const [x, y] = box.get_transformed_position();
        const [width, height] = box.get_transformed_size();
        const inset = CORNER_INSET * St.ThemeContext.get_for_stage(global.stage).scale_factor;
        const [, left, top] = parent.transform_stage_point(x + inset, y + inset);
        this._backdrop.set_position(Math.round(left), Math.round(top));
        this._backdrop.set_size(
            Math.max(0, Math.round(width - 2 * inset)),
            Math.max(0, Math.round(height - 2 * inset))
        );
    }
}
