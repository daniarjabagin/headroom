pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/Metrics.js" as Metrics
import "logic/State.js" as State

ColumnLayout {
    id: full

    required property var view
    required property var now
    required property bool alwaysShowPacing
    required property string versionText
    readonly property bool ready: view.kind === "ready"
    readonly property bool empty: ready && State.visibleAccounts(view.state).length === 0 && view.state.spend === null
    readonly property real contentHeight: content.implicitHeight + footer.implicitHeight

    signal refreshRequested(string accountId)
    signal hiddenRequested(string accountId, bool hidden)
    signal startServiceRequested
    signal settingsRequested

    Layout.preferredWidth: Metrics.popupWidth(Kirigami.Units)
    Layout.minimumWidth: Layout.preferredWidth
    Layout.maximumWidth: Layout.preferredWidth
    Layout.preferredHeight: Math.min(contentHeight, Screen.desktopAvailableHeight * 0.8)
    Layout.minimumHeight: Math.min(Layout.preferredHeight, Kirigami.Units.gridUnit * 10)
    spacing: 0

    PlasmaComponents3.ScrollView {
        id: scroll

        Layout.fillWidth: true
        Layout.fillHeight: true
        contentWidth: availableWidth

        ColumnLayout {
            id: content

            width: scroll.availableWidth
            spacing: 0

            Loader {
                active: full.ready && !full.empty
                visible: active
                Layout.fillWidth: true
                Layout.bottomMargin: Metrics.cardPadding(Kirigami.Units)

                sourceComponent: Dashboard {
                    snapshot: full.view.state
                    now: full.now
                    alwaysShowPacing: full.alwaysShowPacing
                    onRefreshRequested: accountId => full.refreshRequested(accountId)
                    onCopyRequested: text => clipboard.copyText(text)
                }
            }

            StatusView {
                visible: !full.ready || full.empty
                Layout.bottomMargin: Metrics.cardPadding(Kirigami.Units)
                view: full.view
                onStartServiceRequested: full.startServiceRequested()
                onRefreshRequested: full.refreshRequested("")
            }
        }
    }

    Footer {
        id: footer

        Layout.fillWidth: true
        view: full.view
        now: full.now
        versionText: full.versionText
        onRefreshRequested: full.refreshRequested("")
        onHiddenRequested: (accountId, hidden) => full.hiddenRequested(accountId, hidden)
        onSettingsRequested: full.settingsRequested()
    }

    ClipboardHelper {
        id: clipboard
    }
}
