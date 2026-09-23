import Gio from 'gi://Gio';
import { BUS_NAME, DaemonProxy, OBJECT_PATH, PROXY_FLAGS, remoteMessage } from '../daemonInterface.js';
import { parseSettings, serializeSettings } from '../settings.js';
import { parseState } from '../state.js';

export class PrefsClient {
    constructor({ onAvailable, onUnavailable, onState, onSettings, onError }) {
        this._handlers = { onAvailable, onUnavailable, onState, onSettings, onError };
        this._proxy = null;
        this._signalId = 0;
        this.state = null;
        this.settings = null;
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

    updateSettings(change) {
        this._call(async proxy => {
            const [json] = await proxy.GetSettingsAsync();
            await proxy.SetSettingsAsync(serializeSettings(change(parseSettings(json))));
            await this._loadSettings(proxy);
        });
    }

    setAccountLabel(accountId, label) {
        this._call(proxy => proxy.SetAccountLabelAsync(accountId, label));
    }

    setAccountHidden(accountId, hidden) {
        this._call(proxy => proxy.SetAccountHiddenAsync(accountId, hidden));
    }

    setAccountOrder(ids) {
        this._call(proxy => proxy.SetAccountOrderAsync(ids));
    }

    rescan() {
        this._call(proxy => proxy.RescanAsync());
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
        this._signalId = proxy.connectSignal('StateChanged', (_proxy, _sender, [json]) => this._acceptState(json));
        this._call(async current => {
            const [json] = await current.GetStateAsync();
            this._acceptState(json);
            await this._loadSettings(current);
            this._handlers.onAvailable();
        });
    }

    async _loadSettings(proxy) {
        const [json] = await proxy.GetSettingsAsync();
        if (this._proxy !== proxy) return;
        this.settings = parseSettings(json);
        this._handlers.onSettings(this.settings);
    }

    _acceptState(json) {
        if (!this._handlers) return;
        try {
            this.state = parseState(json);
        } catch (error) {
            this._handlers.onError(error.message);
            return;
        }
        this._handlers.onState(this.state);
    }

    _onNameVanished() {
        this._dropProxy();
        this.state = null;
        this.settings = null;
        this._handlers?.onUnavailable();
    }

    _dropProxy() {
        if (!this._proxy) return;
        this._proxy.disconnectSignal(this._signalId);
        this._signalId = 0;
        this._proxy = null;
    }

    async _call(invoke) {
        const proxy = this._proxy;
        if (!proxy) return;
        try {
            await invoke(proxy);
        } catch (error) {
            if (this._proxy === proxy && this._handlers) this._handlers.onError(remoteMessage(error));
        }
    }
}
