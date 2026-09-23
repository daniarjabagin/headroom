import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import org.kde.kirigami as Kirigami
import "logic/Metrics.js" as Metrics
import "logic/Tokens.js" as Tokens

T.Button {
    id: button

    implicitWidth: implicitContentWidth + leftPadding + rightPadding
    implicitHeight: Metrics.controlHeight(Kirigami.Units)
    leftPadding: Math.round(Kirigami.Units.gridUnit * 0.75)
    rightPadding: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing
    text: "Options"
    scale: button.down ? 0.96 : 1

    contentItem: RowLayout {
        spacing: Math.round(Kirigami.Units.smallSpacing * 1.25)

        TextLabel {
            role: "label"
            text: button.text
        }

        Kirigami.Icon {
            implicitWidth: Metrics.tinyIcon(Kirigami.Units)
            implicitHeight: implicitWidth
            source: "arrow-down"
            isMask: true
            color: Kirigami.Theme.textColor
        }
    }

    background: Rectangle {
        radius: height / 2
        color: button.hovered || button.down ? Tokens.chip(Kirigami.Theme) : Tokens.control(Kirigami.Theme)
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
