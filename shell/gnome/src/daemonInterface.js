import Gio from 'gi://Gio';

export const BUS_NAME = 'io.github.headroom.Daemon';
export const OBJECT_PATH = '/io/github/headroom/Daemon';

const INTERFACE_XML = `
<node>
  <interface name="io.github.headroom.Daemon1">
    <method name="GetState">
      <arg type="s" name="state" direction="out"/>
    </method>
    <method name="Refresh">
      <arg type="s" name="account_id" direction="in"/>
    </method>
    <method name="Rescan"/>
    <method name="GetSettings">
      <arg type="s" name="settings" direction="out"/>
    </method>
    <method name="SetSettings">
      <arg type="s" name="json" direction="in"/>
    </method>
    <method name="SetAccountLabel">
      <arg type="s" name="account_id" direction="in"/>
      <arg type="s" name="label" direction="in"/>
    </method>
    <method name="SetAccountOrder">
      <arg type="as" name="ids" direction="in"/>
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

export const DaemonProxy = Gio.DBusProxy.makeProxyWrapper(INTERFACE_XML);

export const PROXY_FLAGS = Gio.DBusProxyFlags.DO_NOT_AUTO_START | Gio.DBusProxyFlags.DO_NOT_LOAD_PROPERTIES;

export function remoteMessage(error) {
    return String(error?.message ?? error).replace(/^GDBus\.Error:[\w.]+:\s*/, '');
}
