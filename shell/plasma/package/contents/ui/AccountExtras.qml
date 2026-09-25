pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Account.js" as Account
import "logic/DaemonText.js" as DaemonText
import "logic/FormatSpend.js" as FormatSpend
import "logic/Metrics.js" as Metrics

Item {
    id: extras

    required property var account
    required property var display
    required property string lang
    required property bool expanded
    property bool animated: true
    readonly property var spendRows: Account.spendRows(lang, account, display)
    readonly property var balances: account.balances
    readonly property real topGap: Metrics.textRowPadding(Kirigami.Units) - Kirigami.Units.smallSpacing / 2
    readonly property real fullHeight: rows.implicitHeight + topGap + Metrics.textRowPadding(Kirigami.Units)

    Layout.fillWidth: true
    Layout.preferredHeight: expanded ? fullHeight : 0
    visible: expanded || heightAnimation.running
    clip: true
    opacity: expanded ? 1 : 0

    ColumnLayout {
        id: rows

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.topMargin: extras.topGap
        spacing: 0

        Repeater {
            model: extras.spendRows.length

            ValueRow {
                required property int index
                readonly property var modelData: extras.spendRows[index]

                title: modelData.title
                value: FormatSpend.spendLine(extras.lang, modelData.totals)
                totals: modelData.totals
                breakdownTitle: modelData.breakdownTitle
                lang: extras.lang
                animated: extras.animated
            }
        }

        Repeater {
            model: extras.balances.length

            ValueRow {
                required property int index
                readonly property var modelData: extras.balances[index]

                title: DaemonText.label(extras.lang, modelData.label)
                value: FormatSpend.balanceValue(extras.lang, modelData)
                lang: extras.lang
                animated: extras.animated
            }
        }
    }

    Behavior on Layout.preferredHeight {
        enabled: extras.animated

        NumberAnimation {
            id: heightAnimation

            duration: Kirigami.Units.longDuration
            easing.type: Easing.OutCubic
        }
    }

    Behavior on opacity {
        enabled: extras.animated

        NumberAnimation {
            duration: Kirigami.Units.longDuration
        }
    }
}
