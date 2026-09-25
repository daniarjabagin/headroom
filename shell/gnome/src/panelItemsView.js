import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { animate, pulse, stopPulse } from './motion.js';
import { PanelBar } from './panelBar.js';
import { sameKeys } from './panelContent.js';
import { PanelRing } from './panelRing.js';
import { fileIcon, label, providerIcon, row } from './widgets.js';

const STALE_OPACITY = 140;
const PULSE_OPACITY = 140;
const PULSE_PERIOD_MS = 2000;
const WINDOW_LABEL_OPACITY = 170;
const TONE_CLASSES = ['warning', 'critical'];
const LEAVES = { ring: PanelRing, bar: PanelBar };
const PANEL_THEME = {
    light: ['headroom-theme-light', 'headroom-panel-light'],
    dark: ['headroom-theme-dark'],
};

function setToneClass(actor, tone) {
    for (const name of TONE_CLASSES) if (name !== tone) actor.remove_style_class_name(name);
    if (TONE_CLASSES.includes(tone)) actor.add_style_class_name(tone);
}

function setText(widget, text) {
    widget.visible = text !== null;
    if (text !== null && widget.text !== text) widget.text = text;
}

class Pulse {
    constructor(motion, actor) {
        this._motion = motion;
        this._actor = actor;
        this._critical = false;
        this._running = false;
        this._mappedId = actor.connect('notify::mapped', () => this.sync());
    }

    setCritical(critical) {
        this._critical = critical;
        this.sync();
    }

    sync() {
        const running = this._critical && this._actor.mapped && this._motion.enabled;
        if (running === this._running) return;
        this._running = running;
        if (running) pulse(this._motion, this._actor, PULSE_OPACITY, PULSE_PERIOD_MS);
        else stopPulse(this._actor);
    }

    destroy() {
        this._actor.disconnect(this._mappedId);
        stopPulse(this._actor);
    }
}

class PanelItemView {
    constructor(ctx, key) {
        this.key = key;
        this.fresh = true;
        this._ctx = ctx;
        this._logo = null;
        this._kind = null;
        this._leaf = null;
        this.actor = new St.Bin({ y_align: Clutter.ActorAlign.CENTER });
        this._row = row({ style_class: 'headroom-panel-item' });
        this.actor.set_child(this._row);
        this._logoSlot = new St.Bin({ style_class: 'headroom-panel-provider', y_align: Clutter.ActorAlign.CENTER });
        this._count = label('', 'headroom-panel-window headroom-panel-count');
        this._letter = label('', 'headroom-panel-window');
        this._count.opacity = WINDOW_LABEL_OPACITY;
        this._letter.opacity = WINDOW_LABEL_OPACITY;
        this._leafSlot = new St.Bin({ y_align: Clutter.ActorAlign.CENTER });
        this._value = label('', 'headroom-panel-label');
        for (const actor of [this._logoSlot, this._count, this._letter, this._leafSlot, this._value])
            this._row.add_child(actor);
        this._pulse = new Pulse(ctx.motion, this._row);
    }

    update(spec) {
        this._setLogo(spec.logo);
        setText(this._count, spec.count);
        setText(this._letter, spec.letter);
        setText(this._value, spec.value);
        setToneClass(this._value, spec.tone);
        this._setLeaf(spec.indicator);
        this._leaf?.update(spec);
        this._pulse.setCritical(spec.tone === 'critical');
    }

    setOpacity(stale, entering) {
        const opacity = stale ? STALE_OPACITY : 255;
        if (!entering) {
            this.actor.remove_transition('opacity');
            this.actor.opacity = opacity;
            return;
        }
        this.actor.opacity = 0;
        animate(this._ctx.motion, this.actor, { opacity });
    }

    syncMotion() {
        this._pulse.sync();
    }

    _setLogo(logo) {
        this._logoSlot.visible = logo !== null;
        if (logo === this._logo) return;
        this._logo = logo;
        this._logoSlot.child?.destroy();
        if (logo !== null) this._logoSlot.set_child(providerIcon(this._ctx.dir, logo, 'headroom-panel-provider-icon'));
    }

    _setLeaf(kind) {
        this._leafSlot.visible = kind !== null;
        if (kind === this._kind) return;
        this._kind = kind;
        this._leafSlot.child?.destroy();
        this._leaf = kind === null ? null : new LEAVES[kind](this._ctx.motion);
        if (this._leaf) this._leafSlot.set_child(this._leaf);
    }

    destroy() {
        this._pulse.destroy();
        this.actor.destroy();
    }
}

export class PanelItemsView {
    constructor({ dir, motion }) {
        this._ctx = { dir, motion };
        this._views = new Map();
        this._keys = [];
        this._style = null;
        this.actor = row({ style_class: 'headroom-panel-box', y_align: Clutter.ActorAlign.CENTER });
        this._mark = fileIcon(dir, 'headroom-symbolic.svg', 'system-status-icon headroom-panel-mark');
        this._markPulse = new Pulse(motion, this._mark);
        this.actor.add_child(this._mark);
        this._animationsId = St.Settings.get().connect('notify::enable-animations', () => this.syncMotion());
        this.setLightPanel(false);
    }

    render(layout) {
        this._mark.visible = layout.mark !== null;
        setToneClass(this._mark, layout.mark?.tone ?? null);
        this._markPulse.setCritical(layout.mark?.tone === 'critical');
        const entering = this._keys.length > 0 && this._style === layout.style;
        this._style = layout.style;
        this._renderItems(layout.items, entering);
    }

    setLightPanel(light) {
        const wanted = light ? PANEL_THEME.light : PANEL_THEME.dark;
        for (const name of [...PANEL_THEME.light, ...PANEL_THEME.dark])
            if (!wanted.includes(name)) this.actor.remove_style_class_name(name);
        for (const name of wanted) this.actor.add_style_class_name(name);
    }

    syncMotion() {
        this._markPulse.sync();
        for (const view of this._views.values()) view.syncMotion();
    }

    destroy() {
        St.Settings.get().disconnect(this._animationsId);
        for (const view of this._views.values()) view.destroy();
        this._views.clear();
        this._markPulse.destroy();
        this.actor.destroy();
    }

    _renderItems(items, entering) {
        const keys = items.map(item => item.key);
        if (!sameKeys(keys, this._keys)) this._reconcile(keys);
        for (const item of items) {
            const view = this._views.get(item.key);
            const added = view.fresh;
            view.fresh = false;
            view.update(item);
            view.setOpacity(item.stale, added && entering);
        }
        this._keys = keys;
    }

    _reconcile(keys) {
        for (const [key, view] of this._views) {
            if (keys.includes(key)) continue;
            view.destroy();
            this._views.delete(key);
        }
        keys.forEach((key, index) => {
            let view = this._views.get(key);
            if (!view) {
                view = new PanelItemView(this._ctx, key);
                this._views.set(key, view);
                this.actor.add_child(view.actor);
            }
            this.actor.set_child_at_index(view.actor, index + 1);
        });
    }
}
