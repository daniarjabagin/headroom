import QtQuick
import org.kde.plasma.workspace.dbus as DBus
import "logic/Settings.js" as Settings
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
    property bool trackSettings: false
    property var settings: null
    property var view: ({
            kind: "loading",
            state: null
        })
    readonly property bool signalsLive: signalLoader.status === Loader.Ready
    property var registration: null

    signal openRequested
    signal commandFailed(string message)

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

    function command(member, signature, args, onReply) {
        if (!watcher.registered)
            return;
        daemonCall(member, signature, args, value => {
            afterCommand();
            if (onReply)
                onReply(value);
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

    function loadSettings() {
        daemonCall("GetSettings", "", [], json => acceptSettings(json));
    }

    function acceptSettings(json) {
        try {
            settings = Settings.parseSettings(json);
        } catch (error) {
            if (!Settings.isSettingsError(error))
                throw error;
            commandFailed(error.message);
        }
    }

    function accept(json) {
        try {
            view = {
                kind: "ready",
                state: State.parseState(json)
            };
            if (trackSettings)
                loadSettings();
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
        if (view.kind === "ready") {
            commandFailed(message);
            return;
        }
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
        command("Refresh", "(s)", [accountId]);
    }

    function rescan() {
        command("Rescan", "", []);
    }

    function setHidden(accountId, hidden) {
        command("SetAccountHidden", "(sb)", [accountId, hidden]);
    }

    function setLabel(accountId, label) {
        command("SetAccountLabel", "(ss)", [accountId, label]);
    }

    function setOrder(ids) {
        if (view.kind === "ready")
            view = {
                kind: "ready",
                state: State.withOrder(view.state, ids)
            };
        command("SetAccountOrder", "(as)", [ids]);
    }

    function updateSettings(change) {
        if (!watcher.registered)
            return;
        daemonCall("GetSettings", "", [], json => {
            try {
                const raw = Settings.decode(json);
                command("SetSettings", "(s)", [JSON.stringify(change(raw))], () => {
                    if (trackSettings)
                        loadSettings();
                });
            } catch (error) {
                if (!Settings.isSettingsError(error))
                    throw error;
                commandFailed(error.message);
            }
        });
    }

    function patchDisplay(patch) {
        if (view.kind === "ready")
            view = {
                kind: "ready",
                state: State.withDisplay(view.state, patch)
            };
        updateSettings(raw => Settings.patchDisplay(raw, patch));
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
            settings = null;
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
