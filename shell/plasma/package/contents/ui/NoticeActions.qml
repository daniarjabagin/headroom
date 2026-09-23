pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

RowLayout {
    id: actionRow

    property var actions: []

    signal triggered(string kind, string value)

    spacing: Kirigami.Units.smallSpacing

    Repeater {
        model: actionRow.actions

        SmallButton {
            required property var modelData

            text: modelData.label
            onClicked: actionRow.triggered(modelData.kind, modelData.value)
        }
    }
}
