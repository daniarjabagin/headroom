import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/AccountList.js" as AccountList
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/Providers.js" as Providers
import "logic/Tokens.js" as Tokens

T.AbstractButton {
    id: listRow

    required property var account
    required property string lang
    required property bool current
    required property bool starred
    property bool animated: true
    readonly property var status: AccountList.status(lang, account)
    readonly property string subtitle: AccountList.subtitle(lang, account)
    readonly property real selectedFill: 0.16

    Layout.fillWidth: true
    Layout.leftMargin: Kirigami.Units.smallSpacing
    Layout.rightMargin: Kirigami.Units.smallSpacing
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    padding: Kirigami.Units.mediumSpacing
    leftPadding: Kirigami.Units.largeSpacing
    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    implicitHeight: implicitContentHeight + topPadding + bottomPadding
    opacity: account.hidden ? 0.6 : 1
    Accessible.role: Accessible.ListItem
    Accessible.name: Providers.accountName(account)

    contentItem: RowLayout {
        spacing: Kirigami.Units.mediumSpacing

        ProviderIcon {
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: Math.round(Kirigami.Units.smallSpacing / 2)
            provider: listRow.account.provider
            implicitWidth: Kirigami.Units.iconSizes.smallMedium
            implicitHeight: implicitWidth
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            TextLabel {
                Layout.fillWidth: true
                weight: Font.Medium
                text: Providers.accountName(listRow.account)
                elide: Text.ElideRight
            }

            TextLabel {
                visible: listRow.subtitle !== ""
                Layout.fillWidth: true
                role: "caption"
                emphasis: "secondary"
                text: listRow.subtitle
                elide: Text.ElideRight
            }

            TextLabel {
                objectName: "accountStatus"
                visible: listRow.status.text !== ""
                Layout.fillWidth: true
                role: "caption"
                color: Tokens.noticeColor(Kirigami.Theme, listRow.status.kind)
                text: listRow.status.text
                elide: Text.ElideRight
            }
        }

        Kirigami.Icon {
            visible: listRow.account.hidden
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Metrics.tinyIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: "view-hidden"
            isMask: true
            color: Tokens.secondaryText(Kirigami.Theme)
        }

        Kirigami.Icon {
            objectName: "listStar"
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Metrics.tinyIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: "starred-symbolic"
            isMask: true
            color: Tokens.secondaryText(Kirigami.Theme)
            opacity: listRow.starred ? 1 : 0
        }
    }

    background: Rectangle {
        radius: Metrics.controlRadius(Kirigami.Units)
        color: listRow.current ? Tokens.alpha(Kirigami.Theme.highlightColor, listRow.selectedFill) : Tokens.hover(Kirigami.Theme)
        opacity: listRow.current || pointer.shown || listRow.visualFocus ? 1 : 0
        border.width: listRow.visualFocus ? Metrics.hairline(Kirigami.Units) : 0
        border.color: Kirigami.Theme.highlightColor

        Behavior on opacity {
            enabled: listRow.animated

            NumberAnimation {
                duration: Motion.hoverDuration(Kirigami.Units)
                easing.type: Easing.OutCubic
            }
        }
    }

    PointerHover {
        id: pointer
    }
}
