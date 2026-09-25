export class ShareHandles {
    constructor() {
        this._active = new Set();
        this._revealed = false;
    }

    get sharing() {
        return this._active.size > 0;
    }

    get revealed() {
        return this._revealed;
    }

    add(handle) {
        if (this._active.has(handle)) return false;
        this._active.add(handle);
        return this._active.size === 1;
    }

    stop(handle) {
        if (!this._active.delete(handle)) return false;
        if (this.sharing) return false;
        this._revealed = false;
        return true;
    }

    reveal() {
        if (!this.sharing || this._revealed) return false;
        this._revealed = true;
        return true;
    }

    masked(enabled) {
        return enabled && this.sharing && !this._revealed;
    }

    clear() {
        this._active.clear();
        this._revealed = false;
    }
}
