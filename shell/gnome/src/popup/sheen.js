import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import St from 'gi://St';
import {
    pointerQuiet,
    QUIET_MS,
    REST_MS,
    sheenClip,
    sheenOffset,
    sheenStrength,
    START_DELAY_MS,
    STEP_MS,
    SWEEP_MS,
    sweepProgress,
} from './sheenPlan.js';

const OPAQUE = 255;

function childBox(x, y, width, height) {
    const box = new Clutter.ActorBox();
    box.set_origin(x, y);
    box.set_size(width, height);
    return box;
}

const MICROS_PER_MS = 1000;
const INTERACTIONS = new Set([
    Clutter.EventType.MOTION,
    Clutter.EventType.SCROLL,
    Clutter.EventType.BUTTON_PRESS,
    Clutter.EventType.TOUCH_BEGIN,
    Clutter.EventType.TOUCH_UPDATE,
]);

function nowMs() {
    return GLib.get_monotonic_time() / MICROS_PER_MS;
}

export class SheenClock {
    constructor(motion) {
        this._motion = motion;
        this._bands = new Set();
        this._actor = null;
        this._sourceId = 0;
        this._sweepStart = 0;
        this._lastInteraction = -QUIET_MS;
        this._running = false;
    }

    attach(actor) {
        this._actor = actor;
        actor.connect('captured-event', (_actor, event) => this._noteEvent(event));
    }

    add(band) {
        this._bands.add(band);
    }

    remove(band) {
        this._bands.delete(band);
    }

    start() {
        if (this._running) return;
        this._running = true;
        this._wait(START_DELAY_MS);
    }

    stop() {
        this._running = false;
        this._clearSource();
        this._park();
    }

    _noteEvent(event) {
        if (INTERACTIONS.has(event.type())) this._lastInteraction = nowMs();
        return Clutter.EVENT_PROPAGATE;
    }

    _quiet() {
        return pointerQuiet(nowMs() - this._lastInteraction);
    }

    _wait(delayMs) {
        this._clearSource();
        this._sourceId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, delayMs, () => {
            this._sourceId = 0;
            this._sweep();
            return GLib.SOURCE_REMOVE;
        });
    }

    _clearSource() {
        if (this._sourceId === 0) return;
        GLib.source_remove(this._sourceId);
        this._sourceId = 0;
    }

    _canSweep() {
        return this._motion.enabled && Boolean(this._actor?.mapped) && [...this._bands].some(band => band.shown);
    }

    _sweep() {
        if (!this._running) return;
        if (!this._canSweep() || !this._quiet()) {
            this._wait(this._quiet() ? REST_MS : QUIET_MS);
            return;
        }
        this._sweepStart = nowMs();
        this._place(0);
        this._sourceId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, STEP_MS, () => this._step());
    }

    _step() {
        const elapsed = nowMs() - this._sweepStart;
        if (elapsed < SWEEP_MS && this._canSweep()) {
            this._place(sweepProgress(elapsed));
            return GLib.SOURCE_CONTINUE;
        }
        this._sourceId = 0;
        this._park();
        if (this._running) this._wait(REST_MS);
        return GLib.SOURCE_REMOVE;
    }

    _place(progress) {
        for (const band of this._bands) band.place(progress);
    }

    _park() {
        for (const band of this._bands) band.park();
    }
}

export class SheenBand {
    constructor(clock) {
        this.actor = new St.Widget({
            style_class: 'headroom-meter-sheen',
            layout_manager: new Clutter.FixedLayout(),
            clip_to_allocation: true,
        });
        this._band = new St.BoxLayout({ style_class: 'headroom-meter-sheen-band' });
        this._band.add_child(new St.Widget({ style_class: 'headroom-meter-sheen-lead', x_expand: true }));
        this._band.add_child(new St.Widget({ style_class: 'headroom-meter-sheen-trail', x_expand: true }));
        this.actor.add_child(this._band);
        this._clipWidth = 0;
        clock.add(this);
        this.actor.connect('destroy', () => clock.remove(this));
        this.park();
    }

    get shown() {
        return this.actor.visible && this._clipWidth > 0;
    }

    setVisible(visible) {
        this.actor.visible = visible;
    }

    allocate(fillX, y, fillWidth, height, shown) {
        const clip = sheenClip(fillX, shown ? fillWidth : 0, height);
        this._clipWidth = clip.width;
        this.actor.allocate(childBox(clip.x, y, clip.width, height));
    }

    place(progress) {
        if (!this.shown) return;
        const bandWidth = this._band.width;
        const offset = sheenOffset(progress, this._clipWidth, bandWidth);
        this._band.translation_x = offset;
        this._band.opacity = Math.round(OPAQUE * sheenStrength(offset, this._clipWidth, bandWidth));
    }

    park() {
        this._band.opacity = 0;
    }
}
