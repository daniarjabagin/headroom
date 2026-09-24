import QtQuick
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/Tokens.js" as Tokens

T.AbstractButton {
    id: button

    required property string iconName
    property bool animated: true
    readonly property bool active: pointer.shown || button.down || button.visualFocus

    implicitWidth: Metrics.controlHeight(Kirigami.Units)
    implicitHeight: implicitWidth
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    display: T.AbstractButton.IconOnly
    scale: button.down && button.animated ? 0.92 : 1
    Accessible.role: Accessible.Button
    Accessible.name: button.text
    Accessible.onPressAction: button.clicked()

    contentItem: Item {
        Kirigami.Icon {
            anchors.centerIn: parent
            implicitWidth: Kirigami.Units.iconSizes.small
            implicitHeight: implicitWidth
            source: button.iconName
            isMask: true
            color: button.active ? Kirigami.Theme.textColor : Tokens.secondaryText(Kirigami.Theme)
        }
    }

    background: Rectangle {
        radius: Metrics.controlRadius(Kirigami.Units)
        color: button.down ? Tokens.pressed(Kirigami.Theme) : Tokens.hover(Kirigami.Theme)
        border.width: button.visualFocus ? Metrics.hairline(Kirigami.Units) : 0
        border.color: Kirigami.Theme.highlightColor
        opacity: button.active ? 1 : 0

        Behavior on opacity {
            NumberAnimation {
                duration: Motion.hoverDuration(Kirigami.Units)
                easing.type: Easing.OutCubic
            }
        }
    }

    HoverTip {
        id: pointer

        text: button.text
    }

    Behavior on scale {
        NumberAnimation {
            duration: Kirigami.Units.shortDuration
            easing.type: Easing.OutCubic
        }
    }
}
