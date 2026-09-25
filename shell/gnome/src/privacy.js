import { EventEmitter } from 'resource:///org/gnome/shell/misc/signals.js';
import { ShareHandles } from './shareHandles.js';

function remoteAccessController() {
    return global.backend?.get_remote_access_controller?.() ?? null;
}

export class Privacy extends EventEmitter {
    constructor() {
        super();
        this._handles = new ShareHandles();
        this._stoppedIds = new Map();
        this._enabled = true;
        this._masked = false;
        this._controller = remoteAccessController();
        this._newHandleId = this._controller?.connect('new-handle', (_controller, handle) => this._track(handle)) ?? 0;
    }

    isMasked() {
        return this._masked;
    }

    get sharing() {
        return this._handles.sharing;
    }

    setEnabled(enabled) {
        this._enabled = enabled;
        this._masked = this._handles.masked(enabled);
    }

    showAnyway() {
        if (this._handles.reveal()) this._sync();
    }

    destroy() {
        if (this._newHandleId) this._controller.disconnect(this._newHandleId);
        this._newHandleId = 0;
        for (const [handle, id] of this._stoppedIds) handle.disconnect(id);
        this._stoppedIds.clear();
        this._handles.clear();
        this._controller = null;
        this.disconnectAll();
    }

    _track(handle) {
        if (this._stoppedIds.has(handle)) return;
        this._handles.add(handle);
        this._stoppedIds.set(
            handle,
            handle.connect('stopped', () => this._untrack(handle))
        );
        this._sync();
    }

    _untrack(handle) {
        const id = this._stoppedIds.get(handle);
        if (id) handle.disconnect(id);
        this._stoppedIds.delete(handle);
        this._handles.stop(handle);
        this._sync();
    }

    _sync() {
        const masked = this._handles.masked(this._enabled);
        if (masked === this._masked) return;
        this._masked = masked;
        this.emit('changed', masked);
    }
}
