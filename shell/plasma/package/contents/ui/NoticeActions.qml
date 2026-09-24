pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

RowLayout {
    id: actionRow

    property var actions: []
    property bool animated: true

    signal triggered(string kind, string value)

    spacing: Kirigami.Units.smallSpacing

    Repeater {
        model: actionRow.actions

        SmallButton {
            required property var modelData

            text: modelData.label
            busy: modelData.busy ?? false
            animated: actionRow.animated
            onClicked: actionRow.triggered(modelData.kind, modelData.value)
        }
    }
}
