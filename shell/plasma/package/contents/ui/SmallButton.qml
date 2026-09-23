import QtQuick
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

T.Button {
    id: button

    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    implicitHeight: implicitContentHeight + topPadding + bottomPadding
    topPadding: Math.round(Kirigami.Units.smallSpacing * 0.75)
    bottomPadding: topPadding
    leftPadding: Kirigami.Units.largeSpacing
    rightPadding: Kirigami.Units.largeSpacing
    scale: button.down ? 0.96 : 1

    contentItem: TextLabel {
        role: "caption"
        weight: Font.DemiBold
        text: button.text
        horizontalAlignment: Text.AlignHCenter
    }

    background: Rectangle {
        radius: Metrics.buttonRadius(Kirigami.Units)
        color: button.hovered ? Tokens.chip(Kirigami.Theme) : Tokens.control(Kirigami.Theme)
        border.width: Metrics.hairline(Kirigami.Units)
        border.color: button.visualFocus ? Kirigami.Theme.highlightColor : Tokens.separator(Kirigami.Theme)

        Behavior on color {
            ColorAnimation {
                duration: Kirigami.Units.shortDuration
            }
        }
    }

    Behavior on scale {
        NumberAnimation {
            duration: Kirigami.Units.shortDuration
            easing.type: Easing.OutCubic
        }
    }
}
