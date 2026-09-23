import QtQuick
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

T.AbstractButton {
    id: toggle

    property string emphasis: "primary"
    property string hint: ""

    implicitWidth: label.implicitWidth + leftPadding + rightPadding
    implicitHeight: label.implicitHeight + topPadding + bottomPadding
    leftPadding: Kirigami.Units.smallSpacing
    rightPadding: Kirigami.Units.smallSpacing
    leftInset: 0
    rightInset: 0
    hoverEnabled: true

    contentItem: TextLabel {
        id: label

        emphasis: toggle.hovered ? "primary" : toggle.emphasis
        text: toggle.text

        Behavior on color {
            ColorAnimation {
                duration: Kirigami.Units.shortDuration
            }
        }
    }

    background: Rectangle {
        radius: Metrics.chipRadius(Kirigami.Units)
        color: Tokens.chip(Kirigami.Theme)
        opacity: toggle.hovered || toggle.visualFocus ? 1 : 0

        Behavior on opacity {
            NumberAnimation {
                duration: Kirigami.Units.shortDuration
            }
        }
    }

    HoverTip {
        text: toggle.hint
    }
}
