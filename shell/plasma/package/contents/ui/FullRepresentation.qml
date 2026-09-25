pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/Share.js" as Share
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
    required property UpdateActions updater
    required property var settings
    readonly property bool ready: view.kind === "ready"
    readonly property bool animated: expanded && Motion.enabled(Kirigami.Units, reducedMotion)
    readonly property bool empty: ready && State.visibleAccounts(view.state).length === 0 && !(display.showSpend && view.state.spend !== null)
    readonly property var popupColors: Tokens.popupPalette(systemTheme, display.theme, display.translucent)
    readonly property real contentHeight: refreshButton.implicitHeight + refreshButton.Layout.bottomMargin + content.implicitHeight + (updateRow.visible ? updateRow.implicitHeight : 0) + footer.implicitHeight
    property bool themed: false
    property real reveal: 1

    signal refreshRequested(string accountId)
    signal signInRequested(string providerId)
    signal refreshNowRequested(var onFailed)
    signal orderRequested(var ids)
    signal displayPatched(var patch)
    signal startServiceRequested
    signal settingsRequested
    signal onboardingDismissed
    signal hideRequested(var accountIds)

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

    function tr(msgid, values) {
        return I18n.tr(lang, msgid, values);
    }

    function openLink(url) {
        Qt.openUrlExternally(url);
    }

    function sharedCard(card, plan) {
        return Share.model(lang, card, plan, view.state.headline, new Date(), display);
    }

    function shareCard(card, plan) {
        exporter.share(sharedCard(card, plan), card.members);
    }

    function copyCard(card, plan) {
        exporter.copy(Share.copyText(sharedCard(card, plan)));
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
        else
            toast.dismiss();
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
            animated: full.animated
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

                OnboardingBanner {
                    settings: full.settings
                    snapshot: full.ready ? full.view.state : null
                    capable: full.ready && full.view.state.supports06
                    lang: full.lang
                    onReviewRequested: full.settingsRequested()
                    onDismissed: full.onboardingDismissed()
                }

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
                        reducedMotion: !full.animated
                        onRefreshRequested: accountId => full.refreshRequested(accountId)
                        onSignInRequested: providerId => full.signInRequested(providerId)
                        onSettingsRequested: full.settingsRequested()
                        onOrderRequested: ids => full.orderRequested(ids)
                        onDisplayPatched: patch => full.displayPatched(patch)
                        onHideRequested: ids => full.hideRequested(ids)
                        onLinkRequested: url => full.openLink(url)
                        onShareRequested: (card, plan) => full.shareCard(card, plan)
                        onCopyRequested: (card, plan) => full.copyCard(card, plan)
                    }
                }

                Loader {
                    active: full.view.kind === "loading"
                    visible: active
                    Layout.fillWidth: true
                    Layout.bottomMargin: Metrics.cardPadding(Kirigami.Units)

                    sourceComponent: Skeleton {
                        lang: full.lang
                        reducedMotion: !full.animated
                    }
                }

                StatusView {
                    visible: full.view.kind !== "loading" && (!full.ready || full.empty)
                    Layout.bottomMargin: Metrics.cardPadding(Kirigami.Units)
                    view: full.view
                    lang: full.lang
                    animated: full.animated
                    onStartServiceRequested: full.startServiceRequested()
                    onRefreshRequested: full.refreshRequested("")
                }
            }
        }

        UpdateRow {
            id: updateRow

            objectName: "updateRow"
            Layout.fillWidth: true
            updater: full.updater
            lang: full.lang
            animated: full.animated
        }

        Footer {
            id: footer

            Layout.fillWidth: true
            view: full.view
            now: full.now
            lang: full.lang
            versionText: full.versionText
            animated: full.animated
            onRefreshRequested: refreshButton.trigger()
            onSettingsRequested: full.settingsRequested()
        }
    }

    Toast {
        id: toast

        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: footer.height + Math.round(Kirigami.Units.gridUnit * 0.75)
        animated: full.animated
        onActionTriggered: url => full.openLink(url)
    }

    ShareExporter {
        id: exporter

        display: full.display
        dark: Tokens.isDark(Kirigami.Theme)
        onSaved: folder => toast.show(full.tr("Saved to Pictures/Headroom"), "", false, full.tr("Open folder"), Share.folderUrl(folder))
        onFailed: toast.show(full.tr("Could not save the image"), "", true, "", "")
        onCopied: toast.show(full.tr("Copied as text"), "", false, "", "")
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
