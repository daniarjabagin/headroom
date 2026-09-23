import Clutter from 'gi://Clutter';
import { _ } from '../i18n.js';
import { EASE } from '../motion.js';
import { iconButton } from '../widgets.js';

const FULL_TURN = 360;
const TURN_MS = 900;
const ROTATION = 'rotation-angle-z';

export class RefreshButton {
    constructor(ctx) {
        this._motion = ctx.motion;
        this._busy = false;
        this.actor = iconButton('view-refresh-symbolic', _('Refresh'), () => ctx.actions.refresh(''));
        this.actor.add_style_class_name('headroom-refresh-button');
        this._icon = this.actor.child;
        this._icon.set_pivot_point(0.5, 0.5);
        ctx.tooltips.attach(this.actor, () => _('Refresh'));
    }

    setBusy(busy) {
        if (busy === this._busy) return;
        this._busy = busy;
        if (busy) this.actor.add_style_class_name('busy');
        else this.actor.remove_style_class_name('busy');
        if (busy && this._motion.enabled) this._spin();
        else this._finishTurn();
    }

    _spin() {
        this._icon.remove_transition(ROTATION);
        this._icon.rotation_angle_z = 0;
        this._icon.ease({
            rotation_angle_z: FULL_TURN,
            duration: TURN_MS,
            mode: Clutter.AnimationMode.LINEAR,
            repeatCount: -1,
        });
    }

    _finishTurn() {
        const angle = this._icon.rotation_angle_z % FULL_TURN;
        this._icon.remove_transition(ROTATION);
        if (angle === 0 || !this._motion.enabled) {
            this._icon.rotation_angle_z = 0;
            return;
        }
        this._icon.rotation_angle_z = angle;
        this._icon.ease({
            rotation_angle_z: FULL_TURN,
            duration: Math.round(((FULL_TURN - angle) / FULL_TURN) * TURN_MS),
            mode: EASE,
            onComplete: () => (this._icon.rotation_angle_z = 0),
        });
    }
}
