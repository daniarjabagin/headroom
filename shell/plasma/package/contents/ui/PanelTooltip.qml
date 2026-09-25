pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents3
import "logic/Tokens.js" as Tokens

Item {
    id: tooltip

    property var rows: ({
            pinned: [],
            rest: []
        })

    implicitWidth: column.implicitWidth + Kirigami.Units.largeSpacing * 2
    implicitHeight: column.implicitHeight + Kirigami.Units.largeSpacing * 2

    ColumnLayout {
        id: column

        anchors.fill: parent
        anchors.margins: Kirigami.Units.largeSpacing
        spacing: Kirigami.Units.mediumSpacing

        PlasmaComponents3.Label {
            textFormat: Text.PlainText
            text: "Headroom"
            font.pointSize: Kirigami.Theme.defaultFont.pointSize + 0.5
            font.weight: Font.DemiBold
        }

        Repeater {
            model: tooltip.rows.pinned.length

            PanelTipRow {
                required property int index

                Layout.fillWidth: true
                row: tooltip.rows.pinned[index]
            }
        }

        Rectangle {
            visible: tooltip.rows.pinned.length > 0 && tooltip.rows.rest.length > 0
            Layout.fillWidth: true
            implicitHeight: 1
            color: Tokens.separator(Kirigami.Theme)
        }

        Repeater {
            model: tooltip.rows.rest.length

            PanelTipRow {
                required property int index

                Layout.fillWidth: true
                row: tooltip.rows.rest[index]
            }
        }
    }
}
