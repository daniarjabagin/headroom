import QtQuick
import org.kde.plasma.workspace.dbus as DBus
import "logic/State.js" as State

Item {
    id: client

    readonly property string busName: "io.github.headroom.Daemon"
    readonly property string objectPath: "/io/github/headroom/Daemon"
    readonly property string interfaceName: "io.github.headroom.Daemon1"
    readonly property int activePollMs: 30000
    readonly property int idlePollMs: 60000
    readonly property int startGraceMs: 5000
    property bool active: false
    property var view: ({
            kind: "loading",
            state: null
        })
    readonly property bool signalsLive: signalLoader.status === Loader.Ready
    property var registration: null

    signal openRequested

    function call(message, onReply) {
        const reply = DBus.SessionBus.asyncCall(message);
        reply.finished.connect(() => {
            if (reply.isError)
                client.reportError(reply.error.message);
            else if (onReply)
                onReply(reply.value);
            reply.destroy();
        });
    }

    function daemonCall(member, signature, args, onReply) {
        call({
            service: busName,
            path: objectPath,
            iface: interfaceName,
            member,
            signature,
            arguments: args
        }, onReply);
    }

    function load() {
        daemonCall("GetState", "", [], json => accept(json));
    }

    function accept(json) {
        try {
            view = {
                kind: "ready",
                state: State.parseState(json)
            };
        } catch (error) {
            if (!State.isStateError(error))
                throw error;
            view = {
                kind: "error",
                state: null,
                error: error.message
            };
        }
    }

    function reportError(message) {
        if (view.kind === "unavailable" && view.starting) {
            view = {
                kind: "unavailable",
                state: null,
                startError: message
            };
            return;
        }
        if (view.kind === "ready")
            return;
        view = {
            kind: "error",
            state: null,
            error: message
        };
    }

    function afterCommand() {
        if (!signalsLive)
            load();
    }

    function refresh(accountId) {
        if (watcher.registered)
            daemonCall("Refresh", "(s)", [accountId], () => afterCommand());
    }

    function setHidden(accountId, hidden) {
        if (watcher.registered)
            daemonCall("SetAccountHidden", "(sb)", [accountId, hidden], () => afterCommand());
    }

    function startService() {
        view = {
            kind: "unavailable",
            state: null,
            starting: true
        };
        call({
            service: "org.freedesktop.systemd1",
            path: "/org/freedesktop/systemd1",
            iface: "org.freedesktop.systemd1.Manager",
            member: "StartUnit",
            signature: "(ss)",
            arguments: ["headroom.service", "replace"]
        }, () => startGrace.restart());
    }

    function syncRegistration() {
        if (registration === watcher.registered)
            return;
        registration = watcher.registered;
        startGrace.stop();
        if (watcher.registered) {
            view = {
                kind: "loading",
                state: null
            };
            load();
        } else {
            view = {
                kind: "unavailable",
                state: null
            };
        }
    }

    DBus.DBusServiceWatcher {
        id: watcher

        busType: DBus.BusType.Session
        watchedService: client.busName
        onRegisteredChanged: client.syncRegistration()
        Component.onCompleted: client.syncRegistration()
    }

    Loader {
        id: signalLoader

        active: watcher.registered
        source: "StateSignalWatcher.qml"
    }

    Connections {
        function onStateReceived(json) {
            client.accept(json);
        }

        function onOpenRequested() {
            client.openRequested();
        }

        target: signalLoader.item
        ignoreUnknownSignals: true
    }

    Timer {
        interval: client.active ? client.activePollMs : client.idlePollMs
        repeat: true
        running: watcher.registered && !client.signalsLive
        onTriggered: client.load()
    }

    Timer {
        id: startGrace

        interval: client.startGraceMs
        onTriggered: {
            if (client.view.kind === "unavailable")
                client.view = {
                    kind: "unavailable",
                    state: null
                };
        }
    }
}
