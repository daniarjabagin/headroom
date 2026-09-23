import Clutter from 'gi://Clutter';
import St from 'gi://St';

export const FAST_MS = 120;
export const STANDARD_MS = 200;
export const STAGGER_MS = 30;
const RISE_PX = 4;
export const EASE = Clutter.AnimationMode.EASE_OUT_CUBIC;

export class Motion {
    constructor() {
        this.reduced = false;
    }

    get enabled() {
        return St.Settings.get().enable_animations && !this.reduced;
    }
}

function stopTransitions(actor, names) {
    for (const name of names) actor.remove_transition(name.replace(/_/g, '-'));
}

export function animate(motion, actor, props, { duration = STANDARD_MS, delay = 0, onComplete = null } = {}) {
    stopTransitions(actor, Object.keys(props));
    if (!motion.enabled) {
        Object.assign(actor, props);
        onComplete?.();
        return;
    }
    actor.ease({ ...props, duration, delay, mode: EASE, onComplete: () => onComplete?.() });
}

export function enter(motion, actor, index) {
    settle(actor);
    if (!motion.enabled) return;
    actor.opacity = 0;
    actor.translation_y = RISE_PX;
    actor.ease({ opacity: 255, translation_y: 0, duration: STANDARD_MS, delay: index * STAGGER_MS, mode: EASE });
}

export function settle(actor) {
    stopTransitions(actor, ['opacity', 'translation_y']);
    actor.opacity = 255;
    actor.translation_y = 0;
}

export function pulse(motion, actor, lowOpacity, periodMs) {
    stopPulse(actor);
    if (!motion.enabled) return;
    actor.ease({
        opacity: lowOpacity,
        duration: periodMs / 2,
        mode: Clutter.AnimationMode.EASE_IN_OUT_SINE,
        autoReverse: true,
        repeatCount: -1,
    });
}

export function stopPulse(actor) {
    actor.remove_transition('opacity');
    actor.opacity = 255;
}
