pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import "logic/Recovery.js" as Recovery

Flow {
    id: actionRow

    property var actions: []
    property bool animated: true
    property string copiedValue: ""

    signal triggered(string kind, string value)

    spacing: Kirigami.Units.smallSpacing

    Repeater {
        model: actionRow.actions.length

        SmallButton {
            required property int index
            readonly property var actionData: actionRow.actions[index]

            text: actionData ? Recovery.buttonLabel(actionData, actionRow.copiedValue) : ""
            busy: actionData?.busy ?? false
            primary: actionData?.primary ?? false
            animated: actionRow.animated
            onClicked: actionRow.triggered(actionData?.kind ?? "", actionData?.value ?? "")
        }
    }
}
