import Gio from 'gi://Gio';

const BUS_NAME = 'io.github.headroom.Daemon';
const OBJECT_PATH = '/io/github/headroom/Daemon';

const INTERFACE_XML = `
<node>
  <interface name="io.github.headroom.Daemon1">
    <method name="GetState">
      <arg type="s" name="state" direction="out"/>
    </method>
    <method name="Refresh">
      <arg type="s" name="account_id" direction="in"/>
    </method>
    <method name="SetAccountLabel">
      <arg type="s" name="account_id" direction="in"/>
      <arg type="s" name="label" direction="in"/>
    </method>
    <method name="SetAccountHidden">
      <arg type="s" name="account_id" direction="in"/>
      <arg type="b" name="hidden" direction="in"/>
    </method>
    <signal name="StateChanged">
      <arg type="s" name="state"/>
    </signal>
    <signal name="OpenRequested"/>
  </interface>
</node>`;

const DaemonProxy = Gio.DBusProxy.makeProxyWrapper(INTERFACE_XML);

const PROXY_FLAGS = Gio.DBusProxyFlags.DO_NOT_AUTO_START | Gio.DBusProxyFlags.DO_NOT_LOAD_PROPERTIES;

export class DaemonClient {
    constructor({ onAvailable, onUnavailable, onState, onError, onOpenRequested }) {
        this._handlers = { onAvailable, onUnavailable, onState, onError, onOpenRequested };
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
            this._handlers.onError(error.message);
            return;
        }
        this._proxy = proxy;
        this._signalIds = [
            proxy.connectSignal('StateChanged', (_proxy, _sender, [json]) => this._handlers.onState(json)),
            proxy.connectSignal('OpenRequested', () => this._handlers.onOpenRequested()),
        ];
        this._handlers.onAvailable();
        this._loadState();
    }

    async _loadState() {
        const proxy = this._proxy;
        try {
            const [json] = await proxy.GetStateAsync();
            if (this._proxy === proxy) this._handlers.onState(json);
        } catch (error) {
            if (this._proxy === proxy) this._handlers.onError(error.message);
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
            if (this._proxy === proxy) this._handlers.onError(error.message);
        }
    }
}
