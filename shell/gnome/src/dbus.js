import { DaemonConnection } from './daemonConnection.js';
import { remoteMessage } from './daemonInterface.js';
import { parseSettings } from './settings.js';

export class DaemonClient {
    constructor({ onAvailable, onUnavailable, onState, onSettings, onError, onOpenRequested }) {
        this._handlers = { onAvailable, onUnavailable, onState, onSettings, onError, onOpenRequested };
        this._connection = new DaemonConnection({
            signals: {
                StateChanged: json => this._onStateChanged(json),
                OpenRequested: () => this._handlers.onOpenRequested(),
            },
            onReady: () => this._onReady(),
            onVanished: () => this._handlers?.onUnavailable(),
            onError: message => this._handlers?.onError(message),
        });
    }

    destroy() {
        this._connection.destroy();
        this._handlers = null;
    }

    refresh(accountId = '') {
        this._connection.enqueue(proxy => proxy.RefreshAsync(accountId));
    }

    refreshNow() {
        return this._connection.enqueue(proxy => proxy.RefreshNowAsync());
    }

    setAccountOrder(ids) {
        this._connection.enqueue(proxy => proxy.SetAccountOrderAsync(ids));
    }

    updateSettings(patch) {
        this._connection.enqueue(proxy => proxy.UpdateSettingsAsync(JSON.stringify(patch)));
    }

    _onReady() {
        this._handlers.onAvailable();
        this._loadState();
    }

    _onStateChanged(json) {
        this._handlers.onState(json);
        this._loadSettings();
    }

    async _loadState() {
        await this._read(
            proxy => proxy.GetStateAsync(),
            json => this._handlers.onState(json)
        );
        this._loadSettings();
    }

    _loadSettings() {
        return this._read(
            proxy => proxy.GetSettingsAsync(),
            json => this._handlers.onSettings(parseSettings(json))
        );
    }

    async _read(fetch, accept) {
        const proxy = this._connection.proxy;
        if (!proxy) return;
        try {
            const [json] = await fetch(proxy);
            if (this._connection.proxy === proxy) accept(json);
        } catch (error) {
            if (this._connection.proxy === proxy) this._handlers?.onError(remoteMessage(error));
        }
    }
}
