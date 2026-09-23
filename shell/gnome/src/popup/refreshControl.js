import GLib from 'gi://GLib';
import { RefreshTracker } from '../refreshTracker.js';

export class RefreshControl {
    constructor(refreshNow) {
        this._refreshNow = refreshNow;
        this._tracker = new RefreshTracker();
        this._button = null;
        this._timeoutId = 0;
        this._destroyed = false;
    }

    attach(button) {
        this._button = button;
        this._button?.show(this._tracker.mode(Date.now()));
    }

    press() {
        if (!this._tracker.press(Date.now())) return;
        this._sync();
        this._refreshNow().then(
            succeeded => this._settle(succeeded),
            () => this._settle(false)
        );
    }

    setDaemonBusy(busy) {
        this._tracker.setDaemonBusy(busy);
        this._sync();
    }

    destroy() {
        this._destroyed = true;
        this._clearTimeout();
        this._button = null;
    }

    _settle(succeeded) {
        if (this._destroyed) return;
        this._tracker.settle(succeeded, Date.now());
        this._sync();
    }

    _sync() {
        const now = Date.now();
        this._button?.show(this._tracker.mode(now));
        this._schedule(this._tracker.nextChange(now), now);
    }

    _schedule(deadline, now) {
        this._clearTimeout();
        if (deadline === null) return;
        this._timeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, deadline - now, () => {
            this._timeoutId = 0;
            this._sync();
            return GLib.SOURCE_REMOVE;
        });
    }

    _clearTimeout() {
        if (this._timeoutId) GLib.source_remove(this._timeoutId);
        this._timeoutId = 0;
    }
}
