export const MIN_SPIN_MS = 800;
export const ERROR_MS = 1000;

export class RefreshTracker {
    constructor() {
        this._inFlight = false;
        this._holdUntil = 0;
        this._errorUntil = 0;
        this._daemonBusy = false;
    }

    press(now) {
        if (this._inFlight || now < this._holdUntil) return false;
        this._inFlight = true;
        this._holdUntil = now + MIN_SPIN_MS;
        this._errorUntil = 0;
        return true;
    }

    settle(succeeded, now) {
        this._inFlight = false;
        if (succeeded) return;
        this._holdUntil = 0;
        this._errorUntil = now + ERROR_MS;
    }

    setDaemonBusy(busy) {
        this._daemonBusy = busy;
    }

    mode(now) {
        if (now < this._errorUntil) return 'error';
        if (this._inFlight || now < this._holdUntil || this._daemonBusy) return 'busy';
        return 'idle';
    }

    nextChange(now) {
        const upcoming = [this._holdUntil, this._errorUntil].filter(deadline => deadline > now);
        return upcoming.length > 0 ? Math.min(...upcoming) : null;
    }
}
