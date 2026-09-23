pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/State.js" as State
import "logic/Tokens.js" as Tokens

Item {
    id: full

    required property var view
    required property var providers
    required property var now
    required property bool live
    required property var display
    required property string lang
    required property bool expanded
    required property var systemTheme
    required property bool reducedMotion
    required property string versionText
    readonly property bool ready: view.kind === "ready"
    readonly property bool empty: ready && State.visibleAccounts(view.state).length === 0 && !(display.showSpend && view.state.spend !== null)
    readonly property var popupColors: Tokens.popupPalette(systemTheme, display.theme, display.translucent)
    readonly property real contentHeight: refreshButton.implicitHeight + refreshButton.Layout.bottomMargin + content.implicitHeight + footer.implicitHeight
    property bool themed: false
    property real reveal: 1

    signal refreshRequested(string accountId)
    signal signInRequested(string providerId)
    signal refreshNowRequested(var onFailed)
    signal orderRequested(var ids)
    signal displayPatched(var patch)
    signal startServiceRequested
    signal settingsRequested

    function applyTheme() {
        const colors = popupColors;
        if (colors === null) {
            if (themed) {
                Kirigami.Theme.backgroundColor = "";
                Kirigami.Theme.textColor = "";
                Kirigami.Theme.inherit = true;
                themed = false;
            }
            return;
        }
        Kirigami.Theme.inherit = false;
        Kirigami.Theme.backgroundColor = colors.backgroundColor;
        Kirigami.Theme.textColor = colors.textColor;
        themed = true;
    }

    function playOpen() {
        if (!Motion.enabled(Kirigami.Units, reducedMotion)) {
            reveal = 1;
            return;
        }
        revealAnimation.restart();
    }

    Layout.preferredWidth: Metrics.popupWidth(Kirigami.Units)
    Layout.minimumWidth: Layout.preferredWidth
    Layout.maximumWidth: Layout.preferredWidth
    Layout.preferredHeight: Math.min(contentHeight, Screen.desktopAvailableHeight * 0.8)
    Layout.minimumHeight: Math.min(Layout.preferredHeight, Kirigami.Units.gridUnit * 10)
    onPopupColorsChanged: applyTheme()
    onExpandedChanged: {
        if (expanded)
            playOpen();
    }
    Component.onCompleted: {
        applyTheme();
        playOpen();
    }

    Rectangle {
        visible: full.popupColors?.painted ?? false
        anchors.fill: parent
        anchors.margins: -Kirigami.Units.smallSpacing
        radius: Metrics.cardRadius(Kirigami.Units)
        color: full.popupColors?.backgroundColor ?? "transparent"
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RefreshButton {
            id: refreshButton

            objectName: "refreshButton"
            Layout.alignment: Qt.AlignRight
            Layout.bottomMargin: Kirigami.Units.smallSpacing
            text: I18n.tr(full.lang, "Refresh")
            enabled: full.view.kind !== "unavailable"
            busy: full.ready && State.isRefreshing(full.view.state)
            animated: Motion.enabled(Kirigami.Units, full.reducedMotion)
            onRefreshNowRequested: full.refreshNowRequested(() => refreshButton.fail())
        }

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
                        providers: full.providers
                        now: full.now
                        live: full.live
                        display: full.display
                        lang: full.lang
                        reveal: full.reveal
                        onRefreshRequested: accountId => full.refreshRequested(accountId)
                        onSignInRequested: providerId => full.signInRequested(providerId)
                        onOrderRequested: ids => full.orderRequested(ids)
                        onDisplayPatched: patch => full.displayPatched(patch)
                    }
                }

                Loader {
                    active: full.view.kind === "loading"
                    visible: active
                    Layout.fillWidth: true
                    Layout.bottomMargin: Metrics.cardPadding(Kirigami.Units)

                    sourceComponent: Skeleton {
                        lang: full.lang
                        reducedMotion: full.reducedMotion
                    }
                }

                StatusView {
                    visible: full.view.kind !== "loading" && (!full.ready || full.empty)
                    Layout.bottomMargin: Metrics.cardPadding(Kirigami.Units)
                    view: full.view
                    lang: full.lang
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
            lang: full.lang
            versionText: full.versionText
            onRefreshRequested: refreshButton.trigger()
            onSettingsRequested: full.settingsRequested()
        }
    }

    NumberAnimation {
        id: revealAnimation

        target: full
        property: "reveal"
        from: 0
        to: 1
        duration: Motion.openDuration(Kirigami.Units)
    }
}
