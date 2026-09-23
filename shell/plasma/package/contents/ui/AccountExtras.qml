pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Account.js" as Account
import "logic/Format.js" as Format
import "logic/Metrics.js" as Metrics

Item {
    id: extras

    required property var account
    required property var display
    required property string lang
    required property bool expanded
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
            model: Account.spendRows(extras.lang, extras.account, extras.display)

            ValueRow {
                required property var modelData

                title: modelData.title
                value: Format.spendLine(extras.lang, modelData.totals)
                totals: modelData.totals
                breakdownTitle: modelData.breakdownTitle
                lang: extras.lang
            }
        }

        Repeater {
            model: extras.account.balances

            ValueRow {
                required property var modelData

                title: modelData.label
                value: Format.balanceValue(extras.lang, modelData)
                lang: extras.lang
            }
        }
    }

    Behavior on Layout.preferredHeight {
        NumberAnimation {
            id: heightAnimation

            duration: Kirigami.Units.longDuration
            easing.type: Easing.OutCubic
        }
    }

    Behavior on opacity {
        NumberAnimation {
            duration: Kirigami.Units.longDuration
        }
    }
}
