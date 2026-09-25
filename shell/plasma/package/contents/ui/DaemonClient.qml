import QtQuick
import org.kde.plasma.workspace.dbus as DBus
import "logic/Compat.js" as Compat
import "logic/Diagnostics.js" as Diagnostics
import "logic/I18n.js" as I18n
import "logic/PatchQueue.js" as PatchQueue
import "logic/Registry.js" as Registry
import "logic/Settings.js" as Settings
import "logic/SpendQuery.js" as SpendQuery
import "logic/State.js" as State
import "logic/UpdateCheck.js" as UpdateCheck

Item {
    id: client

    readonly property string busName: "io.github.daniarjabagin.Headroom"
    readonly property string objectPath: "/io/github/daniarjabagin/Headroom"
    readonly property string interfaceName: "io.github.daniarjabagin.Headroom1"
    readonly property int activePollMs: 30000
    readonly property int idlePollMs: 60000
    readonly property int startGraceMs: 5000
    property bool active: false
    property string lang: "en"
    property bool trackSettings: false
    property bool trackProviders: false
    property var providers: null
    property bool providersRequested: false
    property var settings: null
    property var rawSettings: null
    property var view: ({
            kind: "loading",
            state: null
        })
    readonly property bool signalsLive: signalLoader.status === Loader.Ready
    property var registration: null
    property var patchQueue: PatchQueue.idle()
    readonly property bool supports06: view.kind === "ready" && Compat.supports06(view.state)

    signal openRequested
    signal commandFailed(string message)

    function call(message, onReply, onSettled) {
        const reply = DBus.SessionBus.asyncCall(message);
        reply.finished.connect(() => {
            if (reply.isError)
                client.reportError(reply.error.message);
            else if (onReply)
                onReply(reply.value);
            if (onSettled)
                onSettled(reply.isError);
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

    function daemonCall(member, signature, args, onReply, onSettled) {
        call({
            service: busName,
            path: objectPath,
            iface: interfaceName,
            member,
            signature,
            arguments: args
        }, onReply, onSettled);
    }

    function load() {
        daemonCall("GetState", "", [], json => accept(json));
    }

    function loadSettings() {
        daemonCall("GetSettings", "", [], json => acceptSettings(json));
    }

    function loadProviders() {
        providersRequested = true;
        daemonCall("ListProviders", "", [], json => acceptProviders(json));
    }

    function acceptProviders(json) {
        try {
            providers = Registry.parseRegistry(json);
        } catch (error) {
            if (!Registry.isRegistryError(error))
                throw error;
            commandFailed(I18n.errorText(lang, error));
        }
    }

    function acceptSettings(json) {
        if (patchQueue.busy)
            return;
        try {
            rawSettings = Settings.decode(json);
            settings = Settings.fromRaw(rawSettings);
        } catch (error) {
            if (!Settings.isSettingsError(error))
                throw error;
            commandFailed(I18n.errorText(lang, error));
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
            if (trackProviders && !providersRequested)
                loadProviders();
        } catch (error) {
            if (!State.isStateError(error))
                throw error;
            view = {
                kind: "error",
                state: null,
                error: I18n.errorText(lang, error)
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

    function refreshNow(onFailed) {
        if (!watcher.registered) {
            onFailed();
            return;
        }
        daemonCall("RefreshNow", "()", [], () => client.afterCommand(), failed => {
            if (failed)
                onFailed();
        });
    }

    function checkForUpdates(onDone) {
        if (!watcher.registered) {
            onDone(UpdateCheck.SERVICE_UNKNOWN, null);
            return;
        }
        checkAttempt(1, onDone);
    }

    function checkAttempt(attempt, onDone) {
        const reply = DBus.SessionBus.asyncCall({
            service: busName,
            path: objectPath,
            iface: interfaceName,
            member: "CheckForUpdates",
            signature: "",
            arguments: []
        });
        reply.finished.connect(() => {
            const errorName = reply.isError ? reply.error.name : "";
            const value = reply.value;
            reply.destroy();
            if (UpdateCheck.retries(errorName, attempt))
                client.checkAttempt(attempt + 1, onDone);
            else
                onDone(errorName, value);
        });
    }

    function getSpend(query, onDone) {
        request("GetSpend", "(s)", [JSON.stringify(query)], SpendQuery.parseResult, onDone);
    }

    function getDiagnostics(onDone) {
        request("GetDiagnostics", "", [], Diagnostics.parseDiagnostics, onDone);
    }

    function request(member, signature, args, parse, onDone) {
        if (!watcher.registered || !supports06) {
            onDone(I18n.tr(lang, "Needs a newer Headroom service"), null);
            return;
        }
        const reply = DBus.SessionBus.asyncCall({
            service: busName,
            path: objectPath,
            iface: interfaceName,
            member,
            signature,
            arguments: args
        });
        reply.finished.connect(() => {
            const failure = reply.isError ? reply.error.message : null;
            const value = reply.isError ? null : parse(reply.value);
            reply.destroy();
            if (failure === null && value === null)
                onDone(I18n.tr(lang, "Unexpected answer from the Headroom service"), null);
            else
                onDone(failure, value);
        });
    }

    function resetSettings() {
        if (!supports06)
            return;
        command("ResetSettings", "", [], () => {
            if (client.trackSettings)
                client.loadSettings();
        });
    }

    function rescan() {
        command("Rescan", "", []);
    }

    function restoreAccounts(providerId) {
        command("RestoreAccounts", "(s)", [providerId]);
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

    function updateSettings(patch) {
        const sendable = Compat.compatiblePatch(patch, supports06);
        if (!watcher.registered || sendable === null)
            return;
        if (rawSettings !== null) {
            rawSettings = Settings.mergePatch(rawSettings, sendable);
            settings = Settings.fromRaw(rawSettings);
        }
        advancePatches(PatchQueue.enqueue(patchQueue, sendable));
    }

    function advancePatches(step) {
        patchQueue = step.queue;
        if (step.send !== null)
            daemonCall("UpdateSettings", "(s)", [JSON.stringify(step.send)], null, () => client.advancePatches(PatchQueue.settle(client.patchQueue)));
        else if (step.drained && watcher.registered)
            patchesApplied();
    }

    function patchesApplied() {
        afterCommand();
        if (trackSettings)
            loadSettings();
    }

    function patchDisplay(patch) {
        if (Compat.compatiblePatch(Settings.displayPatch(patch), supports06) === null)
            return;
        if (view.kind === "ready")
            view = {
                kind: "ready",
                state: State.withDisplay(view.state, patch)
            };
        updateSettings(Settings.displayPatch(patch));
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
            rawSettings = null;
            providers = null;
            providersRequested = false;
            patchQueue = PatchQueue.idle();
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
