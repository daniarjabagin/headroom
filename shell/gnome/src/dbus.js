import Gio from 'gi://Gio';
import { BUS_NAME, DaemonProxy, OBJECT_PATH, PROXY_FLAGS, remoteMessage } from './daemonInterface.js';
import { parseSettings, serializeSettings, withDisplay } from './settings.js';

export class DaemonClient {
    constructor({ onAvailable, onUnavailable, onState, onSettings, onError, onOpenRequested }) {
        this._handlers = { onAvailable, onUnavailable, onState, onSettings, onError, onOpenRequested };
        this._proxy = null;
        this._signalIds = [];
        this._cancellable = new Gio.Cancellable();
        this._watchId = Gio.bus_watch_name(
            Gio.BusType.SESSION,
            BUS_NAME,
            Gio.BusNameWatcherFlags.NONE,
            () => this._onNameAppeared(),
            () => this._onNameVanished()
        );
    }

    destroy() {
        this._cancellable.cancel();
        Gio.bus_unwatch_name(this._watchId);
        this._dropProxy();
        this._handlers = null;
    }

    refresh(accountId = '') {
        this._call(proxy => proxy.RefreshAsync(accountId));
    }

    setAccountHidden(accountId, hidden) {
        this._call(proxy => proxy.SetAccountHiddenAsync(accountId, hidden));
    }

    setAccountOrder(ids) {
        this._call(proxy => proxy.SetAccountOrderAsync(ids));
    }

    updateDisplay(patchFor) {
        this._call(async proxy => {
            const [json] = await proxy.GetSettingsAsync();
            const settings = parseSettings(json);
            await proxy.SetSettingsAsync(serializeSettings(withDisplay(settings, patchFor(settings.display))));
        });
    }

    _onNameAppeared() {
        if (this._proxy) return;
        DaemonProxy(
            Gio.DBus.session,
            BUS_NAME,
            OBJECT_PATH,
            (proxy, error) => this._onProxyReady(proxy, error),
            this._cancellable,
            PROXY_FLAGS
        );
    }

    _onProxyReady(proxy, error) {
        if (this._cancellable.is_cancelled()) return;
        if (error) {
            this._handlers.onError(remoteMessage(error));
            return;
        }
        this._proxy = proxy;
        this._signalIds = [
            proxy.connectSignal('StateChanged', (_proxy, _sender, [json]) => this._onStateChanged(json)),
            proxy.connectSignal('OpenRequested', () => this._handlers.onOpenRequested()),
        ];
        this._handlers.onAvailable();
        this._loadState();
    }

    _onStateChanged(json) {
        this._handlers.onState(json);
        this._loadSettings();
    }

    async _loadState() {
        const proxy = this._proxy;
        try {
            const [json] = await proxy.GetStateAsync();
            if (this._proxy === proxy) this._handlers.onState(json);
        } catch (error) {
            if (this._proxy === proxy) this._handlers.onError(remoteMessage(error));
        }
        this._loadSettings();
    }

    async _loadSettings() {
        const proxy = this._proxy;
        if (!proxy) return;
        try {
            const [json] = await proxy.GetSettingsAsync();
            if (this._proxy === proxy) this._handlers.onSettings(parseSettings(json));
        } catch (error) {
            if (this._proxy === proxy) this._handlers.onError(remoteMessage(error));
        }
    }

    _onNameVanished() {
        this._dropProxy();
        this._handlers?.onUnavailable();
    }

    _dropProxy() {
        if (!this._proxy) return;
        for (const id of this._signalIds) this._proxy.disconnectSignal(id);
        this._signalIds = [];
        this._proxy = null;
    }

    async _call(invoke) {
        const proxy = this._proxy;
        if (!proxy) return;
        try {
            await invoke(proxy);
        } catch (error) {
            if (this._proxy === proxy) this._handlers.onError(remoteMessage(error));
        }
    }
}
