import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics

ColumnLayout {
    id: group

    property string title: ""
    property string description: ""
    default property alias rows: card.content

    Layout.fillWidth: true
    spacing: Kirigami.Units.smallSpacing

    TextLabel {
        visible: group.title !== ""
        Layout.leftMargin: Metrics.headerInset(Kirigami.Units)
        role: "caption"
        weight: Font.DemiBold
        emphasis: "secondary"
        text: group.title
    }

    Card {
        id: card
    }

    TextLabel {
        visible: group.description !== ""
        Layout.fillWidth: true
        Layout.leftMargin: Metrics.headerInset(Kirigami.Units)
        Layout.rightMargin: Metrics.headerInset(Kirigami.Units)
        role: "caption"
        emphasis: "secondary"
        wrapMode: Text.Wrap
        text: group.description
    }
}
