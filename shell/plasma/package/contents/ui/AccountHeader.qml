import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/Account.js" as Account
import "logic/Format.js" as Format
import "logic/Metrics.js" as Metrics
import "logic/Providers.js" as Providers
import "logic/Tokens.js" as Tokens

RowLayout {
    id: header

    required property var account
    required property bool showName
    required property bool offline
    required property var now
    readonly property string slot: Account.statusSlot(account, offline)

    Layout.fillWidth: true
    Layout.leftMargin: Metrics.headerInset(Kirigami.Units)
    Layout.rightMargin: Kirigami.Units.smallSpacing
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
        text: "Outdated"

        HoverTip {
            text: header.account.updatedAt ? "Last updated " + Format.agoText(header.account.updatedAt, header.now) : ""
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
            text: header.account.error?.message ?? "Refresh failed"
        }
    }

    Item {
        Layout.fillWidth: true
    }
}
