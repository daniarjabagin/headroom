import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Format.js" as Format
import "logic/Metrics.js" as Metrics

RowLayout {
    id: valueRow

    required property string title
    required property string value
    required property string lang
    property var totals: null
    property string breakdownTitle: ""
    readonly property bool hasTip: totals !== null && totals.totalTokens > 0

    Layout.fillWidth: true
    Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
    Layout.rightMargin: Metrics.rowInset(Kirigami.Units) - Kirigami.Units.smallSpacing
    Layout.topMargin: Kirigami.Units.smallSpacing / 2
    spacing: Kirigami.Units.largeSpacing

    TextLabel {
        Layout.fillWidth: true
        weight: Font.DemiBold
        text: valueRow.title
        elide: Text.ElideRight
    }

    Item {
        implicitWidth: valueLabel.implicitWidth + Kirigami.Units.smallSpacing * 2
        implicitHeight: valueLabel.implicitHeight + Kirigami.Units.smallSpacing / 2

        HoverFill {
            anchors.fill: parent
            radius: Metrics.chipRadius(Kirigami.Units)
            shown: valueRow.hasTip && tip.shown
        }

        TextLabel {
            id: valueLabel

            anchors.centerIn: parent
            text: valueRow.value
        }

        ModelTip {
            id: tip

            lang: valueRow.lang
            title: valueRow.breakdownTitle
            totals: valueRow.hasTip ? valueRow.totals : null
            fallback: valueRow.hasTip ? Format.spendTooltip(valueRow.lang, valueRow.totals) : ""
        }
    }
}
