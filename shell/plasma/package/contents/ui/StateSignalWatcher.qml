import QtQuick
import org.kde.plasma.workspace.dbus as DBus

Item {
    id: bridge

    signal stateReceived(string json)
    signal openRequested

    DBus.SignalWatcher {
        function dbusStateChanged(json) {
            bridge.stateReceived(json);
        }

        function dbusOpenRequested() {
            bridge.openRequested();
        }

        busType: DBus.BusType.Session
        service: "io.github.headroom.Daemon"
        path: "/io/github/headroom/Daemon"
        iface: "io.github.headroom.Daemon1"
    }
}
