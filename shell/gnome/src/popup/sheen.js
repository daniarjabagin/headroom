import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import St from 'gi://St';
import { REST_MS, sheenClip, sheenOffset, sheenStrength, START_DELAY_MS, SWEEP_MS } from './sheenPlan.js';

const OPAQUE = 255;

function childBox(x, y, width, height) {
    const box = new Clutter.ActorBox();
    box.set_origin(x, y);
    box.set_size(width, height);
    return box;
}

export class SheenClock {
    constructor(motion) {
        this._motion = motion;
        this._bands = new Set();
        this._actor = null;
        this._timeline = null;
        this._waitId = 0;
        this._running = false;
    }

    attach(actor) {
        this._actor = actor;
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
        this._clearWait();
        this._timeline?.stop();
        this._timeline = null;
        this._park();
    }

    _wait(delayMs) {
        this._clearWait();
        this._waitId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, delayMs, () => {
            this._waitId = 0;
            this._sweep();
            return GLib.SOURCE_REMOVE;
        });
    }

    _clearWait() {
        if (this._waitId === 0) return;
        GLib.source_remove(this._waitId);
        this._waitId = 0;
    }

    _canSweep() {
        return this._motion.enabled && Boolean(this._actor?.mapped) && [...this._bands].some(band => band.shown);
    }

    _sweep() {
        if (!this._running) return;
        if (!this._canSweep()) {
            this._wait(REST_MS);
            return;
        }
        this._timeline ??= this._createTimeline();
        this._timeline.rewind();
        this._timeline.start();
    }

    _createTimeline() {
        const timeline = new Clutter.Timeline({
            actor: this._actor,
            duration: SWEEP_MS,
            progress_mode: Clutter.AnimationMode.EASE_IN_OUT_SINE,
        });
        timeline.connect('new-frame', () => this._place(timeline.get_progress()));
        timeline.connect('completed', () => {
            this._park();
            if (this._running) this._wait(REST_MS);
        });
        return timeline;
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
