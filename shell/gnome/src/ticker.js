import GLib from 'gi://GLib';

const SLOW_SECONDS = 30;
const FAST_MS = 1000;

export class Ticker {
    constructor({ onTick, wantsSeconds }) {
        this._onTick = onTick;
        this._wantsSeconds = wantsSeconds;
        this._sourceId = 0;
        this._fast = false;
    }

    start() {
        this.stop();
        this._schedule(this._wantsSeconds());
    }

    sync() {
        if (this._sourceId === 0) return;
        const fast = this._wantsSeconds();
        if (fast === this._fast) return;
        this.stop();
        this._schedule(fast);
    }

    stop() {
        if (this._sourceId === 0) return;
        GLib.source_remove(this._sourceId);
        this._sourceId = 0;
    }

    _schedule(fast) {
        this._fast = fast;
        const tick = () => this._tick();
        this._sourceId = fast
            ? GLib.timeout_add(GLib.PRIORITY_DEFAULT, FAST_MS, tick)
            : GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, SLOW_SECONDS, tick);
    }

    _tick() {
        this._onTick();
        if (this._wantsSeconds() === this._fast) return GLib.SOURCE_CONTINUE;
        this._sourceId = 0;
        this._schedule(!this._fast);
        return GLib.SOURCE_REMOVE;
    }
}
