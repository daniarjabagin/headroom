import QtQuick
import QtQuick.Layouts
import org.kde.kcmutils as KCM
import org.kde.kirigami as Kirigami
import "logic/I18n.js" as I18n
import "logic/Settings.js" as Settings

KCM.SimpleKCM {
    id: scaffold

    default property alias content: body.data
    property bool trackProviders: false
    readonly property DaemonClient daemon: DaemonClient {
        trackSettings: true
        trackProviders: scaffold.trackProviders
        onCommandFailed: message => scaffold.message = message
    }
    readonly property var settings: daemon.settings
    readonly property var current: settings ?? Settings.fromRaw({})
    readonly property var snapshot: daemon.view.kind === "ready" ? daemon.view.state : null
    readonly property bool ready: settings !== null && snapshot !== null
    readonly property string lang: I18n.resolve(current.display.language, Qt.locale().name)
    property string message: ""

    function updateSettings(patch) {
        message = "";
        daemon.updateSettings(patch);
    }

    function setDisplay(key, value) {
        updateSettings(Settings.displayPatch({
            [key]: value
        }));
    }

    function tr(msgid, values) {
        return I18n.tr(lang, msgid, values);
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Kirigami.Units.gridUnit

        StatusView {
            visible: scaffold.daemon.view.kind === "unavailable" || scaffold.daemon.view.kind === "error"
            view: scaffold.daemon.view
            lang: scaffold.lang
            onStartServiceRequested: scaffold.daemon.startService()
            onRefreshRequested: scaffold.daemon.load()
        }

        TextLabel {
            visible: !scaffold.ready && (scaffold.daemon.view.kind === "loading" || scaffold.daemon.view.kind === "ready")
            Layout.alignment: Qt.AlignHCenter
            emphasis: "secondary"
            text: I18n.tr(scaffold.lang, "Loading…")
        }

        ColumnLayout {
            id: body

            visible: scaffold.ready
            Layout.fillWidth: true
            spacing: Kirigami.Units.gridUnit
        }

        TextLabel {
            visible: scaffold.message !== ""
            Layout.fillWidth: true
            role: "caption"
            color: Kirigami.Theme.negativeTextColor
            wrapMode: Text.Wrap
            text: scaffold.message
        }
    }
}
