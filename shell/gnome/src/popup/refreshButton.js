import Clutter from 'gi://Clutter';
import { _ } from '../i18n.js';
import { EASE_MS, FULL_TURN, QUARTER_TURN, restingAngle, stopPlan, TURN_MS } from '../spin.js';
import { iconButton } from '../widgets.js';

const ROTATION = 'rotation-angle-z';
const DIMMED_OPACITY = 140;
const SHAKE_OFFSETS = [3, -3, 2, -2, 1, 0];
const SHAKE_STEP_MS = 66;

export class RefreshButton {
    constructor(ctx) {
        this._motion = ctx.motion;
        this._mode = 'idle';
        this._phase = 0;
        this._shakePhase = 0;
        this.actor = iconButton('view-refresh-symbolic', _('Refresh'), () => ctx.pressRefresh());
        this.actor.add_style_class_name('headroom-refresh-button');
        this._icon = this.actor.child;
        this._icon.set_pivot_point(0.5, 0.5);
        this._icon.connect('notify::mapped', () => this._resume());
        ctx.tooltips.attach(this.actor, () => (this._mode === 'error' ? _('Refresh failed') : _('Refresh')));
    }

    relabel() {
        this.actor.accessible_name = _('Refresh');
    }

    show(mode) {
        if (mode === this._mode) return;
        this._mode = mode;
        this._toggleClass('busy', mode === 'busy');
        this._toggleClass('failed', mode === 'error');
        if (mode === 'busy') this._startSpin();
        else this._stopSpin();
        if (mode === 'error') this._shake();
    }

    _resume() {
        if (!this._icon.mapped || !this._motion.enabled) return;
        const angle = restingAngle(this._icon.rotation_angle_z);
        if (this._mode !== 'busy') this._stopSpin();
        else if (angle === 0) this._startSpin();
        else this._cruise(angle);
    }

    _toggleClass(name, enabled) {
        if (enabled) this.actor.add_style_class_name(name);
        else this.actor.remove_style_class_name(name);
    }

    _startSpin() {
        if (!this._motion.enabled) {
            this._icon.opacity = DIMMED_OPACITY;
            return;
        }
        this._icon.opacity = 255;
        const from = restingAngle(this._icon.rotation_angle_z);
        this._rotate(from, from + QUARTER_TURN, EASE_MS, Clutter.AnimationMode.EASE_IN_QUAD, () =>
            this._cruise(from + QUARTER_TURN)
        );
    }

    _cruise(from) {
        this._rotate(from, from + FULL_TURN, TURN_MS, Clutter.AnimationMode.LINEAR, null, -1);
    }

    _stopSpin() {
        this._icon.opacity = 255;
        const plan = this._motion.enabled ? stopPlan(this._icon.rotation_angle_z) : null;
        if (plan === null) {
            this._rest();
            return;
        }
        const settle = () =>
            this._rotate(plan.cruiseTo, plan.target, EASE_MS, Clutter.AnimationMode.EASE_OUT_QUAD, () => this._rest());
        if (plan.cruiseMs === 0) settle();
        else this._rotate(plan.from, plan.cruiseTo, plan.cruiseMs, Clutter.AnimationMode.LINEAR, settle);
    }

    _rotate(from, to, duration, mode, then, repeatCount = 0) {
        const phase = this._restart();
        this._icon.rotation_angle_z = from;
        this._icon.ease({
            rotation_angle_z: to,
            duration,
            mode,
            repeatCount,
            onComplete: () => {
                if (phase === this._phase) then?.();
            },
        });
    }

    _rest() {
        this._restart();
        this._icon.rotation_angle_z = 0;
    }

    _restart() {
        this._phase += 1;
        this._icon.remove_transition(ROTATION);
        return this._phase;
    }

    _shake() {
        if (!this._motion.enabled) return;
        const phase = ++this._shakePhase;
        const step = index => {
            if (phase !== this._shakePhase || index >= SHAKE_OFFSETS.length) return;
            this.actor.ease({
                translation_x: SHAKE_OFFSETS[index],
                duration: SHAKE_STEP_MS,
                mode: Clutter.AnimationMode.EASE_IN_OUT_SINE,
                onComplete: () => step(index + 1),
            });
        };
        step(0);
    }
}
