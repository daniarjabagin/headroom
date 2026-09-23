import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/Account.js" as Account
import "logic/Format.js" as Format
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Providers.js" as Providers
import "logic/Tokens.js" as Tokens

Item {
    id: header

    required property var account
    required property bool showName
    required property bool offline
    required property var now
    required property string lang
    required property bool canReorder
    readonly property string slot: Account.statusSlot(account, offline)

    signal dragMoved(real offset)
    signal dragFinished

    function tr(msgid, values) {
        return I18n.tr(lang, msgid, values);
    }

    Layout.fillWidth: true
    implicitHeight: row.implicitHeight

    HoverHandler {
        id: hover

        cursorShape: header.canReorder ? (drag.active ? Qt.ClosedHandCursor : Qt.OpenHandCursor) : Qt.ArrowCursor
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
            provider: header.account.provider
        }

        TextLabel {
            Layout.alignment: Qt.AlignBaseline
            Layout.minimumWidth: Math.min(implicitWidth, Kirigami.Units.gridUnit * 3)
            role: "title"
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
                    ago: Format.agoText(header.lang, header.account.updatedAt, header.now)
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

        Item {
            Layout.fillWidth: true
        }

        Kirigami.Icon {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Kirigami.Units.iconSizes.small
            implicitHeight: implicitWidth
            source: "handle-sort"
            isMask: true
            color: Tokens.tertiaryText(Kirigami.Theme)
            opacity: header.canReorder && (hover.hovered || drag.active) ? 1 : 0

            Behavior on opacity {
                NumberAnimation {
                    duration: Kirigami.Units.shortDuration
                }
            }
        }
    }
}
