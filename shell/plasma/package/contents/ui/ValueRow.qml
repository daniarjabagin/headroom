import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics

RowLayout {
    id: valueRow

    required property string title
    required property string value
    property string tooltip: ""

    Layout.fillWidth: true
    Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
    Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
    Layout.topMargin: Kirigami.Units.smallSpacing / 2
    spacing: Kirigami.Units.largeSpacing

    TextLabel {
        Layout.fillWidth: true
        weight: Font.DemiBold
        text: valueRow.title
        elide: Text.ElideRight
    }

    TextLabel {
        text: valueRow.value

        HoverTip {
            text: valueRow.tooltip
        }
    }
}
