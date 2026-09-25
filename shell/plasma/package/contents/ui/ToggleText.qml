import QtQuick
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion

T.AbstractButton {
    id: toggle

    property string emphasis: "primary"
    property string hint: ""
    property real step: 0

    implicitWidth: label.implicitWidth + leftPadding + rightPadding
    implicitHeight: label.implicitHeight + topPadding + bottomPadding
    leftPadding: Kirigami.Units.smallSpacing
    rightPadding: Kirigami.Units.smallSpacing
    leftInset: 0
    rightInset: 0
    hoverEnabled: true

    contentItem: TextLabel {
        id: label

        emphasis: pointer.shown ? "primary" : toggle.emphasis
        step: toggle.step
        text: toggle.text

        Behavior on color {
            ColorAnimation {
                duration: Motion.hoverDuration(Kirigami.Units)
                easing.type: Easing.OutCubic
            }
        }
    }

    background: HoverFill {
        radius: Metrics.chipRadius(Kirigami.Units)
        shown: pointer.shown || toggle.visualFocus
    }

    HoverTip {
        id: pointer

        text: toggle.hint
    }
}
