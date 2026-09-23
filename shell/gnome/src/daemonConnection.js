import Gio from 'gi://Gio';
import { BUS_NAME, DaemonProxy, OBJECT_PATH, PROXY_FLAGS, remoteMessage } from './daemonInterface.js';
import { SerialQueue } from './serialQueue.js';

export class DaemonConnection {
    constructor({ signals, onReady, onVanished, onError }) {
        this._signals = signals;
        this._handlers = { onReady, onVanished, onError };
        this._appearance = null;
        this._signalIds = [];
        this._queue = new SerialQueue();
        this.proxy = null;
        this._watchId = Gio.bus_watch_name(
            Gio.BusType.SESSION,
            BUS_NAME,
            Gio.BusNameWatcherFlags.NONE,
            () => this._onNameAppeared(),
            () => this._onNameVanished()
        );
    }

    destroy() {
        Gio.bus_unwatch_name(this._watchId);
        this._reset();
        this._handlers = null;
    }

    async call(invoke) {
        const proxy = this.proxy;
        if (!proxy) return;
        try {
            await invoke(proxy);
        } catch (error) {
            if (this.proxy === proxy) this._handlers?.onError(remoteMessage(error));
        }
    }

    enqueue(invoke) {
        return this._queue.push(() => this.call(invoke));
    }

    _onNameAppeared() {
        if (this._appearance) return;
        const appearance = new Gio.Cancellable();
        this._appearance = appearance;
        DaemonProxy(
            Gio.DBus.session,
            BUS_NAME,
            OBJECT_PATH,
            (proxy, error) => this._onProxyReady(appearance, proxy, error),
            appearance,
            PROXY_FLAGS
        );
    }

    _onProxyReady(appearance, proxy, error) {
        if (appearance !== this._appearance || appearance.is_cancelled()) return;
        if (error) {
            this._appearance = null;
            this._handlers.onError(remoteMessage(error));
            return;
        }
        this.proxy = proxy;
        this._signalIds = Object.entries(this._signals).map(([name, handler]) =>
            proxy.connectSignal(name, (_proxy, _sender, args) => handler(...args))
        );
        this._handlers.onReady(proxy);
    }

    _onNameVanished() {
        this._reset();
        this._handlers?.onVanished();
    }

    _reset() {
        this._appearance?.cancel();
        this._appearance = null;
        for (const id of this._signalIds) this.proxy?.disconnectSignal(id);
        this._signalIds = [];
        this.proxy = null;
    }
}
