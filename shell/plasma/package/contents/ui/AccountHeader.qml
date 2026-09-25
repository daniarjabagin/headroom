pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/Account.js" as Account
import "logic/Density.js" as Density
import "logic/FormatTime.js" as FormatTime
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/Providers.js" as Providers
import "logic/QuickLinks.js" as QuickLinks
import "logic/Tokens.js" as Tokens

Item {
    id: header

    required property var account
    required property bool showName
    required property bool offline
    required property var now
    required property string lang
    required property bool canReorder
    property var links: ({})
    property var incident: null
    property bool compact: false
    property bool starred: false
    property bool canStar: false
    property bool canShare: true
    readonly property string slot: Account.statusSlot(account, offline)
    readonly property var headerLinks: QuickLinks.headerEntries(lang, links)
    readonly property bool revealed: hover.hovered

    signal dragMoved(real offset)
    signal dragFinished
    signal refreshRequested
    signal hideRequested
    signal starToggled
    signal linkOpened(string url)
    signal shareRequested
    signal copyRequested

    function openMenu() {
        menuLoader.open();
    }

    function tr(msgid, values) {
        return I18n.tr(lang, msgid, values);
    }

    Layout.fillWidth: true
    implicitHeight: row.implicitHeight

    PointerHover {
        id: hover

        cursorShape: header.canReorder ? (drag.active ? Qt.ClosedHandCursor : Qt.OpenHandCursor) : Qt.ArrowCursor
    }

    TapHandler {
        acceptedButtons: Qt.RightButton
        onTapped: header.openMenu()
    }

    Loader {
        id: menuLoader

        function open() {
            active = true;
            (item as AccountMenu).popup();
        }

        active: false

        sourceComponent: AccountMenu {
            providerName: header.account.providerName
            links: QuickLinks.menuEntries(header.lang, header.links)
            starred: header.starred
            canStar: header.canStar
            canShare: header.canShare
            lang: header.lang
            onRefreshRequested: header.refreshRequested()
            onHideRequested: header.hideRequested()
            onStarToggled: header.starToggled()
            onLinkOpened: url => header.linkOpened(url)
            onShareRequested: header.shareRequested()
            onCopyRequested: header.copyRequested()
        }
    }

    DragHandler {
        id: drag

        enabled: header.canReorder
        target: null
        xAxis.enabled: false
        onTranslationChanged: {
            if (active)
                header.dragMoved(translation.y);
        }
        onActiveChanged: {
            if (!active)
                header.dragFinished();
        }
    }

    RowLayout {
        id: row

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Metrics.headerInset(Kirigami.Units)
        anchors.rightMargin: Kirigami.Units.smallSpacing
        spacing: Kirigami.Units.mediumSpacing

        ProviderIcon {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Density.providerIcon(Kirigami.Units, header.compact)
            implicitHeight: implicitWidth
            provider: header.account.provider
        }

        TextLabel {
            Layout.alignment: Qt.AlignBaseline
            Layout.minimumWidth: Math.min(implicitWidth, Kirigami.Units.gridUnit * 3)
            role: "title"
            step: Density.fontStep(header.compact)
            text: Providers.accountTitle(header.account, header.showName)
            elide: Text.ElideRight
        }

        TextLabel {
            visible: header.account.plan !== null
            Layout.alignment: Qt.AlignBaseline
            role: "caption"
            emphasis: "secondary"
            text: header.account.plan ?? ""
        }

        PlasmaComponents3.BusyIndicator {
            visible: header.slot === "refreshing"
            running: visible
            Layout.preferredWidth: Kirigami.Units.iconSizes.small * 0.75
            Layout.preferredHeight: Kirigami.Units.iconSizes.small * 0.75
            Layout.alignment: Qt.AlignVCenter
        }

        TextLabel {
            visible: header.slot === "outdated"
            Layout.alignment: Qt.AlignBaseline
            role: "caption"
            emphasis: "tertiary"
            text: header.tr("Outdated")

            HoverTip {
                text: header.account.updatedAt ? header.tr("Last updated {ago}", {
                    ago: FormatTime.agoText(header.lang, header.account.updatedAt, header.now)
                }) : ""
            }
        }

        Kirigami.Icon {
            visible: header.slot === "warning"
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Metrics.tinyIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: "dialog-warning"
            isMask: true
            color: Tokens.noticeColor(Kirigami.Theme, "warning")

            HoverTip {
                text: header.account.error?.message ?? header.tr("Refresh failed")
            }
        }

        Kirigami.Icon {
            objectName: "incidentIcon"
            visible: header.incident !== null
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Metrics.tinyIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: header.incident?.kind === "error" ? "dialog-error" : "dialog-warning"
            isMask: true
            color: Tokens.noticeColor(Kirigami.Theme, header.incident?.kind ?? "warning")

            HoverTip {
                text: header.incident?.title ?? ""
            }
        }

        Item {
            Layout.fillWidth: true
        }

        Row {
            id: linkRow

            objectName: "quickLinks"
            Layout.alignment: Qt.AlignVCenter
            spacing: Metrics.hairline(Kirigami.Units) * 2
            opacity: header.revealed ? 1 : 0

            Repeater {
                model: header.headerLinks

                IconButton {
                    required property var modelData

                    implicitWidth: Math.round(Kirigami.Units.gridUnit * 1.2)
                    iconSize: Metrics.compactIcon(Kirigami.Units)
                    round: true
                    iconName: modelData.icon
                    text: modelData.tip
                    onClicked: header.linkOpened(modelData.url)
                }
            }

            Behavior on opacity {
                NumberAnimation {
                    duration: Kirigami.Units.shortDuration
                    easing.type: Easing.OutCubic
                }
            }
        }

        Kirigami.Icon {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Kirigami.Units.iconSizes.small
            implicitHeight: implicitWidth
            source: "handle-sort"
            isMask: true
            color: Tokens.tertiaryText(Kirigami.Theme)
            opacity: header.canReorder && (hover.shown || drag.active) ? 1 : 0

            Behavior on opacity {
                NumberAnimation {
                    duration: Motion.hoverDuration(Kirigami.Units)
                    easing.type: Easing.OutCubic
                }
            }
        }
    }
}
